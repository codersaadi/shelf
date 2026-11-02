use crate::state::AppState;
use std::borrow::Cow;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use tauri::http::{Request, Response, StatusCode};
use tauri::{Manager, UriSchemeContext, UriSchemeResponder};

/// Thumbnail work is queued to a small worker pool. The protocol callback runs
/// on WebKit's resource thread; returning immediately keeps scrolling and other
/// asset loads responsive even when several videos need a first frame.
struct Job {
    path: PathBuf,
    size: u32,
    responder: UriSchemeResponder,
}

struct Queue {
    jobs: Mutex<VecDeque<Job>>,
    ready: Condvar,
}

static QUEUE: OnceLock<Arc<Queue>> = OnceLock::new();

fn queue() -> &'static Arc<Queue> {
    QUEUE.get_or_init(|| {
        let queue = Arc::new(Queue { jobs: Mutex::new(VecDeque::new()), ready: Condvar::new() });
        let workers =
            std::thread::available_parallelism().map(|n| n.get().clamp(2, 4)).unwrap_or(4);
        for i in 0..workers {
            let queue = queue.clone();
            let spawned =
                std::thread::Builder::new().name(format!("shelf-thumb-{i}")).spawn(move || {
                    loop {
                        let job = {
                            let mut jobs = queue
                                .jobs
                                .lock()
                                .unwrap_or_else(std::sync::PoisonError::into_inner);
                            while jobs.is_empty() {
                                jobs = queue.ready.wait(jobs).unwrap_or_else(|e| e.into_inner());
                            }
                            jobs.pop_front()
                        };
                        let Some(job) = job else { break };
                        let response = match shelf_media::thumbnail_png(&job.path, job.size) {
                            Some(bytes) => png(bytes),
                            None => empty(StatusCode::NOT_FOUND),
                        };
                        job.responder.respond(response);
                    }
                });
            if spawned.is_err() {
                tracing::error!("could not start a thumbnail worker");
            }
        }
        queue
    })
}

fn enqueue(job: Job) {
    let queue = queue();
    queue.jobs.lock().unwrap_or_else(std::sync::PoisonError::into_inner).push_back(job);
    queue.ready.notify_one();
}

/// `thumb://localhost/<file id>?s=<px>&v=<mtime>` serves a cached PNG immediately
/// and queues generation on a miss.
pub fn handle(
    ctx: UriSchemeContext<'_, tauri::Wry>,
    request: Request<Vec<u8>>,
    responder: UriSchemeResponder,
) {
    let app = ctx.app_handle();
    let uri = request.uri();
    let trash_path = uri.query().and_then(|q| {
        q.split('&').find_map(|kv| kv.strip_prefix("p=")).map(|raw| {
            let decoded = percent_encoding::percent_decode_str(raw).decode_utf8_lossy();
            PathBuf::from(decoded.as_ref())
        })
    });
    let id = uri
        .path()
        .trim_start_matches('/')
        .parse::<i64>()
        .ok()
        .or_else(|| uri.host().and_then(|h| h.parse::<i64>().ok()));
    let Some(id) = id else {
        responder.respond(empty(StatusCode::BAD_REQUEST));
        return;
    };
    let size = uri
        .query()
        .and_then(|q| q.split('&').find_map(|kv| kv.strip_prefix("s=")))
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(320)
        .clamp(64, 1024);

    let full = uri.query().is_some_and(|q| q.split('&').any(|kv| kv == "full=1"));

    let (path, kind) = if let Some(path) = trash_path {
        if !shelf_platform::is_trash_file_path(&path) || !path.is_file() {
            responder.respond(empty(StatusCode::NOT_FOUND));
            return;
        }
        (path, shelf_core::FileKind::Other)
    } else {
        let state = app.state::<AppState>();
        let Ok(Some(file)) = state.store.get_files(&[id]).map(|mut v| v.pop()) else {
            responder.respond(empty(StatusCode::NOT_FOUND));
            return;
        };
        (PathBuf::from(&file.path), file.kind)
    };
    if !path.is_file() {
        responder.respond(empty(StatusCode::NOT_FOUND));
        return;
    }

    if full {
        if kind != shelf_core::FileKind::Image {
            responder.respond(empty(StatusCode::NOT_FOUND));
            return;
        }
        match full_image(&path) {
            Some(response) => responder.respond(response),
            None => responder.respond(empty(StatusCode::NOT_FOUND)),
        }
        return;
    }

    if let Some(bytes) = cached_png(&path, size) {
        responder.respond(png(bytes));
        return;
    }

    enqueue(Job { path, size, responder });
}

fn full_image(path: &Path) -> Option<Response<Cow<'static, [u8]>>> {
    // A Quick Look should not pull an unbounded original into the WebView.
    if std::fs::metadata(path).ok()?.len() > 64 * 1024 * 1024 {
        return None;
    }
    let bytes = std::fs::read(path).ok()?;
    let ext = path.extension()?.to_string_lossy().to_lowercase();
    let mime = match ext.as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "avif" => "image/avif",
        "bmp" => "image/bmp",
        "svg" => "image/svg+xml",
        _ => return None,
    };
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", mime)
        .header("Access-Control-Allow-Origin", "*")
        .header("Cache-Control", "private, max-age=60")
        .body(Cow::Owned(bytes))
        .ok()
}

fn cached_png(path: &Path, size: u32) -> Option<Vec<u8>> {
    if let Some(cached) = shelf_media::find_cached_thumb(path)
        && let Ok(bytes) = std::fs::read(&cached)
        && !bytes.is_empty()
    {
        return Some(bytes);
    }
    let generated = shelf_media::cached_thumb_path(path, size);
    std::fs::read(generated).ok().filter(|bytes| !bytes.is_empty())
}

fn png(bytes: Vec<u8>) -> Response<Cow<'static, [u8]>> {
    match Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "image/png")
        .header("Access-Control-Allow-Origin", "*")
        .header("Cache-Control", "public, max-age=86400")
        .body(Cow::Owned(bytes))
    {
        Ok(response) => response,
        Err(_) => empty(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

fn empty(status: StatusCode) -> Response<Cow<'static, [u8]>> {
    let mut response = Response::new(Cow::Borrowed(&[][..]));
    *response.status_mut() = status;
    response
}
