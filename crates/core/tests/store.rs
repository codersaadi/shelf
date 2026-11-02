#![allow(clippy::unwrap_used, clippy::expect_used)]

use shelf_core::*;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64
}

fn sf(path: &str, kind: FileKind, size: u64, mtime: i64) -> ScannedFile {
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

fn store() -> Store {
    Store::open_in_memory().unwrap()
}

fn seed(s: &Store) {
    let t = now();
    let files = vec![
        sf("/home/u/Downloads/invoice-0387-acme.pdf", FileKind::Document, 6_200_000, t - 60),
        sf(
            "/home/u/Downloads/invoice-0412-northwind.pdf",
            FileKind::Document,
            5_600_000,
            t - 3_600,
        ),
        sf("/home/u/Downloads/design-assets.zip", FileKind::Archive, 212_000_000, t - 86_400),
        sf("/home/u/Pictures/IMG_4821.jpg", FileKind::Image, 3_100_000, t - 7200),
        sf("/home/u/Music/night-drive.flac", FileKind::Music, 38_000_000, t - 100_000),
        sf("/home/u/notes.md", FileKind::Document, 900, t - 200_000),
    ];
    s.upsert_batch(&files).unwrap();
}

#[test]
fn added_at_uses_the_discovered_creation_time() {
    let s = store();
    let created = now() - 10_000;
    let mut f = sf("/home/u/Downloads/old.zip", FileKind::Archive, 10, created + 5_000);
    f.created = created;
    s.upsert_file(&f).unwrap();
    assert_eq!(s.by_path(&f.path).unwrap().unwrap().added_at, created);
}

#[test]
fn recent_hides_source_code_and_uses_mtime_without_creation_time() {
    let s = store();
    let t = now();
    let mut code = sf("/home/u/work/apps/web/src/route.tsx", FileKind::Other, 100, t);
    code.created = 0;
    let doc = sf("/home/u/Documents/lease.pdf", FileKind::Document, 100, t - 60);
    s.upsert_batch(&[code.clone(), doc.clone()]).unwrap();
    let recent = s.recent(10).unwrap();
    assert_eq!(recent.len(), 1);
    assert_eq!(recent[0].path, doc.path.to_string_lossy());
    assert_eq!(s.by_path(&code.path).unwrap().unwrap().added_at, t);
}

#[test]
fn duration_survives_a_rescan_without_probe() {
    let s = store();
    let mut f = sf("/home/u/Videos/clip.mp4", FileKind::Video, 10, now());
    f.duration_ms = Some(42_000);
    s.upsert_file(&f).unwrap();
    assert_eq!(s.by_path(&f.path).unwrap().unwrap().duration_ms, Some(42_000));

    f.duration_ms = None;
    s.upsert_file(&f).unwrap();
    assert_eq!(s.by_path(&f.path).unwrap().unwrap().duration_ms, Some(42_000));
}

#[test]
fn upsert_is_idempotent_and_updates() {
    let s = store();
    seed(&s);
    assert_eq!(s.totals().unwrap().0, 6);

    let same =
        sf("/home/u/Downloads/invoice-0387-acme.pdf", FileKind::Document, 6_200_000, now() - 60);
    assert!(!s.upsert_file(&same).unwrap());

    let grown = sf("/home/u/Downloads/invoice-0387-acme.pdf", FileKind::Document, 9_000_000, now());
    assert!(s.upsert_file(&grown).unwrap());
    let meta = s.by_path(Path::new("/home/u/Downloads/invoice-0387-acme.pdf")).unwrap().unwrap();
    assert_eq!(meta.size, 9_000_000);
}

#[test]
fn delete_paths_removes_rows() {
    let s = store();
    seed(&s);
    s.delete_paths(&[PathBuf::from("/home/u/notes.md")]).unwrap();
    assert_eq!(s.totals().unwrap().0, 5);
}

#[test]
fn fts_matches_substrings_and_terms() {
    let s = store();
    seed(&s);
    let q =
        Query { text: vec!["invoice".into()], sort: Sort::Name, limit: 50, ..Default::default() };
    let page = s.list(&q).unwrap();
    assert_eq!(page.items.len(), 2);

    // Short terms fall back to LIKE on the name.
    let q =
        Query { text: vec!["IMG".into()], sort: Sort::Modified, limit: 50, ..Default::default() };
    let page = s.list(&q).unwrap();
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].name, "IMG_4821.jpg");
}

#[test]
fn pagination_walks_every_item_exactly_once() {
    let s = store();
    seed(&s);
    let mut seen = vec![];
    let mut cursor = None;
    loop {
        let q =
            Query { sort: Sort::Modified, limit: 2, cursor: cursor.clone(), ..Default::default() };
        let page = s.list(&q).unwrap();
        seen.extend(page.items.iter().map(|f| f.id));
        match page.next_cursor {
            Some(c) => cursor = Some(c),
            None => break,
        }
    }
    seen.sort();
    seen.dedup();
    assert_eq!(seen.len(), 6);
}

#[test]
fn kind_filter_and_size_range() {
    let s = store();
    seed(&s);
    let q = Query {
        kinds: vec![FileKind::Document],
        min_size: Some(1_000_000),
        sort: Sort::Size,
        limit: 50,
        ..Default::default()
    };
    let page = s.list(&q).unwrap();
    assert_eq!(page.items.len(), 2);
    assert!(page.items[0].size >= page.items[1].size);
}

#[test]
fn direct_children_scope_excludes_nested_folder_contents() {
    let s = store();
    let direct = sf("/home/u/Downloads/readme.pdf", FileKind::Document, 10, now());
    let nested = sf("/home/u/Downloads/helio/cmake.list", FileKind::Other, 10, now());
    s.upsert_batch(&[direct, nested]).unwrap();

    let q = Query {
        under_path: Some("/home/u/Downloads".into()),
        direct_children: true,
        limit: 50,
        ..Default::default()
    };
    let page = s.list(&q).unwrap();
    assert_eq!(page.items.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(), ["readme.pdf"]);
}

#[test]
fn trash_roundtrip_via_receipt() {
    let s = store();
    seed(&s);
    let orig = PathBuf::from("/home/u/Downloads/design-assets.zip");
    let trashed = PathBuf::from("/home/u/.local/share/Trash/files/design-assets.2.zip");
    s.mark_trashed(&[(orig.clone(), trashed.clone())], Some("test")).unwrap();

    assert!(s.by_path(&orig).unwrap().unwrap().in_trash);
    let q = Query { sort: Sort::Modified, limit: 50, ..Default::default() };
    assert_eq!(s.list(&q).unwrap().items.len(), 5);

    let receipts = s.list_receipts().unwrap();
    assert_eq!(receipts.len(), 1);
    assert_eq!(s.receipt_paths(receipts[0].id).unwrap()[0].1, trashed.to_string_lossy());

    s.mark_restored(receipts[0].id, &[trashed]).unwrap();
    assert!(!s.by_path(&orig).unwrap().unwrap().in_trash);
}

#[test]
fn scan_generation_prunes_only_unseen_old_rows() {
    let s = store();
    let root = Path::new("/home/u");
    let old = now() - 10_000;
    let mut keep = sf("/home/u/Downloads/keep.zip", FileKind::Archive, 10, old);
    keep.created = old;
    let mut stale = sf("/home/u/Downloads/stale.zip", FileKind::Archive, 10, old);
    stale.created = old;
    let mut fresh = sf("/home/u/Downloads/fresh.zip", FileKind::Archive, 10, now() + 100);
    fresh.created = now() + 100;

    let gen1 = s.begin_scan().unwrap();
    s.upsert_batch_at(&[keep.clone(), stale.clone(), fresh.clone()], Some(gen1)).unwrap();
    assert_eq!(s.finish_scan(root, gen1, now() + 1).unwrap(), 0);

    let gen2 = s.begin_scan().unwrap();
    s.upsert_batch_at(&[keep.clone(), fresh.clone()], Some(gen2)).unwrap();
    // `fresh` was not seen again but is newer than the scan start, so it survives.
    assert_eq!(s.finish_scan(root, gen2, now()).unwrap(), 1);
    assert!(s.by_path(&stale.path).unwrap().is_none());
    assert!(s.by_path(&keep.path).unwrap().is_some());
    assert!(s.by_path(&fresh.path).unwrap().is_some());
}

#[test]
fn rename_path_updates_the_row_and_descendants() {
    let s = store();
    let t = now();
    let dir = sf("/home/u/old-project", FileKind::Other, 1, t);
    let child = sf("/home/u/old-project/src/main.rs", FileKind::Other, 10, t);
    s.upsert_batch(&[dir, child]).unwrap();
    s.rename_path(
        Path::new("/home/u/old-project"),
        Path::new("/home/u/redis-project"),
        FileKind::Other,
    )
    .unwrap();
    assert!(s.by_path(Path::new("/home/u/redis-project")).unwrap().is_some());
    assert_eq!(
        s.by_path(Path::new("/home/u/redis-project/src/main.rs")).unwrap().unwrap().parent,
        "/home/u/redis-project/src"
    );
    assert!(s.by_path(Path::new("/home/u/old-project")).unwrap().is_none());
}

#[test]
fn scan_prune_skips_unreadable_subtrees() {
    let s = store();
    let root = PathBuf::from("/home/u");
    let a = sf("/home/u/seen.txt", FileKind::Document, 1, now());
    let b = sf("/home/u/locked/hidden.txt", FileKind::Document, 1, now());
    let c = sf("/home/u/gone.txt", FileKind::Document, 1, now());
    s.upsert_batch(&[a.clone(), b.clone(), c.clone()]).unwrap();
    let generation = s.begin_scan().unwrap();
    s.upsert_file_at(&a, Some(generation)).unwrap();
    let removed = s
        .finish_scan_excluding(&root, generation, now() + 1, &[PathBuf::from("/home/u/locked")])
        .unwrap();
    assert_eq!(removed, 1);
    assert!(s.by_path(&c.path).unwrap().is_none());
    assert!(s.by_path(&a.path).unwrap().is_some());
    assert!(s.by_path(&b.path).unwrap().is_some());
}

#[test]
fn scan_generation_keeps_trashed_rows() {
    let s = store();
    let root = Path::new("/home/u");
    let old = now() - 10_000;
    let mut f = sf("/home/u/Downloads/gone.zip", FileKind::Archive, 10, old);
    f.created = old;
    let gen1 = s.begin_scan().unwrap();
    s.upsert_batch_at(std::slice::from_ref(&f), Some(gen1)).unwrap();
    s.mark_trashed(
        &[(f.path.clone(), PathBuf::from("/home/u/.local/share/Trash/files/gone.zip"))],
        None,
    )
    .unwrap();
    let gen2 = s.begin_scan().unwrap();
    assert_eq!(s.finish_scan(root, gen2, now()).unwrap(), 0);
    assert!(s.by_path(&f.path).unwrap().unwrap().in_trash);
}

#[test]
fn sources_are_stored_and_joined() {
    let s = store();
    seed(&s);
    s.set_source(
        Path::new("/home/u/Downloads/invoice-0387-acme.pdf"),
        "https://mail.google.com/attachment/abc",
        Some("Gmail"),
        Some("mail.google.com"),
    )
    .unwrap();
    let q = Query { sources: vec!["gmail".into()], limit: 50, ..Default::default() };
    let page = s.list(&q).unwrap();
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].origin_app.as_deref(), Some("Gmail"));
}

#[test]
fn frecency_ranks_opened_files_first() {
    let s = store();
    seed(&s);
    s.record_open(Path::new("/home/u/notes.md")).unwrap();
    s.record_open(Path::new("/home/u/notes.md")).unwrap();
    let q = Query { sort: Sort::Frecency, limit: 50, ..Default::default() };
    let page = s.list(&q).unwrap();
    assert_eq!(page.items[0].name, "notes.md");
    assert_eq!(page.items[0].opens, 2);
}

#[test]
fn saved_searches_roundtrip() {
    let s = store();
    s.save_search("Invoices", "pdf from gmail invoice").unwrap();
    s.save_search("Invoices", "pdf from gmail invoice updated").unwrap();
    let all = s.list_saved_searches().unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].query, "pdf from gmail invoice updated");
    s.delete_saved_search(all[0].id).unwrap();
    assert!(s.list_saved_searches().unwrap().is_empty());
}

#[test]
fn kind_stats_and_totals() {
    let s = store();
    seed(&s);
    let stats = s.kind_stats().unwrap();
    let docs = stats.iter().find(|k| k.kind == FileKind::Document).unwrap();
    assert_eq!(docs.count, 3);
    let (count, _bytes) = s.totals().unwrap();
    assert_eq!(count, 6);
}

#[test]
fn tag_filter_matches_all_requested_tags() {
    let s = store();
    let a = sf("/home/u/a.txt", FileKind::Document, 1, now());
    let b = sf("/home/u/b.txt", FileKind::Document, 1, now());
    s.upsert_batch(&[a.clone(), b.clone()]).unwrap();
    s.set_tags(&a.path, &["work".into(), "urgent".into()]).unwrap();
    s.set_tags(&b.path, &["work".into()]).unwrap();
    let q = Query { tags: vec!["work".into(), "urgent".into()], limit: 50, ..Query::default() };
    let page = s.list(&q).unwrap();
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].path, a.path.to_string_lossy());
}

#[test]
fn tags_roundtrip() {
    let s = store();
    seed(&s);
    let p = Path::new("/home/u/Downloads/invoice-0387-acme.pdf");
    s.set_tags(p, &["money".into(), "2026".into()]).unwrap();
    assert_eq!(s.tags_of(p).unwrap(), vec!["2026".to_string(), "money".to_string()]);
    s.set_tags(p, &[]).unwrap();
    assert!(s.tags_of(p).unwrap().is_empty());
}

#[test]
fn file_backed_store_survives_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("index.db");
    {
        let s = Store::open(&db).unwrap();
        s.upsert_batch(&[sf("/home/u/notes.md", FileKind::Document, 900, now())]).unwrap();
    }
    let s = Store::open(&db).unwrap();
    assert_eq!(s.totals().unwrap().0, 1);
}
