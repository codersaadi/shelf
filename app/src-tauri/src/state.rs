use shelf_core::{FileKind, Query, Sort, Store};
use shelf_query::parse;
use shelf_watcher::{Event, Excludes, Watcher, scan, scan_path, start_watching};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

/// Everything the commands need. One process owns the index (plan §8); the
/// store is `Send + Sync` so the watcher thread can write while commands read.
pub struct AppState {
    pub store: Arc<Store>,
    pub home: PathBuf,
    pub roots: Vec<PathBuf>,
    pub excludes: Excludes,
    /// Roots added from Settings, persisted in the index.
    pub extra_roots: Mutex<Vec<PathBuf>>,
    /// Dropping the watcher stops the inotify handles; keep it for the app's life.
    _watcher: Mutex<Option<Watcher>>,
    /// Cleanup findings are four queries; the sidebar badge only needs a
    /// coarse, periodically refreshed total.
    pub reclaimable: Mutex<Option<(Instant, u64)>>,
}

impl AppState {
    pub fn new(app: AppHandle) -> anyhow::Result<AppState> {
        let db = db_path();
        if let Some(parent) = db.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let store = Arc::new(Store::open(&db)?);
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
        let mut roots = index_roots(&home);
        let extra_roots = load_extra_roots(&store);
        for root in &extra_roots {
            if !roots.contains(root) {
                roots.push(root.clone());
            }
        }
        let excludes = Excludes::default();
        let watcher = start_watcher(&app, &store, &roots, &excludes);

        let state = AppState {
            store,
            home,
            roots,
            excludes,
            extra_roots: Mutex::new(extra_roots),
            _watcher: Mutex::new(watcher),
            reclaimable: Mutex::new(None),
        };
        state.index_in_background(&app);
        Ok(state)
    }

    /// A shelf or a full-text query, never both: typing in the search field
    /// searches the whole index (the field is the front door).
    pub fn build_query(&self, req: &ListRequest) -> Query {
        let now = chrono::Utc::now().timestamp();
        let sort = req.sort.unwrap_or_else(|| {
            if req.view.as_deref() == Some("recent") { Sort::Added } else { Sort::Modified }
        });
        let text = req.text.as_deref().map(str::trim).filter(|t| !t.is_empty());

        let mut q = if text.is_some() {
            if req.view.as_deref() == Some("trash") {
                Query { in_trash: true, ..Query::default() }
            } else {
                Query::default()
            }
        } else {
            self.view_query(req.view.as_deref())
        };

        if let Some(kind) = req.kind.as_deref().and_then(kind_from_name) {
            q.kinds.push(kind);
        }
        if let Some(text) = text {
            let parsed = parse(text);
            let refined =
                parsed.filter.into_query(&self.home, sort, req.limit, req.cursor.clone(), now);
            q.kinds.extend(refined.kinds);
            q.kinds.dedup();
            q.exts = refined.exts;
            q.screenshot = refined.screenshot.or(q.screenshot);
            q.sources = refined.sources;
            q.tags = refined.tags;
            q.under_path = refined.under_path.or(q.under_path);
            q.mtime_from = refined.mtime_from;
            q.mtime_to = refined.mtime_to;
            q.min_size = refined.min_size;
            q.max_size = refined.max_size;
            q.text = refined.text;
        }
        if !req.exts.is_empty() {
            let requested: Vec<String> = req
                .exts
                .iter()
                .map(|e| e.trim().to_lowercase())
                .filter(|e| !e.is_empty())
                .collect();
            q.exts = if q.exts.is_empty() {
                requested
            } else {
                q.exts.retain(|e| requested.contains(e));
                q.exts
            };
        }
        if req.mtime_from.is_some() {
            q.mtime_from = req.mtime_from;
        }
        if req.mtime_to.is_some() {
            q.mtime_to = req.mtime_to;
        }
        q.sort = sort;
        q.limit = req.limit;
        q.cursor = req.cursor.clone();
        q
    }

    fn view_query(&self, view: Option<&str>) -> Query {
        match view {
            Some("downloads") => Query {
                under_path: Some(self.home.join("Downloads").to_string_lossy().into_owned()),
                direct_children: true,
                ..Query::default()
            },
            Some(place) if place.starts_with("place:") => Query {
                under_path: Some(path_of(&self.home, &place[6..]).to_string_lossy().into_owned()),
                direct_children: true,
                ..Query::default()
            },
            Some("screenshots") | Some("screens") => {
                Query { kinds: vec![FileKind::Image], screenshot: Some(true), ..Query::default() }
            }
            Some("trash") => Query { in_trash: true, ..Query::default() },
            Some("recent") => Query { sort: Sort::Added, exclude_code: true, ..Query::default() },
            Some("home") | None | Some("") => Query::default(),
            Some(k) => {
                let name = k.strip_prefix("kind:").unwrap_or(k);
                match kind_from_name(name) {
                    Some(kind) => Query { kinds: vec![kind], ..Query::default() },
                    None => Query::default(),
                }
            }
        }
    }

    /// Scan on every launch so files added while the app was closed appear.
    /// `SHELF_AUTOINDEX=0` opts out (tests, demos).
    fn index_in_background(&self, app: &AppHandle) {
        if std::env::var("SHELF_AUTOINDEX").is_ok_and(|v| v == "0") {
            return;
        }
        let first_run = self.store.totals().map(|(n, _)| n == 0).unwrap_or(false);
        let store = self.store.clone();
        let roots = self.roots.clone();
        let excludes = self.excludes.clone();
        let app = app.clone();
        std::thread::spawn(move || {
            let _ = app.emit("shelf://index", serde_json::json!({ "phase": "start" }));
            for root in roots {
                let started = std::time::Instant::now();
                let on_progress = |done: u64| {
                    let _ = app.emit(
                        "shelf://index",
                        serde_json::json!({ "phase": "progress", "done": done }),
                    );
                };
                match scan(&store, &root, &excludes, on_progress) {
                    Ok(n) => {
                        let _ = app.emit(
                            "shelf://index",
                            serde_json::json!({
                                "phase": "done",
                                "files": n,
                                "ms": started.elapsed().as_millis() as u64,
                                "first": first_run,
                            }),
                        );
                    }
                    Err(e) => {
                        let _ = app.emit(
                            "shelf://index",
                            serde_json::json!({ "phase": "error", "message": e.to_string() }),
                        );
                    }
                }
            }
            let _ = store.analyze();
            let _ = app.emit(
                "shelf://files-changed",
                serde_json::json!({ "added": 0, "removed": 0, "rescan": true }),
            );
        });
    }
}

/// What the UI sends per request. `text` is the raw NL query; `view` names a
/// sidebar shelf.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ListRequest {
    #[serde(default)]
    pub view: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub exts: Vec<String>,
    #[serde(default)]
    pub mtime_from: Option<i64>,
    #[serde(default)]
    pub mtime_to: Option<i64>,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub sort: Option<Sort>,
    #[serde(default = "default_limit")]
    pub limit: u32,
    #[serde(default)]
    pub cursor: Option<String>,
}

fn default_limit() -> u32 {
    120
}

fn kind_from_name(name: &str) -> Option<FileKind> {
    use FileKind::*;
    Some(match name.to_lowercase().as_str() {
        "images" | "image" => Image,
        "videos" | "video" => Video,
        "music" => Music,
        "documents" | "document" => Document,
        "archives" | "archive" => Archive,
        "apps" | "app" => App,
        "other" => Other,
        _ => return None,
    })
}

fn db_path() -> PathBuf {
    std::env::var_os("SHELF_DB")
        .map(PathBuf::from)
        .or_else(|| dirs::data_dir().map(|d| d.join("shelf").join("index.db")))
        .unwrap_or_else(|| PathBuf::from("shelf.db"))
}

/// `SHELF_INDEX_ROOTS` (colon-separated) exists so demos and tests can point at
/// a scratch tree without indexing a real home directory.
fn index_roots(home: &Path) -> Vec<PathBuf> {
    let from_env = std::env::var_os("SHELF_INDEX_ROOTS")
        .map(|v| std::env::split_paths(&v).filter(|p| p.exists()).collect::<Vec<_>>());
    let mut roots = match from_env {
        Some(roots) if !roots.is_empty() => roots,
        _ => vec![home.to_path_buf()],
    };
    // Applications live outside the usual document roots. Include them even
    // when SHELF_INDEX_ROOTS replaces the defaults.
    for app_dir in [
        home.join("Applications"),
        home.join(".local/bin"),
        home.join(".local/share/applications"),
        PathBuf::from("/usr/local/share/applications"),
        PathBuf::from("/usr/share/applications"),
        PathBuf::from("/var/lib/flatpak/exports/share/applications"),
        PathBuf::from("/var/lib/snapd/desktop/applications"),
    ] {
        if app_dir.is_dir() && !roots.iter().any(|root| app_dir.starts_with(root)) {
            roots.push(app_dir);
        }
    }
    roots
}

fn start_watcher(
    app: &AppHandle,
    store: &Arc<Store>,
    roots: &[PathBuf],
    excludes: &Excludes,
) -> Option<Watcher> {
    let (tx, rx) = std::sync::mpsc::channel();
    let watcher = match start_watching(roots, tx) {
        Ok(w) => w,
        Err(e) => {
            tracing::warn!("file watching disabled: {e}");
            return None;
        }
    };
    let app = app.clone();
    let store = store.clone();
    let excludes = excludes.clone();
    let roots = roots.to_vec();
    std::thread::spawn(move || {
        // The debouncer coalesces bursts; this loop turns each batch into index
        // writes and one UI event, never one event per file.
        while let Ok(first) = rx.recv() {
            let mut changed = Vec::new();
            let mut removed = Vec::new();
            collect(first, &mut changed, &mut removed);
            while let Ok(ev) = rx.recv_timeout(Duration::from_millis(120)) {
                collect(ev, &mut changed, &mut removed);
            }
            let mut added = 0u64;
            for path in &changed {
                let root = roots.iter().find(|r| path.starts_with(r));
                let root =
                    root.cloned().unwrap_or_else(|| roots.first().cloned().unwrap_or_default());
                if scan_path(&store, &root, path, &excludes).unwrap_or(false) {
                    added += 1;
                }
            }
            let mut removed_n = store.delete_paths(&removed).unwrap_or(0) as u64;
            // A directory removal arrives as one event; drop its indexed children too.
            for path in &removed {
                removed_n += store.prune_under_path(path).unwrap_or(0) as u64;
            }
            if added > 0 || removed_n > 0 {
                let _ = app.emit(
                    "shelf://files-changed",
                    serde_json::json!({ "added": added, "removed": removed_n, "rescan": false }),
                );
            }
        }
    });
    Some(watcher)
}

fn load_extra_roots(store: &Store) -> Vec<PathBuf> {
    let Ok(Some(raw)) = store.meta_get("extra_roots") else { return Vec::new() };
    serde_json::from_str::<Vec<PathBuf>>(&raw).unwrap_or_default()
}

/// Persist a user-added indexing root. The watcher picks it up on next launch;
/// `index_root` scans it immediately.
pub fn add_extra_root(store: &Store, path: &Path) -> anyhow::Result<()> {
    let mut roots = load_extra_roots(store);
    if !roots.iter().any(|r| r == path) {
        roots.push(path.to_path_buf());
        store.meta_set("extra_roots", &serde_json::to_string(&roots)?)?;
    }
    Ok(())
}

fn collect(event: Event, changed: &mut Vec<PathBuf>, removed: &mut Vec<PathBuf>) {
    match event {
        Event::Changed(paths) => changed.extend(paths),
        Event::Removed(paths) => removed.extend(paths),
        Event::Rescan => {}
    }
}

/// Resolve a UI-supplied path. Absolute paths pass through; `~` expands.
pub fn path_of(home: &Path, p: &str) -> PathBuf {
    if let Some(rest) = p.strip_prefix("~/") {
        return home.join(rest);
    }
    PathBuf::from(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> AppState {
        AppState {
            store: Arc::new(Store::open_in_memory().expect("memory store")),
            home: PathBuf::from("/home/shelf"),
            roots: vec![PathBuf::from("/home/shelf")],
            excludes: Excludes::default(),
            extra_roots: Mutex::new(Vec::new()),
            _watcher: Mutex::new(None),
            reclaimable: Mutex::new(None),
        }
    }

    #[test]
    fn text_search_ignores_the_active_kind_view() {
        let st = state();
        let q = st.build_query(&ListRequest {
            view: Some("images".into()),
            text: Some("invoice".into()),
            kind: None,
            exts: vec![],
            mtime_from: None,
            mtime_to: None,
            sort: None,
            limit: 20,
            cursor: None,
        });
        assert!(q.kinds.is_empty());
        assert_eq!(q.text, vec!["invoice".to_string()]);
    }

    #[test]
    fn empty_text_keeps_the_view_filter() {
        let st = state();
        let q = st.build_query(&ListRequest {
            view: Some("images".into()),
            text: None,
            kind: None,
            exts: vec![],
            mtime_from: None,
            mtime_to: None,
            sort: None,
            limit: 20,
            cursor: None,
        });
        assert_eq!(q.kinds, vec![FileKind::Image]);
    }

    #[test]
    fn kind_view_prefix_selects_one_kind() {
        let st = state();
        let q = st.build_query(&ListRequest {
            view: Some("kind:image".into()),
            text: None,
            kind: None,
            exts: vec![],
            mtime_from: None,
            mtime_to: None,
            sort: None,
            limit: 20,
            cursor: None,
        });
        assert_eq!(q.kinds, vec![FileKind::Image]);
    }

    #[test]
    fn explicit_ext_and_date_filters_apply_to_a_view() {
        let st = state();
        let q = st.build_query(&ListRequest {
            view: Some("kind:document".into()),
            text: None,
            kind: None,
            exts: vec!["pdf".into(), "md".into()],
            mtime_from: Some(1_700_000_000),
            mtime_to: None,
            sort: None,
            limit: 20,
            cursor: None,
        });
        assert_eq!(q.kinds, vec![FileKind::Document]);
        assert_eq!(q.exts, vec!["pdf".to_string(), "md".to_string()]);
        assert_eq!(q.mtime_from, Some(1_700_000_000));
    }

    #[test]
    fn explicit_filters_intersect_a_parsed_search() {
        let st = state();
        let q = st.build_query(&ListRequest {
            view: None,
            text: Some("pdf lease".into()),
            kind: Some("document".into()),
            exts: vec!["pdf".into(), "docx".into()],
            mtime_from: Some(1_700_000_000),
            mtime_to: None,
            sort: None,
            limit: 20,
            cursor: None,
        });
        assert!(q.kinds.contains(&FileKind::Document));
        assert_eq!(q.exts, vec!["pdf".to_string()]);
        assert_eq!(q.mtime_from, Some(1_700_000_000));
    }

    #[test]
    fn location_view_is_a_direct_child_browse() {
        let st = state();
        let q = st.build_query(&ListRequest {
            view: Some("place:/home/shelf/work".into()),
            text: None,
            kind: None,
            exts: vec![],
            mtime_from: None,
            mtime_to: None,
            sort: None,
            limit: 20,
            cursor: None,
        });
        assert_eq!(q.under_path.as_deref(), Some("/home/shelf/work"));
        assert!(q.direct_children);
    }

    #[test]
    fn downloads_view_lists_direct_children_only() {
        let st = state();
        let q = st.build_query(&ListRequest {
            view: Some("downloads".into()),
            text: None,
            kind: None,
            exts: vec![],
            mtime_from: None,
            mtime_to: None,
            sort: None,
            limit: 20,
            cursor: None,
        });
        assert_eq!(q.under_path.as_deref(), Some("/home/shelf/Downloads"));
        assert!(q.direct_children);
    }

    #[test]
    fn trash_view_is_kept_for_text_searches() {
        let st = state();
        let q = st.build_query(&ListRequest {
            view: Some("trash".into()),
            text: Some("pdf".into()),
            kind: None,
            exts: vec![],
            mtime_from: None,
            mtime_to: None,
            sort: None,
            limit: 20,
            cursor: None,
        });
        assert!(q.in_trash);
        assert_eq!(q.exts, vec!["pdf".to_string()]);
    }

    #[test]
    fn recent_view_defaults_to_date_added() {
        let st = state();
        let q = st.build_query(&ListRequest {
            view: Some("recent".into()),
            text: None,
            kind: None,
            exts: vec![],
            mtime_from: None,
            mtime_to: None,
            sort: None,
            limit: 20,
            cursor: None,
        });
        assert_eq!(q.sort, Sort::Added);
    }

    #[test]
    fn plain_english_query_lands_in_one_filter() {
        let st = state();
        let q = st.build_query(&ListRequest {
            view: Some("home".into()),
            text: Some("pdf in august over 5 MB from gmail".into()),
            kind: None,
            exts: vec![],
            mtime_from: None,
            mtime_to: None,
            sort: None,
            limit: 20,
            cursor: None,
        });
        assert_eq!(q.exts, vec!["pdf".to_string()]);
        assert_eq!(q.min_size, Some(5_000_001));
        assert_eq!(q.sources, vec!["gmail".to_string()]);
        assert!(q.mtime_from.is_some());
        assert!(q.kinds.is_empty());
    }

    #[test]
    fn tilde_paths_expand_against_home() {
        assert_eq!(
            path_of(Path::new("/home/shelf"), "~/Downloads"),
            PathBuf::from("/home/shelf/Downloads")
        );
        assert_eq!(path_of(Path::new("/home/shelf"), "/tmp/x"), PathBuf::from("/tmp/x"));
    }
}
