use crate::exclude::Excludes;
use shelf_core::{FileKind, ScannedFile, Store};
use shelf_media::{created_of, detect_kind, looks_like_screenshot, probe_duration_ms};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

const BATCH: usize = 512;

/// Full scan of `root` into the store. Returns the number of files indexed.
/// Throttling: batches flush per 512 files; callback fires every 4096 files.
pub fn scan(
    store: &Store,
    root: &Path,
    excludes: &Excludes,
    on_progress: impl Fn(u64),
) -> io::Result<u64> {
    if !root.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("{} is not a directory", root.display()),
        ));
    }
    if root.is_symlink() {
        // The walker does not descend through a symlinked root, so a scan would
        // see zero files, count a clean walk, and prune the root's whole index.
        // Callers are expected to canonicalize roots up front; refuse loudly.
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            format!("{} is a symlink to a directory; pass the target path instead", root.display()),
        ));
    }
    let started = Instant::now();
    let generation = store.begin_scan().map_err(io::Error::other)?;
    let scan_started = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let excludes = Arc::new(excludes.clone());
    let unreadable: Arc<Mutex<Vec<PathBuf>>> = Arc::new(Mutex::new(Vec::new()));
    let mut count: u64 = 0;
    let mut walk_errors: u64 = 0;
    let mut batch: Vec<ScannedFile> = Vec::with_capacity(BATCH);

    for entry in jwalk::WalkDir::new(root).skip_hidden(false).process_read_dir({
        let excludes = excludes.clone();
        let unreadable = unreadable.clone();
        move |depth, dir, _, children| {
            // The first callback invocation carries the root entry only; never filter it.
            if depth.is_none() {
                return;
            }
            if dir.file_name().is_some_and(|n| Excludes::is_bundle_root(&n.to_string_lossy())) {
                children.retain(|r| match r {
                    Ok(d) => !d.file_type.is_dir() && is_launcher_file(&dir.join(&d.file_name)),
                    Err(_) => false,
                });
                return;
            }
            children.retain(|r| match r {
                Ok(d) => {
                    let name = d.file_name.to_string_lossy();
                    let ft = d.file_type;
                    if ft.is_dir() {
                        !excludes.skips_dir(&name)
                    } else {
                        !excludes.skips_entry(&name) && !ft.is_symlink()
                    }
                }
                Err(error) => {
                    if let Some(path) = error.path() {
                        unreadable
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner)
                            .push(path.to_path_buf());
                    }
                    false
                }
            });
        }
    }) {
        let entry = match entry {
            Ok(e) => e,
            Err(err) => {
                walk_errors += 1;
                if let Some(path) = err.path() {
                    unreadable
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .push(path.to_path_buf());
                }
                tracing::debug!("scan skip: {err}");
                continue;
            }
        };
        // jwalk does not yield subdirectory read failures as Err: it stores them
        // on the directory entry. A chmod-000 subdir must count as an error, or
        // the stale-row prune would treat its subtree as user-deleted and wipe
        // the index for files that are still on disk.
        if let Some(err) = entry.read_children.as_ref().and_then(|c| c.error()) {
            walk_errors += 1;
            tracing::debug!("scan read_dir skip {}: {err}", entry.path().display());
            continue;
        }
        let ft = entry.file_type();
        if ft.is_dir() || ft.is_symlink() {
            continue;
        }
        let path = entry.path();
        match fs::metadata(&path) {
            Ok(md) => {
                let kind =
                    if is_bundle_launcher(&path) { FileKind::App } else { detect_kind(&path) };
                let duration_ms = media_duration(store, &path, kind);
                batch.push(ScannedFile {
                    size: md.len(),
                    mtime: mtime_of(&md),
                    created: created_of(&md),
                    duration_ms,
                    inode: inode_of(&md),
                    path: path.clone(),
                    kind,
                    is_screenshot: looks_like_screenshot(&path),
                });
                count += 1;
                if batch.len() == BATCH {
                    store.upsert_batch_at(&batch, Some(generation)).map_err(io::Error::other)?;
                    for f in &batch {
                        ingest_origin(store, &f.path);
                    }
                    batch.clear();
                    std::thread::yield_now();
                }
                if count.is_multiple_of(4096) {
                    on_progress(count);
                    // Background indexing must not starve the UI; yield briefly
                    // after each large batch.
                    std::thread::sleep(std::time::Duration::from_millis(2));
                }
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                // Vanished between readdir and stat; the most common race on an
                // active home dir, and not evidence of a broken tree.
                tracing::debug!("metadata gone {}: {err}", path.display());
            }
            Err(err) => {
                walk_errors += 1;
                tracing::debug!("metadata skip {}: {err}", path.display());
            }
        }
    }
    if !batch.is_empty() {
        store.upsert_batch_at(&batch, Some(generation)).map_err(io::Error::other)?;
        for f in &batch {
            ingest_origin(store, &f.path);
        }
    }
    let mut protected =
        unreadable.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clone();
    if walk_errors > 0 && protected.is_empty() {
        // The root itself was unreadable; no prefix is safe to prune.
        protected.push(root.to_path_buf());
    }
    let removed = store
        .finish_scan_excluding(root, generation, scan_started, &protected)
        .map_err(io::Error::other)?;
    tracing::info!(
        files = count,
        removed,
        walk_errors,
        elapsed_ms = started.elapsed().as_millis() as u64,
        "scan done"
    );
    Ok(count)
}

/// Scan a single path (used by the watcher for created/modified files).
/// `root` is the watched root: excludes apply to ancestors between path and root.
/// Returns false if the path does not exist or is excluded.
pub fn scan_path(store: &Store, root: &Path, path: &Path, excludes: &Excludes) -> io::Result<bool> {
    if !path.exists() {
        return Ok(false);
    }
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    if path.is_dir()
        || path.extension().map(|e| e.to_string_lossy().to_lowercase() == "lnk").unwrap_or(false)
        || excludes.skips_entry(&name)
    {
        return Ok(false);
    }
    // A single path can live inside an excluded directory (e.g. `.cache/x.bin`);
    // the walker handles this during full scans, so check ancestors up to root here.
    let any_hidden_ancestor = path
        .ancestors()
        .take_while(|a| *a != root)
        .any(|a| a.file_name().map(|n| excludes.skips_dir(&n.to_string_lossy())).unwrap_or(false));
    if any_hidden_ancestor {
        return Ok(false);
    }
    if let Some(bundle) = path
        .ancestors()
        .skip(1)
        .take_while(|a| *a != root)
        .find(|a| a.file_name().is_some_and(|n| Excludes::is_bundle_root(&n.to_string_lossy())))
        && (path.parent() != Some(bundle) || !is_launcher_file(path))
    {
        return Ok(false);
    }
    // Symlinks never enter the index (avoid loops and stale targets).
    if path.symlink_metadata().map(|m| m.file_type().is_symlink()).unwrap_or(true) {
        return Ok(false);
    }
    let md = fs::metadata(path)?;
    let kind = if is_bundle_launcher(path) { FileKind::App } else { detect_kind(path) };
    let f = ScannedFile {
        size: md.len(),
        mtime: mtime_of(&md),
        created: created_of(&md),
        duration_ms: media_duration(store, path, kind),
        inode: inode_of(&md),
        path: path.to_path_buf(),
        kind,
        is_screenshot: looks_like_screenshot(path),
    };
    let generation = store.current_scan_gen().map_err(io::Error::other)?;
    store.upsert_file_at(&f, (generation > 0).then_some(generation)).map_err(io::Error::other)?;
    Ok(true)
}

fn is_bundle_launcher(path: &Path) -> bool {
    let Some(parent) =
        path.parent().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().into_owned())
    else {
        return false;
    };
    if !Excludes::is_bundle_root(&parent) || !is_launcher_file(path) {
        return false;
    }
    let file = path.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
    if path.extension().is_some_and(|ext| {
        ext.eq_ignore_ascii_case("desktop") || ext.eq_ignore_ascii_case("appimage")
    }) {
        return true;
    }
    let base = bundle_base_name(&parent);
    !base.is_empty() && file == base
}

/// `Antigravity-x64/antigravity` is a launcher; `node/linux-x64/node` is a
/// platform artifact. The executable must match the bundle's base name.
fn bundle_base_name(parent: &str) -> String {
    let lower = parent.to_ascii_lowercase();
    for suffix in [
        "-linux-x64",
        "-linux64",
        "-x86_64",
        "-aarch64",
        "-arm64",
        "-amd64",
        "-x64",
        "-unpacked",
        ".appdir",
        ".bundle",
        ".pkg",
        ".app",
    ] {
        if lower.ends_with(suffix) {
            return lower[..lower.len() - suffix.len()].to_string();
        }
    }
    lower
}

fn is_launcher_file(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    let name = path.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
    if path.extension().is_some() {
        let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
        // Bundle-root shared objects, sandboxes and crash handlers are not apps.
        return ext == "desktop" || ext == "appimage";
    }
    if name.starts_with("lib")
        || name.contains("sandbox")
        || name.contains("crashpad")
        || name.contains("uninstall")
        || name.contains("license")
        || name.contains("readme")
    {
        return false;
    }
    std::fs::metadata(path)
        .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

fn media_duration(store: &Store, path: &Path, kind: shelf_core::FileKind) -> Option<u64> {
    if !matches!(kind, shelf_core::FileKind::Video | shelf_core::FileKind::Music) {
        return None;
    }
    store.duration_for(path).ok().flatten().or_else(|| probe_duration_ms(path))
}

/// Wire browser origin xattrs into the sources table. When no URL is present,
/// use the writer app or a messaging-app filename pattern.
fn ingest_origin(store: &Store, path: &Path) {
    let origin = shelf_platform::read_origin(path);
    let inferred = infer_source(&path.file_name().map(|n| n.to_string_lossy()).unwrap_or_default());
    let app = origin.app.as_deref().or(inferred.map(|(app, _)| app));
    let url = origin.url.clone().or_else(|| inferred.map(|(_, host)| format!("app://{host}")));
    let Some(url) = url else { return };
    let host = host_of(&url).or_else(|| app.map(|a| a.to_lowercase()));
    if let Err(err) = store.set_source(path, &url, app, host.as_deref()) {
        tracing::debug!("source skip {}: {err}", path.display());
    }
}

/// Native messaging clients often save files without browser xattrs. Their
/// filename conventions are stable enough to attribute the source.
fn infer_source(name: &str) -> Option<(&'static str, &'static str)> {
    let lower = name.to_lowercase();
    if lower.starts_with("whatsapp ") {
        return Some(("WhatsApp", "whatsapp"));
    }
    if (lower.starts_with("img-") || lower.starts_with("vid-") || lower.starts_with("aud-"))
        && (lower.contains("-wa") || lower.contains("_wa"))
    {
        return Some(("WhatsApp", "whatsapp"));
    }
    if lower.starts_with("telegram-cloud-")
        || lower.starts_with("photo_")
        || lower.starts_with("video_")
    {
        return Some(("Telegram", "telegram"));
    }
    if lower.starts_with("signal-") {
        return Some(("Signal", "signal"));
    }
    None
}

fn host_of(url: &str) -> Option<String> {
    let rest = url.split_once("://").map_or(url, |(_, rest)| rest);
    let host = rest.split(['/', '?', '#']).next()?.rsplit('@').next()?;
    let host = host.split(':').next()?.trim_start_matches("www.");
    let host = match host {
        "mail.google.com" => "gmail",
        "docs.google.com" => "google docs",
        "drive.google.com" => "google drive",
        "web.whatsapp.com" => "whatsapp",
        "web.telegram.org" => "telegram",
        "app.slack.com" => "slack",
        "discord.com" => "discord",
        "outlook.live.com" => "outlook",
        "outlook.office.com" => "outlook",
        other => other,
    };
    (!host.is_empty()).then(|| host.to_string())
}

fn mtime_of(md: &fs::Metadata) -> i64 {
    md.modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn inode_of(md: &fs::Metadata) -> Option<u64> {
    use std::os::unix::fs::MetadataExt;
    Some(md.ino())
}

#[cfg(test)]
mod tests {
    use super::{host_of, infer_source};

    #[test]
    fn host_of_extracts_download_hosts() {
        assert_eq!(host_of("https://mail.google.com/a/b").as_deref(), Some("gmail"));
        assert_eq!(host_of("https://www.github.com/x").as_deref(), Some("github.com"));
        assert_eq!(host_of("http://user@example.com:8080/x").as_deref(), Some("example.com"));
        assert_eq!(host_of("not a url"), Some("not a url".into()));
    }

    #[test]
    fn infer_source_recognises_messaging_files() {
        assert_eq!(infer_source("WhatsApp Image 2026-11-02.jpeg"), Some(("WhatsApp", "whatsapp")));
        assert_eq!(infer_source("IMG-20261102-WA0001.jpg"), Some(("WhatsApp", "whatsapp")));
        assert_eq!(infer_source("photo_2026-11-02_12-00-00.jpg"), Some(("Telegram", "telegram")));
        assert_eq!(infer_source("random.jpg"), None);
    }

    #[test]
    fn bundle_base_names_strip_platform_suffixes() {
        assert_eq!(super::bundle_base_name("Antigravity-x64"), "antigravity");
        assert_eq!(super::bundle_base_name("MyApp-linux-x64"), "myapp");
        assert_eq!(super::bundle_base_name("linux-x64"), "linux");
    }
}
