#![allow(clippy::unwrap_used, clippy::expect_used)]

use shelf_cleanup::findings;
use shelf_core::{FileKind, ScannedFile, Store};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64
}

fn file(path: &str, kind: FileKind, size: u64, mtime: i64) -> ScannedFile {
    ScannedFile {
        path: PathBuf::from(path),
        kind,
        size,
        mtime,
        created: mtime,
        duration_ms: None,
        inode: None,
        is_screenshot: false,
    }
}

#[test]
fn hardlinked_files_count_once() {
    let dir = tempfile::tempdir().unwrap();
    let downloads = dir.path().join("Downloads");
    std::fs::create_dir_all(&downloads).unwrap();
    let a = downloads.join("archive-a.zip");
    let b = downloads.join("archive-b.zip");
    let handle = std::fs::File::create(&a).unwrap();
    handle.set_len(60 * 1024 * 1024).unwrap();
    drop(handle);
    std::fs::hard_link(&a, &b).unwrap();

    let store = Store::open_in_memory().unwrap();
    let t = now();
    let mk = |p: &std::path::Path| ScannedFile {
        path: p.to_path_buf(),
        kind: FileKind::Archive,
        size: 60 * 1024 * 1024,
        mtime: t,
        created: t,
        duration_ms: None,
        inode: None,
        is_screenshot: false,
    };
    store.upsert_batch(&[mk(&a), mk(&b)]).unwrap();

    let found = findings(&store, dir.path()).unwrap();
    let archives = found.iter().find(|f| f.id == "download-archives").unwrap();
    assert_eq!(archives.count, 1);
    assert_eq!(archives.bytes, 60 * 1024 * 1024);
}

#[test]
fn findings_are_review_sets_with_honest_copy() {
    let store = Store::open_in_memory().unwrap();
    let t = now();
    let mut deb = file("/home/u/Downloads/tool.deb", FileKind::App, 20_000_000, t - 100);
    deb.is_screenshot = false;
    let zip = file("/home/u/Downloads/big.zip", FileKind::Archive, 80_000_000, t - 200);
    let mut old_shot =
        file("/home/u/Pictures/Screenshots/old.png", FileKind::Image, 1_000_000, t - 400 * 86_400);
    old_shot.is_screenshot = true;
    store.upsert_batch(&[deb, zip, old_shot]).unwrap();

    let found = findings(&store, Path::new("/home/u")).unwrap();
    let installers = found.iter().find(|f| f.id == "installers").unwrap();
    assert_eq!(installers.title, "Installers in Downloads");
    assert!(!installers.reason.contains("already"));
    assert_eq!(installers.count, 1);
    assert_eq!(installers.bytes, 20_000_000);

    let screens = found.iter().find(|f| f.id == "old-screenshots").unwrap();
    assert!(!screens.reason.contains("likely"));
    assert_eq!(screens.count, 1);

    // A large installer is counted once, not in both cards.
    let total_files: usize = found.iter().map(|f| f.count).sum();
    let unique: std::collections::HashSet<_> =
        found.iter().flat_map(|f| f.files.iter().map(|x| &x.path)).collect();
    assert_eq!(total_files, unique.len());
}
