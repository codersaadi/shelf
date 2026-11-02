use crate::detect::detect_kind;
use md5::{Digest, Md5};
use percent_encoding::{AsciiSet, CONTROLS, percent_encode};
use shelf_core::FileKind;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Condvar, Mutex};

/// The freedesktop thumbnail spec: `$XDG_CACHE_HOME/thumbnails/{normal,large}/<md5(uri)>.png`.
/// We read that cache before generating our own (plan v0.1).
const URI_SET: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'<')
    .add(b'>')
    .add(b'?')
    .add(b'`')
    .add(b'{')
    .add(b'}')
    .add(b'\\')
    .add(b'^')
    .add(b'%');

/// Cap concurrent decode/generate work so fast scrolling cannot saturate every
/// CPU while the user is still moving the viewport.
const MAX_GENERATIONS: usize = 4;
static ACTIVE: Mutex<usize> = Mutex::new(0);
static ACTIVE_CV: Condvar = Condvar::new();
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

struct Slot;

impl Drop for Slot {
    fn drop(&mut self) {
        let mut active = ACTIVE.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        *active = active.saturating_sub(1);
        ACTIVE_CV.notify_one();
    }
}

fn generation_slot() -> Slot {
    let mut active = ACTIVE.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    while *active >= MAX_GENERATIONS {
        active = ACTIVE_CV.wait(active).unwrap_or_else(|e| e.into_inner());
    }
    *active += 1;
    Slot
}

pub fn file_uri(path: &Path) -> String {
    let enc = percent_encode(path.as_os_str().as_encoded_bytes(), URI_SET).to_string();
    format!("file://{enc}")
}

fn cache_root() -> PathBuf {
    dirs::cache_dir()
        .map(|c| c.join("thumbnails"))
        .unwrap_or_else(|| PathBuf::from(".cache/thumbnails"))
}

/// Freedesktop caches we may read, primary first. The app can run with a
/// redirected XDG cache (Flatpak, tests, demos) while the desktop's own cache
/// still has useful thumbnails, so check both.
fn cache_roots() -> Vec<PathBuf> {
    let mut roots = vec![cache_root()];
    if let Some(home) = dirs::home_dir() {
        let fallback = home.join(".cache").join("thumbnails");
        if !roots.contains(&fallback) {
            roots.push(fallback);
        }
    }
    roots
}

fn uri_hash(file: &Path) -> String {
    let mut hasher = Md5::new();
    hasher.update(file_uri(file).as_bytes());
    hasher.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// Where the freedesktop cache would store this file's thumbnail.
pub fn freedesktop_thumb_path(file: &Path, large: bool) -> PathBuf {
    cache_root()
        .join(if large { "large" } else { "normal" })
        .join(format!("{}.png", uri_hash(file)))
}

/// Cached thumbnail that exists and is fresh enough to show (mtime >= file mtime is
/// not required by the spec; we trust the cache like Nautilus does).
pub fn find_cached_thumb(file: &Path) -> Option<PathBuf> {
    let hash = uri_hash(file);
    for root in cache_roots() {
        for large in [true, false] {
            let p = root.join(if large { "large" } else { "normal" }).join(format!("{hash}.png"));
            if p.is_file() {
                return Some(p);
            }
        }
    }
    None
}

/// Where we generate our own thumbnails when the cache misses.
pub fn cache_dir() -> PathBuf {
    cache_root().join("shelf")
}

fn mtime_secs(file: &Path) -> i64 {
    fs::metadata(file)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Our cache key includes mtime, so editing a file never serves a stale frame.
fn cache_key(file: &Path, size: u32) -> String {
    let mut hasher = Md5::new();
    hasher.update(file.as_os_str().as_encoded_bytes());
    hasher.update(mtime_secs(file).to_le_bytes());
    hasher.update(size.to_le_bytes());
    hasher.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

pub fn cached_thumb_path(file: &Path, size: u32) -> PathBuf {
    cache_dir().join(format!("{}-{size}.png", cache_key(file, size)))
}

fn temp_thumb_path(file: &Path, size: u32) -> PathBuf {
    let sequence = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "shelf-thumb-{}-{size}-{}-{sequence}.png",
        cache_key(file, size),
        std::process::id()
    ))
}

fn cache_bytes(dest: &Path, bytes: &[u8]) {
    if let Some(parent) = dest.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let sequence = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let Some(name) = dest.file_name().and_then(|name| name.to_str()) else {
        return;
    };
    let tmp = dest.with_file_name(format!(".{name}.{sequence}.tmp"));
    if fs::write(&tmp, bytes).is_ok() {
        if fs::rename(&tmp, dest).is_err() {
            let _ = fs::remove_file(&tmp);
        }
    } else {
        let _ = fs::remove_file(&tmp);
    }
}

/// Decode + downscale an image to fit `size` and return PNG bytes.
/// Only formats the `image` crate compiled in; None for everything else.
pub fn generate_png_thumb(file: &Path, size: u32) -> Option<Vec<u8>> {
    use image::ImageEncoder as _;
    let img = image::ImageReader::open(file).ok()?.decode().ok()?;
    let thumb = img.thumbnail(size, size);
    let mut out = std::io::Cursor::new(Vec::with_capacity(64 * 1024));
    image::codecs::png::PngEncoder::new(&mut out)
        .write_image(thumb.as_bytes(), thumb.width(), thumb.height(), thumb.color().into())
        .ok()?;
    Some(out.into_inner())
}

/// PNG bytes for any supported type: images, video frames, PDF page 1 and
/// embedded audio covers. Results are cached on disk and keyed by mtime.
pub fn thumbnail_png(file: &Path, size: u32) -> Option<Vec<u8>> {
    let size = size.clamp(64, 1024);
    let dest = cached_thumb_path(file, size);
    if let Some(bytes) = fs::read(&dest).ok().filter(|b| !b.is_empty()) {
        return Some(bytes);
    }

    let _slot = generation_slot();

    match detect_kind(file) {
        // Image decodes are cheap enough to run in parallel.
        FileKind::Image => {
            let bytes = generate_png_thumb(file, size)?;
            cache_bytes(&dest, &bytes);
            Some(bytes)
        }
        // External decoders need a writable output. Generate in the temp dir so
        // a read-only XDG cache (tests, some sandboxes) still yields a preview,
        // then copy into the cache when possible.
        FileKind::Video => external_thumb(file, size, &dest, External::Frame),
        FileKind::Music => external_thumb(file, size, &dest, External::Cover),
        FileKind::Document if is_pdf(file) => external_thumb(file, size, &dest, External::Pdf),
        _ => None,
    }
}

/// Generate and store a thumbnail for `file` in our cache; returns its path.
pub fn make_and_cache_thumb(file: &Path, size: u32) -> Option<PathBuf> {
    thumbnail_png(file, size)?;
    let path = cached_thumb_path(file, size.clamp(64, 1024));
    path.is_file().then_some(path)
}

enum External {
    Frame,
    Cover,
    Pdf,
}

fn external_thumb(file: &Path, size: u32, dest: &Path, kind: External) -> Option<Vec<u8>> {
    let work = temp_thumb_path(file, size);
    let _ = fs::remove_file(&work);
    let generated = match kind {
        External::Frame => ffmpeg_frame(file, size, &work, false),
        External::Cover => ffmpeg_frame(file, size, &work, true),
        External::Pdf => pdf_frame(file, size, &work),
    };
    let bytes = if generated.is_some() { fs::read(&work).ok() } else { None };
    let _ = fs::remove_file(&work);
    if let Some(bytes) = &bytes {
        cache_bytes(dest, bytes);
    }
    bytes
}

fn is_pdf(file: &Path) -> bool {
    file.extension().is_some_and(|e| e.eq_ignore_ascii_case("pdf"))
}

fn ffmpeg_frame(file: &Path, size: u32, dest: &Path, cover: bool) -> Option<()> {
    // A video's first frame is often black; one second in is a better poster.
    let seeks: &[&str] = if cover { &["0"] } else { &["1", "0"] };
    for seek in seeks {
        let mut cmd = Command::new("ffmpeg");
        cmd.args(["-v", "error", "-y", "-ss", seek, "-i"]).arg(file);
        if cover {
            cmd.args(["-an"]);
        }
        cmd.args(["-frames:v", "1", "-vf", &format!("scale=w='min({size},iw)':h=-2")]);
        cmd.args(["-f", "image2"]).arg(dest);
        let status =
            cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).status().ok()?;
        if status.success() && fs::metadata(dest).is_ok_and(|m| m.len() > 0) {
            return Some(());
        }
    }
    None
}

fn pdf_frame(file: &Path, size: u32, dest: &Path) -> Option<()> {
    let prefix = dest.with_extension("");
    let status = Command::new("pdftoppm")
        .args(["-f", "1", "-l", "1", "-singlefile", "-scale-to", &size.to_string(), "-png"])
        .arg(file)
        .arg(&prefix)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .ok()?;
    (status.success() && dest.is_file()).then_some(())
}
