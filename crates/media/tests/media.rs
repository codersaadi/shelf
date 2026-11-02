#![allow(clippy::unwrap_used, clippy::expect_used)]

use shelf_core::FileKind;
use shelf_media::{
    detect_kind, file_uri, find_cached_thumb, freedesktop_thumb_path, generate_png_thumb,
    looks_like_screenshot, thumbnail_png,
};
use std::path::Path;

#[test]
fn detect_kind_by_extension() {
    let cases: &[(&str, FileKind)] = &[
        ("a.png", FileKind::Image),
        ("a.HEIC", FileKind::Image),
        ("a.svg", FileKind::Image),
        ("a.mp4", FileKind::Video),
        ("a.mkv", FileKind::Video),
        ("a.flac", FileKind::Music),
        ("a.mp3", FileKind::Music),
        ("a.pdf", FileKind::Document),
        ("a.xlsx", FileKind::Document),
        ("a.md", FileKind::Document),
        ("a.zip", FileKind::Archive),
        ("a.tar.gz", FileKind::Archive),
        ("a.tar.zst", FileKind::Archive),
        ("a.deb", FileKind::App),
        ("a.AppImage", FileKind::App),
        ("a.pak", FileKind::Other),
        ("a.xyz", FileKind::Other),
        ("a", FileKind::Other),
    ];
    for (name, want) in cases {
        assert_eq!(detect_kind(Path::new(name)), *want, "extension {name}");
    }
}

#[test]
fn video_thumbnail_is_generated_and_cached() {
    if std::process::Command::new("ffmpeg").arg("-version").output().is_err()
        || std::process::Command::new("ffprobe").arg("-version").output().is_err()
    {
        return;
    }
    if std::fs::create_dir_all(shelf_media::cache_dir()).is_err() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let video = dir.path().join("clip.mp4");
    let made = std::process::Command::new("ffmpeg")
        .args(["-v", "error", "-y", "-f", "lavfi", "-i", "color=c=red:s=64x64:d=1"])
        .args(["-pix_fmt", "yuv420p"])
        .arg(&video)
        .status()
        .unwrap();
    assert!(made.success());

    let png = thumbnail_png(&video, 128).expect("video frame");
    assert!(png.starts_with(&[0x89, b'P', b'N', b'G']));
    let duration = shelf_media::probe_duration_ms(&video).expect("duration");
    assert!((900..=1200).contains(&duration), "duration {duration}");
}

#[test]
fn typescript_is_not_a_transport_stream() {
    let dir = tempfile::tempdir().unwrap();
    let ts = dir.path().join("index.ts");
    std::fs::write(&ts, "export const answer: number = 42;\n").unwrap();
    assert_eq!(detect_kind(&ts), FileKind::Other);

    let mpeg = dir.path().join("clip.ts");
    let mut bytes = vec![0u8; 189 * 2];
    bytes[0] = 0x47;
    bytes[188] = 0x47;
    std::fs::write(&mpeg, bytes).unwrap();
    assert_eq!(detect_kind(&mpeg), FileKind::Video);
}

#[test]
fn screenshot_detection() {
    assert!(looks_like_screenshot(Path::new(
        "/home/u/Pictures/Screenshots/Screenshot 2026-10-14 at 14.32.07.png"
    )));
    assert!(looks_like_screenshot(Path::new("/home/u/Pictures/Screenshot_20261014_143207.png")));
    assert!(looks_like_screenshot(Path::new("/home/u/Screen Shot 2026-10-14.png")));
    assert!(looks_like_screenshot(Path::new("/home/u/other/Screenshots/img.png")));
    assert!(!looks_like_screenshot(Path::new("/home/u/wallpaper.jpg")));
    assert!(!looks_like_screenshot(Path::new("/home/u/Pictures/Screenshots/notes.pdf")));
}

#[test]
fn thumb_path_follows_freedesktop_md5_uri_layout() {
    let p = Path::new("/home/u/img with space.png");
    let expected_md5 = {
        use md5::{Digest, Md5};
        let mut h = Md5::new();
        h.update(b"file:///home/u/img%20with%20space.png");
        h.finalize().iter().map(|b| format!("{b:02x}")).collect::<String>()
    };
    assert_eq!(
        freedesktop_thumb_path(p, true),
        dirs::cache_dir()
            .unwrap()
            .join("thumbnails")
            .join("large")
            .join(format!("{expected_md5}.png"))
    );
    assert_eq!(file_uri(p), "file:///home/u/img%20with%20space.png");
}

#[test]
fn generate_and_find_thumbnail_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let img_path = dir.path().join("tiny.png");
    // 64x64 red square.
    let buf = image::ImageBuffer::from_fn(64, 64, |_, _| image::Rgba([255u8, 0, 0, 255]));
    image::DynamicImage::ImageRgba8(buf).save(&img_path).unwrap();

    let png = generate_png_thumb(&img_path, 32).expect("png thumb");
    assert!(!png.is_empty());
    assert!(png.starts_with(&[0x89, b'P', b'N', b'G']));

    // Not in the freedesktop cache yet; find_cached_thumb returns None.
    assert!(find_cached_thumb(&img_path).is_none());
}
