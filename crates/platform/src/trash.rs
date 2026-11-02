use crate::error::Result;
use std::path::{Path, PathBuf};

/// A path that was moved to the Trash, with the location it now lives at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trashed {
    pub original: PathBuf,
    pub trash_path: PathBuf,
}

/// Move one path to the freedesktop Trash. Returns the item's path inside the Trash.
pub fn to_trash(path: &Path) -> Result<Trashed> {
    let moved = to_trash_many(std::slice::from_ref(&path.to_path_buf()))?;
    moved.into_iter().next().ok_or_else(|| crate::Error::NotInTrash(path.display().to_string()))
}

/// Move several paths to the Trash with a single `delete_all` call, then resolve
/// each destination from the Trash index. One listing for the whole batch keeps
/// bulk cleanup cheap.
pub fn to_trash_many(paths: &[PathBuf]) -> Result<Vec<Trashed>> {
    if paths.is_empty() {
        return Ok(vec![]);
    }
    // The Trash spec stores canonical originals; canonicalize before the files
    // move so relative or symlinked CLI paths still match their Trash item.
    let targets: Vec<(PathBuf, PathBuf)> = paths
        .iter()
        .map(|p| (p.clone(), std::fs::canonicalize(p).unwrap_or_else(|_| p.clone())))
        .collect();
    if let Err(err) = trash::delete_all(paths) {
        tracing::warn!("trash crate failed, using home Trash fallback: {err}");
        return match fallback_trash_many(&targets) {
            Ok(items) => Ok(items),
            Err(_) => Err(crate::Error::Trash(err)),
        };
    }
    // A failed listing must not strand files that already moved: fall back to
    // the original path so the receipt still resolves on undo.
    let listed = trash::os_limited::list().unwrap_or_default();
    Ok(targets
        .into_iter()
        .map(|(original, canonical)| {
            let item = listed
                .iter()
                .filter(|i| i.original_path() == canonical)
                .max_by_key(|i| i.time_deleted);
            let trash_path = item.map_or_else(|| original.clone(), |i| trash_file_path(&i.id));
            Trashed { original, trash_path }
        })
        .collect())
}

/// The `trash` crate refuses when the mount's `.Trash-$uid` cannot be created
/// (for example a read-only `/home` topdir). Fall back to the freedesktop home
/// Trash, which is always owned by the user and supports cross-device moves.
fn fallback_trash_many(targets: &[(PathBuf, PathBuf)]) -> Result<Vec<Trashed>> {
    let root = dirs::data_dir().unwrap_or_else(|| PathBuf::from(".")).join("Trash");
    fallback_trash_to(&root, targets)
}

fn fallback_trash_to(root: &Path, targets: &[(PathBuf, PathBuf)]) -> Result<Vec<Trashed>> {
    let files_dir = root.join("files");
    let info_dir = root.join("info");
    std::fs::create_dir_all(&files_dir)?;
    std::fs::create_dir_all(&info_dir)?;

    let mut out = Vec::with_capacity(targets.len());
    for (original, canonical) in targets {
        if !canonical.exists() {
            return Err(crate::Error::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("{} no longer exists", canonical.display()),
            )));
        }
        let name = canonical.file_name().map(|n| n.to_owned()).unwrap_or_default();
        let (trash_path, info_path) = unique_trash_paths(&files_dir, &info_dir, &name);
        let encoded = percent_encoding::utf8_percent_encode(
            &canonical.to_string_lossy(),
            percent_encoding::NON_ALPHANUMERIC,
        )
        .to_string();
        let now = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S");
        let info = format!("[Trash Info]\nPath={encoded}\nDeletionDate={now}\n");
        std::fs::write(&info_path, info)?;
        if move_with_fallback(canonical, &trash_path).is_err() {
            let _ = std::fs::remove_file(&info_path);
            return Err(crate::Error::Io(std::io::Error::other(format!(
                "cannot move {} into the home Trash",
                canonical.display()
            ))));
        }
        out.push(Trashed { original: original.clone(), trash_path });
    }
    Ok(out)
}

fn unique_trash_paths(
    files_dir: &Path,
    info_dir: &Path,
    name: &std::ffi::OsStr,
) -> (PathBuf, PathBuf) {
    let name = Path::new(name);
    let stem = name.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let ext = name.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    for n in 1..10_000 {
        let candidate = if n == 1 { format!("{stem}{ext}") } else { format!("{stem}.{n}{ext}") };
        let file = files_dir.join(&candidate);
        let info = info_dir.join(format!("{candidate}.trashinfo"));
        if !file.exists() && !info.exists() {
            return (file, info);
        }
    }
    let fallback = format!("{stem}.{}{ext}", std::process::id());
    (files_dir.join(&fallback), info_dir.join(format!("{fallback}.trashinfo")))
}

fn move_with_fallback(source: &Path, target: &Path) -> std::io::Result<()> {
    if std::fs::rename(source, target).is_ok() {
        return Ok(());
    }
    if source.is_dir() {
        copy_dir(source, target)?;
        std::fs::remove_dir_all(source)?;
    } else {
        std::fs::copy(source, target)?;
        std::fs::remove_file(source)?;
    }
    Ok(())
}

fn copy_dir(source: &Path, target: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(target)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let from = entry.path();
        let to = target.join(entry.file_name());
        if from.is_dir() {
            copy_dir(&from, &to)?;
        } else if from.is_file() {
            std::fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

/// A Trash entry that belongs to any application, not just Shelf.
#[derive(Debug, Clone)]
pub struct TrashEntry {
    pub original: PathBuf,
    pub trash_path: PathBuf,
    pub name: String,
    pub size: u64,
    pub mtime: i64,
    pub is_dir: bool,
}

/// List every freedesktop Trash entry Shelf can see, newest first.
pub fn list_trash() -> Vec<TrashEntry> {
    use std::collections::HashMap;

    let mut by_path: HashMap<PathBuf, TrashEntry> = HashMap::new();
    for item in trash::os_limited::list().unwrap_or_default() {
        let trash_path = trash_file_path(&item.id);
        let entry = entry_from_paths(item.original_path(), trash_path);
        by_path.insert(entry.trash_path.clone(), entry);
    }
    for root in trash_roots() {
        let info_dir = root.join("info");
        let files_dir = root.join("files");
        let Ok(read) = std::fs::read_dir(&info_dir) else { continue };
        for item in read.flatten() {
            let info_path = item.path();
            if info_path.extension().is_none_or(|e| e != "trashinfo") {
                continue;
            }
            let Some(original) = parse_trashinfo_path(&info_path) else { continue };
            let Some(stem) = info_path.file_stem() else { continue };
            let trash_path = files_dir.join(stem);
            let entry = entry_from_paths(original, trash_path);
            by_path.entry(entry.trash_path.clone()).or_insert(entry);
        }
    }
    let mut out: Vec<TrashEntry> = by_path.into_values().collect();
    out.sort_by_key(|e| std::cmp::Reverse(e.mtime));
    out
}

fn trash_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(home) = dirs::home_dir() {
        roots.push(home.join(".local/share/Trash"));
        if let (Ok(meta), Some(top)) = (std::fs::metadata(&home), home.parent()) {
            use std::os::unix::fs::MetadataExt;
            roots.push(top.join(format!(".Trash-{}", meta.uid())));
        }
    }
    if let Some(data) = dirs::data_dir() {
        roots.push(data.join("Trash"));
    }
    roots.sort();
    roots.dedup();
    roots
}

fn entry_from_paths(original: PathBuf, trash_path: PathBuf) -> TrashEntry {
    let meta = std::fs::metadata(&trash_path).ok();
    let is_dir = meta.as_ref().is_some_and(|m| m.is_dir());
    TrashEntry {
        name: original
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "file".into()),
        original,
        trash_path,
        size: meta.as_ref().map(|m| if m.is_dir() { 0 } else { m.len() }).unwrap_or(0),
        mtime: meta
            .as_ref()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0),
        is_dir,
    }
}

fn parse_trashinfo_path(info: &Path) -> Option<PathBuf> {
    let text = std::fs::read_to_string(info).ok()?;
    for line in text.lines() {
        let Some(rest) = line.trim().strip_prefix("Path=") else { continue };
        let decoded = percent_encoding::percent_decode_str(rest).decode_utf8_lossy();
        let path = PathBuf::from(decoded.as_ref());
        if path.is_absolute() {
            return Some(path);
        }
    }
    None
}

/// Permanently remove entries from a Trash. This is exposed only for paths
/// already inside a known Trash `files` directory.
pub fn purge_trash_paths(paths: &[PathBuf]) -> Result<usize> {
    let mut removed = 0;
    for path in paths {
        if !is_trash_file_path(path) {
            return Err(crate::Error::Io(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                format!("{} is not a Trash file", path.display()),
            )));
        }
        let info = path.parent().and_then(Path::parent).and_then(|root| {
            path.file_name()
                .map(|name| root.join("info").join(format!("{}.trashinfo", name.to_string_lossy())))
        });
        let meta = std::fs::symlink_metadata(path).ok();
        if let Some(meta) = meta {
            if meta.file_type().is_dir() {
                std::fs::remove_dir_all(path)?;
            } else {
                std::fs::remove_file(path)?;
            }
            removed += 1;
        }
        if let Some(info) = info {
            let _ = std::fs::remove_file(info);
        }
    }
    Ok(removed)
}

pub fn is_trash_file_path(path: &Path) -> bool {
    let Some(files) = path.parent() else { return false };
    if files.file_name().is_none_or(|n| n != "files") {
        return false;
    }
    files.parent().is_some_and(|root| trash_roots().iter().any(|candidate| candidate == root))
}

/// Restore one entry by its exact Trash path.
pub fn restore_trash_entry(trash_path: &Path, original: &Path) -> Result<()> {
    if !trash_path.exists() {
        return Err(crate::Error::NotInTrash(trash_path.display().to_string()));
    }
    if original.exists() {
        return Err(crate::Error::Io(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!("{} already exists", original.display()),
        )));
    }
    if let Some(parent) = original.parent() {
        std::fs::create_dir_all(parent)?;
    }
    move_with_fallback(trash_path, original)?;
    if let (Some(files), Some(name)) = (trash_path.parent(), trash_path.file_name())
        && let Some(root) = files.parent()
    {
        let info = root.join("info").join(format!("{}.trashinfo", name.to_string_lossy()));
        let _ = std::fs::remove_file(info);
    }
    Ok(())
}

/// Restore the exact Trash entries recorded in a receipt.
///
/// Matching the original path alone can restore a newer file that happens to sit
/// at the same path. Prefer the recorded `trash_path`; legacy receipts that only
/// stored the original path fall back to that path explicitly.
pub fn restore_receipt(entries: &[(PathBuf, PathBuf)]) -> Result<usize> {
    use trash::os_limited::{list, restore_all};
    let exact: std::collections::HashSet<PathBuf> = entries
        .iter()
        .filter(|(original, trashed)| trashed != original)
        .map(|(_, trashed)| trashed.clone())
        .collect();
    let legacy: std::collections::HashSet<PathBuf> = entries
        .iter()
        .filter(|(original, trashed)| trashed == original)
        .map(|(original, _)| original.clone())
        .collect();

    let items: Vec<_> = list()?
        .into_iter()
        .filter(|item| receipt_matches(&item.id, &item.original_path(), &exact, &legacy))
        .collect();
    let n = items.len();
    restore_all(items)?;
    Ok(n)
}

fn receipt_matches(
    id: &std::ffi::OsStr,
    original: &Path,
    exact: &std::collections::HashSet<PathBuf>,
    legacy: &std::collections::HashSet<PathBuf>,
) -> bool {
    exact.contains(&trash_file_path(id)) || legacy.contains(original)
}

/// A TrashItem id on Linux is `<trash>/info/<name>.trashinfo`; the file itself
/// lives at `<trash>/files/<name>`. Unknown layouts fall back to the id.
fn trash_file_path(id: &std::ffi::OsStr) -> PathBuf {
    let id = Path::new(id);
    let Some(name) = id.file_name().and_then(|n| n.to_str()) else {
        return id.to_path_buf();
    };
    let Some(stem) = name.strip_suffix(".trashinfo") else {
        return id.to_path_buf();
    };
    match id.parent().and_then(Path::parent) {
        Some(trash_root) => trash_root.join("files").join(stem),
        None => id.to_path_buf(),
    }
}

#[cfg(test)]
mod tests {
    use super::{fallback_trash_to, receipt_matches, trash_file_path};
    use std::path::PathBuf;

    #[test]
    fn trash_file_path_derives_files_dir_from_info_id() {
        let id = "/home/u/.local/share/Trash/info/report.pdf.2.trashinfo";
        assert_eq!(
            trash_file_path(id.as_ref()),
            PathBuf::from("/home/u/.local/share/Trash/files/report.pdf.2")
        );
    }

    #[test]
    fn trash_file_path_falls_back_for_unexpected_ids() {
        let id = "/tmp/not-a-trash-entry";
        assert_eq!(trash_file_path(id.as_ref()), PathBuf::from(id));
    }

    #[test]
    fn home_trash_fallback_writes_files_and_trashinfo() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("note.txt");
        std::fs::write(&source, "hello").unwrap();
        let root = dir.path().join("Trash");
        let moved = fallback_trash_to(&root, &[(source.clone(), source.clone())]).unwrap();
        assert_eq!(moved.len(), 1);
        assert!(moved[0].trash_path.is_file());
        let info = root.join("info/note.txt.trashinfo");
        let text = std::fs::read_to_string(info).unwrap();
        assert!(text.contains("[Trash Info]"));
        assert!(text.contains("Path="));
        assert!(text.contains("DeletionDate="));
        assert!(!source.exists());
    }

    #[test]
    fn receipt_matching_prefers_the_exact_trash_entry() {
        use std::collections::HashSet;
        let id: &std::ffi::OsStr =
            std::path::Path::new("/home/u/.local/share/Trash/info/a.pdf.2.trashinfo").as_ref();
        let original = PathBuf::from("/home/u/Downloads/a.pdf");
        let exact = HashSet::from([PathBuf::from("/home/u/.local/share/Trash/files/a.pdf.2")]);
        let legacy = HashSet::new();
        assert!(receipt_matches(id, &original, &exact, &legacy));
        assert!(!receipt_matches(id, &original, &HashSet::new(), &legacy));

        let legacy = HashSet::from([original.clone()]);
        assert!(receipt_matches(id, &original, &HashSet::new(), &legacy));
    }
}
