use shelf_core::FileKind;
use std::fs::File;
use std::io::Read;
use std::path::Path;

const IMAGE: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "webp", "bmp", "tif", "tiff", "heic", "heif", "avif", "jfif",
    "svg",
];
const VIDEO: &[&str] =
    &["mp4", "mkv", "mov", "avi", "webm", "m4v", "mpg", "mpeg", "wmv", "flv", "m2ts", "3gp", "ogv"];
const MUSIC: &[&str] =
    &["mp3", "flac", "wav", "ogg", "oga", "m4a", "aac", "opus", "wma", "aiff", "mid"];
const DOCUMENT: &[&str] = &[
    "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx", "odt", "ods", "odp", "md", "markdown",
    "mdx", "txt", "rtf", "csv", "tsv", "tex", "epub", "djvu", "docm", "xlsm", "pptm", "pages",
];
const ARCHIVE: &[&str] =
    &["zip", "tar", "gz", "bz2", "xz", "zst", "7z", "rar", "tgz", "tbz2", "txz", "tzst", "iso"];
// `.pak` is usually an application/resource bundle (Chromium, games, SDKs),
// not an installable application. Keep it out of the Apps shelf.
const APP: &[&str] = &["deb", "rpm", "appimage", "flatpak", "desktop"];

fn ext_of(path: &Path) -> Option<String> {
    path.extension().map(|e| e.to_string_lossy().to_lowercase())
}

pub fn detect_kind(path: &Path) -> FileKind {
    let Some(ext) = ext_of(path) else {
        return FileKind::Other;
    };
    // Multi-part archives beat single extensions: "tar.gz" ends in "gz".
    let name = path.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
    if name.ends_with(".tar.gz")
        || name.ends_with(".tar.bz2")
        || name.ends_with(".tar.xz")
        || name.ends_with(".tar.zst")
    {
        return FileKind::Archive;
    }
    // ".ts" is TypeScript far more often than MPEG transport stream on a
    // developer's disk. Sniff the packet sync bytes before claiming video.
    if ext == "ts" {
        return if is_mpeg_ts(path) { FileKind::Video } else { FileKind::Other };
    }
    for (list, kind) in [
        (IMAGE, FileKind::Image),
        (VIDEO, FileKind::Video),
        (MUSIC, FileKind::Music),
        (DOCUMENT, FileKind::Document),
        (ARCHIVE, FileKind::Archive),
        (APP, FileKind::App),
    ] {
        if list.contains(&ext.as_str()) {
            return kind;
        }
    }
    FileKind::Other
}

fn is_mpeg_ts(path: &Path) -> bool {
    let mut buf = [0u8; 189];
    let Ok(mut file) = File::open(path) else {
        return false;
    };
    let Ok(n) = file.read(&mut buf) else {
        return false;
    };
    n > 0 && buf[0] == 0x47 && (n < 189 || buf[188] == 0x47)
}

/// Screenshot heuristic: GNOME ("Screenshot 2026-10-14 at 14.32.07.png"),
/// KDE ("Screenshot_20261014_143207.png"), macOS ("Screen Shot ..."), or a
/// Screenshots parent directory.
pub fn looks_like_screenshot(path: &Path) -> bool {
    if detect_kind(path) != FileKind::Image {
        return false;
    }
    if let Some(parent) = path.parent().and_then(|p| p.file_name())
        && (parent.eq_ignore_ascii_case("Screenshots") || parent.eq_ignore_ascii_case("CAP2024"))
    {
        return true;
    }
    let name = path.file_name().map(|n| n.to_string_lossy()).unwrap_or_default();
    name.starts_with("Screenshot")
        || name.starts_with("Screen Shot")
        || name.starts_with("screencapture")
        || name.to_lowercase().contains("screenshot_")
}
