#![allow(clippy::unwrap_used, clippy::expect_used)]

use shelf_core::{FileKind, Store};
use shelf_watcher::{Excludes, scan, scan_path};
use std::fs;
use std::path::Path;

fn fixture() -> (tempfile::TempDir, Vec<String>) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::create_dir_all(root.join("Downloads")).unwrap();
    fs::create_dir_all(root.join("node_modules/pkg")).unwrap();
    fs::create_dir_all(root.join(".hidden")).unwrap();
    fs::create_dir_all(root.join("chatgpt-1.pkg/usr/lib/chatgpt/locales")).unwrap();
    fs::create_dir_all(root.join("Antigravity-x64/locales")).unwrap();
    fs::write(root.join("Downloads/invoice-0387-acme.pdf"), "pdf").unwrap();
    fs::write(root.join("Downloads/design-assets.zip"), "zip").unwrap();
    fs::write(root.join("node_modules/pkg/index.js"), "js").unwrap();
    fs::write(root.join(".hidden/secret.txt"), "txt").unwrap();
    fs::write(root.join("notes.md"), "md").unwrap();
    fs::write(root.join(".env"), "SECRET=1").unwrap();
    fs::write(root.join("chatgpt-1.pkg/usr/lib/chatgpt/locales/en-US.pak"), "pak").unwrap();
    fs::write(root.join("Antigravity-x64/locales/en-US.pak"), "pak").unwrap();
    let launcher = root.join("Antigravity-x64/antigravity");
    fs::write(
        &launcher,
        "#!/bin/sh
",
    )
    .unwrap();
    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(&launcher).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&launcher, perms).unwrap();
    let paths = vec![
        "Downloads/invoice-0387-acme.pdf".into(),
        "Downloads/design-assets.zip".into(),
        "notes.md".into(),
        ".env".into(),
        "Antigravity-x64/antigravity".into(),
    ];
    (dir, paths)
}

#[test]
fn scan_indexes_files_respecting_excludes() {
    let (dir, paths) = fixture();
    let store = Store::open_in_memory().unwrap();
    let n = scan(&store, dir.path(), &Excludes::default(), |_| {}).unwrap();
    assert_eq!(n, 5);
    for p in paths {
        let full = dir.path().join(&p);
        assert!(store.by_path(&full).unwrap().is_some(), "missing {p}");
    }
    let secret = dir.path().join(".hidden/secret.txt");
    let nm = dir.path().join("node_modules/pkg/index.js");
    let bundle_file = dir.path().join("chatgpt-1.pkg/usr/lib/chatgpt/locales/en-US.pak");
    let unpacked_file = dir.path().join("Antigravity-x64/locales/en-US.pak");
    assert!(store.by_path(&secret).unwrap().is_none());
    assert!(store.by_path(&nm).unwrap().is_none());
    assert!(store.by_path(&bundle_file).unwrap().is_none());
    assert!(store.by_path(&unpacked_file).unwrap().is_none());
    let app = store.by_path(&dir.path().join("Antigravity-x64/antigravity")).unwrap().unwrap();
    assert_eq!(app.kind, FileKind::App);
    assert!(store.by_path(&dir.path().join("Antigravity-x64/libEGL.so")).unwrap().is_none());
    assert!(store.by_path(&dir.path().join("Antigravity-x64/chrome-sandbox")).unwrap().is_none());
}

#[test]
fn rescan_prunes_deleted_and_renamed_paths() {
    let (dir, paths) = fixture();
    let store = Store::open_in_memory().unwrap();
    let root = dir.path().to_path_buf();
    scan(&store, &root, &Excludes::default(), |_| {}).unwrap();

    fs::remove_file(root.join(&paths[0])).unwrap();
    fs::rename(root.join(&paths[2]), root.join("notes-renamed.md")).unwrap();
    scan(&store, &root, &Excludes::default(), |_| {}).unwrap();

    assert!(store.by_path(&root.join(&paths[0])).unwrap().is_none());
    assert!(store.by_path(&root.join(&paths[2])).unwrap().is_none());
    assert!(store.by_path(&root.join("notes-renamed.md")).unwrap().is_some());
    assert_eq!(store.totals().unwrap().0, 4);
}

#[test]
fn scan_is_incremental_over_existing_rows() {
    let (dir, _) = fixture();
    let store = Store::open_in_memory().unwrap();
    scan(&store, dir.path(), &Excludes::default(), |_| {}).unwrap();
    scan(&store, dir.path(), &Excludes::default(), |_| {}).unwrap();
    assert_eq!(store.totals().unwrap().0, 5);
}

#[test]
fn scan_detects_kinds_and_screenshots() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("Screenshots")).unwrap();
    fs::write(dir.path().join("Screenshots/Screenshot 2026-10-14 at 14.32.07.png"), "x").unwrap();
    fs::write(dir.path().join("wallpaper.jpg"), "y").unwrap();
    fs::write(dir.path().join("archive.tar.gz"), "z").unwrap();
    let store = Store::open_in_memory().unwrap();
    scan(&store, dir.path(), &Excludes::default(), |_| {}).unwrap();

    let shot = store
        .by_path(&dir.path().join("Screenshots/Screenshot 2026-10-14 at 14.32.07.png"))
        .unwrap()
        .unwrap();
    assert!(shot.is_screenshot);
    assert_eq!(shot.kind, FileKind::Image);

    let img = store.by_path(&dir.path().join("wallpaper.jpg")).unwrap().unwrap();
    assert!(!img.is_screenshot);

    let arc = store.by_path(&dir.path().join("archive.tar.gz")).unwrap().unwrap();
    assert_eq!(arc.kind, FileKind::Archive);
}

#[test]
fn scan_path_single_file_and_deleted() {
    let (dir, paths) = fixture();
    let store = Store::open_in_memory().unwrap();
    let ex = Excludes::default();
    let root = dir.path();
    let file = root.join(&paths[0]);
    assert!(scan_path(&store, root, &file, &ex).unwrap());
    assert!(store.by_path(&file).unwrap().is_some());
    fs::remove_file(&file).unwrap();
    assert!(!scan_path(&store, root, &file, &ex).unwrap());

    let hidden = root.join(".hidden/secret.txt");
    assert!(!scan_path(&store, root, &hidden, &ex).unwrap());
    let _ = Path::new("x");
}

#[test]
fn unreadable_subdir_does_not_wipe_its_index() {
    let (dir, paths) = fixture();
    let store = Store::open_in_memory().unwrap();
    let root = dir.path().to_path_buf();
    scan(&store, &root, &Excludes::default(), |_| {}).unwrap();

    // A private subdirectory is unreadable for us; a scan must leave the rows
    // under it alone instead of treating them as user-deleted.
    let locked = root.join("Downloads");
    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(&locked).unwrap().permissions();
    perms.set_mode(0o000);
    fs::set_permissions(&locked, perms).unwrap();

    let result = scan(&store, &root, &Excludes::default(), |_| {});
    let mut restore = fs::metadata(&locked).unwrap().permissions();
    restore.set_mode(0o755);
    fs::set_permissions(&locked, restore).unwrap();
    result.unwrap();

    assert!(
        store.by_path(&root.join(&paths[0])).unwrap().is_some(),
        "indexed file under an unreadable directory must survive"
    );
    assert_eq!(store.totals().unwrap().0, 5);
}

#[test]
fn symlinked_root_is_refused() {
    let (dir, _) = fixture();
    let target = dir.path().join("Downloads");
    let root = dir.path().join("root-link");
    std::os::unix::fs::symlink(&target, &root).unwrap();
    let store = Store::open_in_memory().unwrap();
    assert!(scan(&store, &root, &Excludes::default(), |_| {}).is_err());
}
