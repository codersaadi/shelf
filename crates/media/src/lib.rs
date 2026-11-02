mod detect;
mod text;
mod thumb;

pub use detect::{detect_kind, looks_like_screenshot};
pub use text::{TextSnippet, text_snippet};
pub use thumb::{
    cache_dir, cached_thumb_path, file_uri, find_cached_thumb, freedesktop_thumb_path,
    generate_png_thumb, make_and_cache_thumb, thumbnail_png,
};

use shelf_core::ScannedFile;
use std::fs;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Build a `ScannedFile` from filesystem metadata, or None for directories/symlinks.
pub fn to_scanned(path: &Path) -> Option<ScannedFile> {
    let md = fs::symlink_metadata(path).ok()?;
    if md.is_dir() || md.file_type().is_symlink() {
        return None;
    }
    Some(ScannedFile {
        size: md.len(),
        mtime: md
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0),
        created: created_of(&md),
        duration_ms: None,
        inode: {
            #[allow(unused_imports)]
            use std::os::unix::fs::MetadataExt;
            Some(md.ino())
        },
        path: path.to_path_buf(),
        kind: detect_kind(path),
        is_screenshot: looks_like_screenshot(path),
    })
}

/// Outcome of a bounded external run: status plus captured stdout.
pub(crate) struct Run {
    pub ok: bool,
    pub stdout: Vec<u8>,
}

/// Spawn `cmd` and wait, killing it at the deadline. Without this a single
/// wedged ffprobe (corrupt file, stalled mount) would freeze indexing or
/// starve thumbnail generation for the rest of the process lifetime.
pub(crate) fn run_with_timeout(cmd: &mut Command, timeout: Duration) -> Option<Run> {
    // Stdout is captured only for commands with known-small output; stop
    // reading there so a runaway child cannot grow memory through this API.
    const MAX_CAPTURE: u64 = 256 * 1024;

    let mut child = cmd.spawn().ok()?;
    let deadline = Instant::now() + timeout;
    let exited = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(25));
            }
            Ok(None) => break None,
            Err(_) => break None,
        }
    };
    match exited {
        Some(status) => {
            let stdout = child
                .stdout
                .take()
                .map(|s| {
                    let mut buf = Vec::new();
                    s.take(MAX_CAPTURE).read_to_end(&mut buf).ok();
                    buf
                })
                .unwrap_or_default();
            Some(Run { ok: status.success(), stdout })
        }
        None => {
            let _ = child.kill();
            let _ = child.wait();
            None
        }
    }
}

pub fn probe_duration_ms(path: &Path) -> Option<u64> {
    let mut cmd = Command::new("ffprobe");
    cmd.args(["-v", "error", "-show_entries", "format=duration", "-of", "default=nw=1:nk=1"])
        .arg(path)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .stdout(Stdio::piped());
    let run = run_with_timeout(&mut cmd, Duration::from_secs(10))?;
    if !run.ok {
        return None;
    }
    parse_seconds(&String::from_utf8_lossy(&run.stdout))
}

fn parse_seconds(text: &str) -> Option<u64> {
    let seconds: f64 = text.trim().parse().ok()?;
    // Absurd values are junk, not durations; nothing real is longer than a day.
    (seconds.is_finite() && (0.0..86_400.0).contains(&seconds))
        .then(|| (seconds * 1000.0).round() as u64)
}

pub fn image_dimensions(path: &Path) -> Option<(u32, u32)> {
    image::image_dimensions(path).ok()
}

pub fn created_of(md: &fs::Metadata) -> i64 {
    md.created()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .filter(|t| *t > 0)
        .unwrap_or_else(|| {
            md.modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0)
        })
}
