#![allow(clippy::unwrap_used, clippy::expect_used)]

use shelf_platform::{read_origin, read_tags_xattr, write_tags_xattr};
use std::fs;

#[test]
fn tags_xattr_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("doc.pdf");
    fs::write(&file, b"pdf").unwrap();

    assert!(read_tags_xattr(&file).is_empty());
    write_tags_xattr(&file, &["money".to_string(), "2026".to_string()]).unwrap();
    assert_eq!(read_tags_xattr(&file), vec!["money".to_string(), "2026".to_string()]);
    write_tags_xattr(&file, &[]).unwrap();
    assert!(read_tags_xattr(&file).is_empty());
}

#[test]
fn origin_absent_by_default() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("doc.pdf");
    fs::write(&file, b"pdf").unwrap();
    let origin = read_origin(&file);
    assert!(origin.url.is_none());
    assert!(origin.referrer.is_none());
    assert!(origin.app.is_none());
}
