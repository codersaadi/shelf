use crate::state::{AppState, ListRequest};
use shelf_core::{FileKind, FileMeta, Page, Query, Sort};
use shelf_query::parse;
use shelf_watcher::{Excludes, scan, scan_path};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, State};

pub type CmdResult<T> = Result<T, String>;

fn err<E: std::fmt::Display>(e: E) -> String {
    tracing::error!("{e}");
    e.to_string()
}

/// One page of a shelf/list/search view.
#[tauri::command]
pub fn list_shelf(state: State<'_, AppState>, req: ListRequest) -> CmdResult<Page> {
    let q = state.build_query(&req);
    state.store.list(&q).map_err(err)
}

/// Search is `list_shelf` with text; kept as its own command so the front door
/// is obvious in the IPC surface.
#[tauri::command]
pub fn search(
    state: State<'_, AppState>,
    text: String,
    view: Option<String>,
    sort: Option<Sort>,
    limit: Option<u32>,
    cursor: Option<String>,
) -> CmdResult<Page> {
    let req = ListRequest {
        view,
        kind: None,
        exts: vec![],
        mtime_from: None,
        mtime_to: None,
        text: Some(text),
        sort,
        limit: limit.unwrap_or(120),
        cursor,
    };
    state.store.list(&state.build_query(&req)).map_err(err)
}

#[derive(serde::Serialize)]
pub struct ParsedQuery {
    chips: Vec<shelf_query::Chip>,
    terms: Vec<String>,
}

/// Live parse for the chips inside the search field. No database access.
#[tauri::command]
pub fn parse_query(text: String) -> ParsedQuery {
    let parsed = parse(&text);
    ParsedQuery { chips: parsed.chips, terms: parsed.filter.text }
}

#[tauri::command]
pub fn get_files(state: State<'_, AppState>, ids: Vec<i64>) -> CmdResult<Vec<FileMeta>> {
    state.store.get_files(&ids).map_err(err)
}

#[tauri::command]
pub fn stats(state: State<'_, AppState>) -> CmdResult<Vec<shelf_core::KindStat>> {
    state.store.kind_stats().map_err(err)
}

#[derive(serde::Serialize)]
pub struct HomeShelfData {
    kind: String,
    files: Vec<FileMeta>,
    count: u64,
    bytes: u64,
}

#[derive(serde::Serialize)]
pub struct HomePayload {
    recent: Vec<FileMeta>,
    shelves: Vec<HomeShelfData>,
}

/// One backend call for Home: recent strip plus up to six items per category.
#[tauri::command]
pub fn home(state: State<'_, AppState>) -> CmdResult<HomePayload> {
    let recent = state.store.recent(12).map_err(err)?;
    let stats = state.store.kind_stats().map_err(err)?;
    let mut shelves = Vec::new();
    let (skill_count, skill_bytes) = state.store.skill_stats().map_err(err)?;
    if skill_count > 0 {
        let files = state.store.skills(6).map_err(err)?;
        shelves.push(HomeShelfData {
            kind: "skill".into(),
            files,
            count: skill_count,
            bytes: skill_bytes,
        });
    }
    for kind in [
        FileKind::Image,
        FileKind::Video,
        FileKind::Music,
        FileKind::Document,
        FileKind::Archive,
        FileKind::App,
    ] {
        let q = Query { kinds: vec![kind], sort: Sort::Modified, limit: 6, ..Query::default() };
        let files = state.store.list(&q).map_err(err)?.items;
        if files.is_empty() {
            continue;
        }
        let stat = stats.iter().find(|s| s.kind == kind);
        shelves.push(HomeShelfData {
            kind: kind.as_str().to_string(),
            files,
            count: stat.map_or(0, |s| s.count),
            bytes: stat.map_or(0, |s| s.bytes),
        });
    }
    Ok(HomePayload { recent, shelves })
}

#[derive(serde::Serialize)]
pub struct Counts {
    kinds: Vec<shelf_core::KindStat>,
    total: u64,
    bytes: u64,
    trash: u64,
    trash_bytes: u64,
    reclaimable: u64,
}

#[tauri::command]
pub fn counts(state: State<'_, AppState>) -> CmdResult<Counts> {
    let kinds = state.store.kind_stats().map_err(err)?;
    let (total, bytes) = state.store.totals().map_err(err)?;
    let (db_trash, db_trash_bytes) = state.store.trash_totals().map_err(err)?;
    let system = shelf_platform::list_trash();
    let (trash, trash_bytes) = if system.is_empty() {
        (db_trash, db_trash_bytes)
    } else {
        (system.len() as u64, system.iter().map(|e| e.size).sum())
    };
    let reclaimable = {
        let mut cache = state.reclaimable.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        match *cache {
            Some((at, value)) if at.elapsed() < std::time::Duration::from_secs(30) => value,
            _ => {
                let value = shelf_cleanup::findings(&state.store, &state.home)
                    .map(|f| f.iter().map(|x| x.bytes).sum())
                    .unwrap_or(0);
                *cache = Some((std::time::Instant::now(), value));
                value
            }
        }
    };
    Ok(Counts { kinds, total, bytes, trash, trash_bytes, reclaimable })
}

#[derive(serde::Serialize)]
pub struct Inspect {
    file: FileMeta,
    tags: Vec<String>,
    on_disk: bool,
    width: Option<u32>,
    height: Option<u32>,
    created: Option<i64>,
    text_preview: Option<String>,
}

#[derive(serde::Serialize)]
pub struct NativeIcon {
    data_url: String,
}

#[tauri::command]
pub async fn native_icon(path: String) -> Option<NativeIcon> {
    use base64::Engine;
    let icon = shelf_platform::app_icon_path(Path::new(&path))?;
    let meta = std::fs::metadata(&icon).ok()?;
    if meta.len() > 512 * 1024 {
        return None;
    }
    let bytes = std::fs::read(&icon).ok()?;
    let ext = icon.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    let mime = if bytes.starts_with(b"\x89PNG") || ext == "png" {
        "image/png"
    } else if bytes.windows(4).any(|w| w == b"<svg") || ext == "svg" {
        "image/svg+xml"
    } else {
        return None;
    };
    let data_url =
        format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes));
    Some(NativeIcon { data_url })
}

#[tauri::command]
pub fn path_is_dir(state: State<'_, AppState>, path: String) -> bool {
    crate::state::path_of(&state.home, &path).is_dir()
}

#[tauri::command]
pub fn set_ui_zoom(window: tauri::WebviewWindow, scale: f64) -> CmdResult<()> {
    window.set_zoom(scale.clamp(0.8, 1.5)).map_err(err)
}

#[tauri::command]
pub fn close_window(window: tauri::WebviewWindow) -> CmdResult<()> {
    window.close().map_err(err)
}

#[derive(serde::Serialize)]
pub struct FolderInfo {
    name: String,
    path: String,
    is_codebase: bool,
    score: i32,
    signals: Vec<String>,
}

#[tauri::command]
pub fn folder_info(state: State<'_, AppState>, path: String) -> CmdResult<FolderInfo> {
    let dir = crate::state::path_of(&state.home, &path);
    if !dir.is_dir() {
        return Err(format!("{} is not a directory", dir.display()));
    }
    let mut score = 0;
    let mut signals = Vec::new();
    let mut signal = |name: &str, weight: i32| {
        if !signals.iter().any(|s| s == name) {
            signals.push(name.to_string());
            score += weight;
        }
    };
    // Repository and build/manifest files are the strongest folder-type signals.
    if dir.join(".git").exists() {
        signal(".git repository", 8);
    }
    for (file, weight, label) in [
        ("Cargo.toml", 5, "Cargo.toml"),
        ("package.json", 5, "package.json"),
        ("pyproject.toml", 5, "pyproject.toml"),
        ("setup.py", 4, "setup.py"),
        ("go.mod", 5, "go.mod"),
        ("pom.xml", 5, "pom.xml"),
        ("build.gradle", 5, "build.gradle"),
        ("build.gradle.kts", 5, "build.gradle.kts"),
        ("composer.json", 3, "composer.json"),
        ("Gemfile", 3, "Gemfile"),
        ("CMakeLists.txt", 3, "CMakeLists.txt"),
        ("Makefile", 2, "Makefile"),
        ("tsconfig.json", 3, "tsconfig.json"),
        ("deno.json", 3, "deno.json"),
        ("mix.exs", 3, "mix.exs"),
        ("pubspec.yaml", 3, "pubspec.yaml"),
        ("Package.swift", 3, "Package.swift"),
        ("README.md", 1, "README"),
    ] {
        if dir.join(file).is_file() {
            signal(label, weight);
        }
    }
    if !dir.join(".git").exists() {
        for ancestor in dir.ancestors().skip(1).take(4) {
            if ancestor.join(".git").exists() {
                signal("inside a Git worktree", 6);
                break;
            }
        }
    }
    for (folder, weight, label) in [
        ("src", 3, "src/"),
        ("app", 2, "app/"),
        ("lib", 2, "lib/"),
        ("node_modules", 1, "node_modules/"),
        ("target", 1, "target/"),
        (".github", 2, ".github/"),
        (".vscode", 1, ".vscode/"),
    ] {
        if dir.join(folder).is_dir() {
            signal(label, weight);
        }
    }
    let is_codebase = dir.join(".git").exists() || score >= 5;
    Ok(FolderInfo {
        name: dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| dir.to_string_lossy().into_owned()),
        path: dir.to_string_lossy().into_owned(),
        is_codebase,
        score,
        signals,
    })
}

const DEFAULT_THEME: &str = include_str!("../theme.default.json");

fn theme_file_path() -> std::path::PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("shelf")
        .join("theme.json")
}

#[tauri::command]
pub fn theme_file() -> CmdResult<String> {
    let path = theme_file_path();
    if !path.exists() {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(err)?;
        }
        std::fs::write(&path, DEFAULT_THEME).map_err(err)?;
    }
    Ok(path.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn read_theme() -> CmdResult<String> {
    let path = theme_file_path();
    if !path.exists() {
        return theme_file().and_then(|_| std::fs::read_to_string(&path).map_err(err));
    }
    std::fs::read_to_string(path).map_err(err)
}

#[tauri::command]
pub fn open_theme_file() -> CmdResult<()> {
    let path = theme_file().map(std::path::PathBuf::from)?;
    shelf_platform::open_path(&path).map_err(err)
}

#[derive(serde::Serialize)]
pub struct KnownLocation {
    name: String,
    path: String,
}

#[tauri::command]
pub fn known_locations(state: State<'_, AppState>) -> Vec<KnownLocation> {
    let mut out: Vec<KnownLocation> = Vec::new();
    let mut push = |name: &str, path: std::path::PathBuf| {
        if !path.is_dir() {
            return;
        }
        let path = path.to_string_lossy().into_owned();
        if out.iter().any(|l| l.path == path) {
            return;
        }
        out.push(KnownLocation { name: name.to_string(), path });
    };
    push("Home", state.home.clone());
    for (name, path) in [
        ("Desktop", state.home.join("Desktop")),
        ("Documents", state.home.join("Documents")),
        ("Downloads", state.home.join("Downloads")),
        ("Pictures", state.home.join("Pictures")),
        ("Music", state.home.join("Music")),
        ("Videos", state.home.join("Videos")),
    ] {
        push(name, path);
    }
    for root in &state.roots {
        let name = root
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| root.to_string_lossy().into_owned());
        push(&name, root.clone());
    }
    out
}

#[derive(serde::Serialize)]
pub struct FolderEntry {
    name: String,
    path: String,
    mtime: i64,
}

#[tauri::command]
pub fn child_folders(
    state: State<'_, AppState>,
    path: Option<String>,
) -> CmdResult<Vec<FolderEntry>> {
    let dir = path
        .map(|p| crate::state::path_of(&state.home, &p))
        .unwrap_or_else(|| state.home.join("Downloads"));
    if !dir.is_dir() {
        return Ok(vec![]);
    }
    let excludes = Excludes::default();
    let mut out = Vec::new();
    for entry in std::fs::read_dir(&dir).map_err(err)?.flatten() {
        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
        if !is_dir || excludes.skips_dir(&entry.file_name().to_string_lossy()) {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = entry.path().to_string_lossy().into_owned();
        let mtime = entry
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        out.push(FolderEntry { name, path, mtime });
    }
    out.sort_by_key(|f| std::cmp::Reverse(f.mtime));
    Ok(out)
}

#[derive(serde::Serialize)]
pub struct DefaultCategory {
    mime: String,
    label: String,
    default_app: Option<String>,
    apps: Vec<DefaultApp>,
}

#[derive(serde::Serialize)]
pub struct DefaultApp {
    id: String,
    name: String,
    default: bool,
}

#[derive(serde::Serialize)]
pub struct SettingsInfo {
    roots: Vec<String>,
    db: String,
    home: String,
}

#[derive(serde::Serialize)]
pub struct TrashItem {
    original: String,
    trash_path: String,
    name: String,
    ext: String,
    kind: String,
    size: u64,
    mtime: i64,
    is_dir: bool,
}

#[tauri::command]
pub fn trash_entries() -> Vec<TrashItem> {
    shelf_platform::list_trash()
        .into_iter()
        .map(|entry| {
            let ext = Path::new(&entry.name)
                .extension()
                .map(|e| e.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            let kind = shelf_media::detect_kind(Path::new(&entry.name)).as_str().to_string();
            TrashItem {
                original: entry.original.to_string_lossy().into_owned(),
                trash_path: entry.trash_path.to_string_lossy().into_owned(),
                name: entry.name,
                ext,
                kind,
                size: entry.size,
                mtime: entry.mtime,
                is_dir: entry.is_dir,
            }
        })
        .collect()
}

#[tauri::command]
pub fn rename_path(state: State<'_, AppState>, path: String, name: String) -> CmdResult<String> {
    let old = crate::state::path_of(&state.home, &path);
    let trimmed = name.trim();
    if trimmed.is_empty() || trimmed == "." || trimmed == ".." || trimmed.contains('/') {
        return Err("Enter a valid file name without slashes.".into());
    }
    let parent = old.parent().ok_or("Cannot rename this path.")?;
    let new = parent.join(trimmed);
    if new == old {
        return Ok(old.to_string_lossy().into_owned());
    }
    if new.exists() {
        return Err("A file or folder with that name already exists.".into());
    }
    std::fs::rename(&old, &new).map_err(err)?;
    state.store.rename_path(&old, &new, shelf_media::detect_kind(&new)).map_err(err)?;
    Ok(new.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn restore_trash(trash_path: String, original: String) -> CmdResult<()> {
    shelf_platform::restore_trash_entry(Path::new(&trash_path), Path::new(&original)).map_err(err)
}

#[tauri::command]
pub fn purge_trash(paths: Vec<String>) -> CmdResult<usize> {
    let paths: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    shelf_platform::purge_trash_paths(&paths).map_err(err)
}

#[tauri::command]
pub fn settings_info(state: State<'_, AppState>) -> SettingsInfo {
    let extra = state.extra_roots.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clone();
    let mut roots = state.roots.clone();
    for root in extra {
        if !roots.contains(&root) {
            roots.push(root);
        }
    }
    let db = std::env::var("SHELF_DB")
        .ok()
        .map(std::path::PathBuf::from)
        .or_else(|| dirs::data_dir().map(|d| d.join("shelf").join("index.db")))
        .unwrap_or_default();
    SettingsInfo {
        roots: roots.iter().map(|p| p.to_string_lossy().into_owned()).collect(),
        db: db.to_string_lossy().into_owned(),
        home: state.home.to_string_lossy().into_owned(),
    }
}

#[tauri::command]
pub fn add_index_root(state: State<'_, AppState>, path: String) -> CmdResult<()> {
    let root = crate::state::path_of(&state.home, &path);
    if !root.is_dir() {
        return Err(format!("{} is not a directory", root.display()));
    }
    crate::state::add_extra_root(&state.store, &root).map_err(err)?;
    let mut extra = state.extra_roots.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if !extra.contains(&root) {
        extra.push(root);
    }
    Ok(())
}

#[tauri::command]
pub fn default_apps() -> Vec<DefaultCategory> {
    let categories = [
        ("image/jpeg", "Images · JPEG"),
        ("image/png", "Images · PNG"),
        ("video/mp4", "Videos · MP4"),
        ("audio/mpeg", "Audio · MP3"),
        ("application/pdf", "PDF documents"),
        ("text/plain", "Plain text"),
        ("application/zip", "Archives · ZIP"),
    ];
    categories
        .into_iter()
        .map(|(mime, label)| {
            let apps = shelf_platform::apps_for_mime(mime);
            let default = apps.iter().find(|a| a.is_default).map(|a| a.id.clone());
            DefaultCategory {
                mime: mime.into(),
                label: label.into(),
                default_app: default,
                apps: apps
                    .into_iter()
                    .map(|a| DefaultApp { id: a.id, name: a.name, default: a.is_default })
                    .collect(),
            }
        })
        .collect()
}

#[tauri::command]
pub fn set_default_mime(mime: String, app: String) -> CmdResult<()> {
    shelf_platform::set_default_mime(&mime, &app).map_err(err)
}

#[tauri::command]
pub fn set_default_app(path: String, app: String) -> CmdResult<()> {
    shelf_platform::set_default_for(Path::new(&path), &app).map_err(err)
}

#[tauri::command]
pub fn open_terminal(path: String) -> CmdResult<()> {
    let target = Path::new(&path);
    let dir = if target.is_dir() { target } else { target.parent().unwrap_or(Path::new("/")) };
    shelf_platform::open_terminal(dir).map_err(err)
}

#[tauri::command]
pub fn transfer_paths(
    state: State<'_, AppState>,
    paths: Vec<String>,
    dest: String,
    mode: String,
) -> CmdResult<Vec<String>> {
    if paths.is_empty() {
        return Ok(vec![]);
    }
    let sources: Vec<std::path::PathBuf> = paths.iter().map(std::path::PathBuf::from).collect();
    let destination = crate::state::path_of(&state.home, &dest);
    let moved = mode == "move";
    let results = if moved {
        shelf_platform::move_paths(&sources, &destination)
    } else {
        shelf_platform::copy_paths(&sources, &destination)
    }
    .map_err(|e| err(anyhow::anyhow!(e)))?;

    if moved {
        let _ = state.store.delete_paths(&sources);
    }
    let mut changed = Vec::new();
    for path in &results {
        if path.is_dir() {
            if let Err(e) = scan(&state.store, path, &state.excludes, |_| {}) {
                tracing::warn!("index copy at {}: {e}", path.display());
            }
        } else if scan_path(&state.store, &state.home, path, &state.excludes).unwrap_or(false) {
            changed.push(path.to_string_lossy().into_owned());
        }
    }
    Ok(changed)
}

#[tauri::command]
pub fn open_data_folder(state: State<'_, AppState>) -> CmdResult<()> {
    let db = std::env::var("SHELF_DB")
        .ok()
        .map(std::path::PathBuf::from)
        .or_else(|| dirs::data_dir().map(|d| d.join("shelf").join("index.db")))
        .unwrap_or_else(|| state.home.join(".local/share/shelf/index.db"));
    let dir = db.parent().unwrap_or(Path::new("/"));
    shelf_platform::open_path(dir).map_err(err)
}

#[tauri::command]
pub async fn text_snippet(path: String) -> Option<shelf_media::TextSnippet> {
    shelf_media::text_snippet(Path::new(&path))
}

#[tauri::command]
pub fn inspect(state: State<'_, AppState>, path: String) -> CmdResult<Inspect> {
    let p = crate::state::path_of(&state.home, &path);
    let file = state
        .store
        .by_path(&p)
        .map_err(err)?
        .ok_or_else(|| format!("{} is not in the index", p.display()))?;
    let tags = state.store.tags_of(&p).map_err(err)?;
    let dimensions = shelf_media::image_dimensions(&p);
    let created = std::fs::metadata(&p)
        .ok()
        .and_then(|m| m.created().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64);
    let text_preview = text_preview(&p);
    Ok(Inspect {
        on_disk: p.exists(),
        file,
        tags,
        width: dimensions.map(|(w, _)| w),
        height: dimensions.map(|(_, h)| h),
        created,
        text_preview,
    })
}

#[derive(serde::Serialize)]
pub struct MovedFile {
    path: String,
    trash_path: String,
}

#[derive(serde::Serialize)]
pub struct TrashOutcome {
    receipt_id: i64,
    moved: Vec<MovedFile>,
    bytes: u64,
}

fn text_preview(path: &Path) -> Option<String> {
    if path.file_name().is_some_and(|n| n.to_string_lossy().starts_with('.')) {
        return None;
    }
    let ext = path.extension()?.to_string_lossy().to_lowercase();
    const TEXT: &[&str] = &[
        "txt", "md", "markdown", "rs", "ts", "tsx", "js", "jsx", "json", "toml", "yaml", "yml",
        "py", "sh", "bash", "zsh", "css", "html", "htm", "xml", "csv", "log", "ini", "conf",
    ];
    if !TEXT.contains(&ext.as_str()) {
        return None;
    }
    let meta = std::fs::metadata(path).ok()?;
    if meta.len() > 512 * 1024 {
        return None;
    }
    let bytes = std::fs::read(path).ok()?;
    let text = String::from_utf8_lossy(&bytes);
    let preview: String = text.lines().take(40).collect::<Vec<_>>().join("\n");
    Some(preview.chars().take(8000).collect())
}

/// Move files to the Trash (platform layer), record a receipt, update the index.
/// Refuses paths outside the indexed roots: Shelf never touches system files.
#[tauri::command]
pub fn trash(state: State<'_, AppState>, paths: Vec<String>) -> CmdResult<TrashOutcome> {
    let originals: Vec<PathBuf> =
        paths.iter().map(|p| crate::state::path_of(&state.home, p)).collect();
    if originals.is_empty() {
        return Ok(TrashOutcome { receipt_id: 0, moved: vec![], bytes: 0 });
    }
    let outside: Vec<_> =
        originals.iter().filter(|p| !state.roots.iter().any(|root| p.starts_with(root))).collect();
    if !outside.is_empty() {
        return Err(format!("refusing to trash outside indexed folders: {}", outside[0].display()));
    }
    let mut bytes = 0u64;
    let mut movable = Vec::new();
    for p in &originals {
        if !p.exists() {
            continue;
        }
        bytes += std::fs::metadata(p).map(|m| m.len()).unwrap_or(0);
        movable.push(p.clone());
    }
    if movable.is_empty() {
        return Err("those files are no longer on disk".into());
    }
    let trashed = shelf_platform::to_trash_many(&movable).map_err(err)?;
    let pairs: Vec<(PathBuf, PathBuf)> =
        trashed.iter().map(|t| (t.original.clone(), t.trash_path.clone())).collect();
    let receipt_id = state.store.mark_trashed(&pairs, Some("shelf")).map_err(err)?;
    let moved = trashed
        .iter()
        .map(|t| MovedFile {
            path: t.original.to_string_lossy().into_owned(),
            trash_path: t.trash_path.to_string_lossy().into_owned(),
        })
        .collect();
    Ok(TrashOutcome { receipt_id, moved, bytes })
}

/// Undo a trash receipt: restore from Trash and update the index.
#[tauri::command]
pub fn undo_trash(state: State<'_, AppState>, receipt_id: i64) -> CmdResult<usize> {
    let entries: Vec<(PathBuf, PathBuf)> = state
        .store
        .receipt_paths(receipt_id)
        .map_err(err)?
        .into_iter()
        .map(|(o, t)| (PathBuf::from(o), PathBuf::from(t)))
        .collect();
    let trash_paths: Vec<PathBuf> = entries.iter().map(|(_, t)| t.clone()).collect();
    let restored = shelf_platform::restore_receipt(&entries).map_err(err)?;
    state.store.mark_restored(receipt_id, &trash_paths).map_err(err)?;
    Ok(restored)
}

#[tauri::command]
pub fn duplicates(state: State<'_, AppState>) -> CmdResult<Vec<shelf_cleanup::DuplicateGroup>> {
    shelf_cleanup::find_duplicates(&state.store).map_err(err)
}

#[tauri::command]
pub fn cleanup_findings(state: State<'_, AppState>) -> CmdResult<Vec<shelf_cleanup::Finding>> {
    shelf_cleanup::findings(&state.store, &state.home).map_err(err)
}

#[tauri::command]
pub fn show_in_files(paths: Vec<String>) -> CmdResult<()> {
    let ps: Vec<PathBuf> = paths.iter().map(PathBuf::from).collect();
    shelf_platform::show_in_file_manager(&ps).map_err(err)
}

/// Open a file with its default app; opening from Shelf feeds frecency.
#[tauri::command]
pub fn open_file(state: State<'_, AppState>, path: String, record: Option<bool>) -> CmdResult<()> {
    let p = crate::state::path_of(&state.home, &path);
    shelf_platform::open_path(&p).map_err(err)?;
    if record.unwrap_or(true) {
        state.store.record_open(&p).map_err(err)?;
    }
    Ok(())
}

#[tauri::command]
pub fn clear_activity(state: State<'_, AppState>) -> CmdResult<usize> {
    state.store.clear_activity().map_err(err)
}

#[derive(serde::Serialize)]
pub struct OpenWithApp {
    id: String,
    name: String,
    default: bool,
}

#[tauri::command]
pub fn open_with_apps(state: State<'_, AppState>, path: String) -> CmdResult<Vec<OpenWithApp>> {
    let p = crate::state::path_of(&state.home, &path);
    Ok(shelf_platform::apps_for_path(&p)
        .into_iter()
        .map(|app| OpenWithApp { id: app.id, name: app.name, default: app.is_default })
        .collect())
}

#[tauri::command]
pub fn open_with(state: State<'_, AppState>, path: String, app: String) -> CmdResult<()> {
    let p = crate::state::path_of(&state.home, &path);
    shelf_platform::open_with(&p, &app).map_err(err)?;
    state.store.record_open(&p).map_err(err)?;
    Ok(())
}

#[tauri::command]
pub fn tags_of(state: State<'_, AppState>, path: String) -> CmdResult<Vec<String>> {
    let p = crate::state::path_of(&state.home, &path);
    state.store.tags_of(&p).map_err(err)
}

#[tauri::command]
pub fn set_tags(state: State<'_, AppState>, path: String, tags: Vec<String>) -> CmdResult<()> {
    let p = crate::state::path_of(&state.home, &path);
    if p.exists() {
        shelf_platform::write_tags_xattr(&p, &tags).map_err(err)?;
    }
    state.store.set_tags(&p, &tags).map_err(err)
}

#[tauri::command]
pub fn saved_searches(state: State<'_, AppState>) -> CmdResult<Vec<shelf_core::SavedSearch>> {
    state.store.list_saved_searches().map_err(err)
}

#[tauri::command]
pub fn save_search(state: State<'_, AppState>, name: String, query: String) -> CmdResult<i64> {
    state.store.save_search(&name, &query).map_err(err)
}

#[tauri::command]
pub fn delete_saved_search(state: State<'_, AppState>, id: i64) -> CmdResult<()> {
    state.store.delete_saved_search(id).map_err(err)
}

/// Reindex a root in the background; progress arrives as `shelf://index` events.
#[tauri::command]
pub fn index_root(
    app: AppHandle,
    state: State<'_, AppState>,
    root: Option<String>,
) -> CmdResult<()> {
    let roots = match root {
        Some(r) => vec![crate::state::path_of(&state.home, &r)],
        None => {
            let mut roots = state.roots.clone();
            for root in
                state.extra_roots.lock().unwrap_or_else(std::sync::PoisonError::into_inner).iter()
            {
                if !roots.contains(root) {
                    roots.push(root.clone());
                }
            }
            roots
        }
    };
    let store = state.store.clone();
    let excludes = Excludes::default();
    for root in roots {
        let app = app.clone();
        let store = store.clone();
        let excludes = excludes.clone();
        std::thread::spawn(move || {
            let started = std::time::Instant::now();
            let _ = app.emit("shelf://index", serde_json::json!({ "phase": "start" }));
            let result = scan(&store, &root, &excludes, |done| {
                let _ = app.emit(
                    "shelf://index",
                    serde_json::json!({ "phase": "progress", "done": done }),
                );
            });
            match result {
                Ok(files) => {
                    let _ = store.analyze();
                    let _ = app.emit(
                        "shelf://index",
                        serde_json::json!({
                            "phase": "done",
                            "files": files,
                            "ms": started.elapsed().as_millis() as u64,
                        }),
                    );
                    let _ = app.emit(
                        "shelf://files-changed",
                        serde_json::json!({ "added": files, "removed": 0, "rescan": true }),
                    );
                }
                Err(e) => {
                    let _ = app.emit(
                        "shelf://index",
                        serde_json::json!({ "phase": "error", "message": e.to_string() }),
                    );
                }
            }
        });
    }
    Ok(())
}

/// A busy cursor helper for the UI's path copy (validated on the Rust side so
/// the command exists once, not per widget).
#[tauri::command]
pub fn file_exists(path: String) -> bool {
    Path::new(&path).exists()
}
