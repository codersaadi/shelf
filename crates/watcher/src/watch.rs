use notify_debouncer_full::notify;
use notify_debouncer_full::{
    DebounceEventHandler, DebounceEventResult, DebouncedEvent, RecommendedCache, new_debouncer,
};
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::time::Duration;

/// Coalesced filesystem changes, ready for rescan.
#[derive(Debug)]
pub enum Event {
    /// Created or modified paths (deduped by the debouncer).
    Changed(Vec<PathBuf>),
    /// Paths that no longer exist.
    Removed(Vec<PathBuf>),
    /// The kernel dropped events (inotify queue overflow) or a watch failed;
    /// only a full rescan can restore certainty.
    Rescan,
}

struct Relay {
    tx: Sender<Event>,
}

impl DebounceEventHandler for Relay {
    fn handle_event(&mut self, events: DebounceEventResult) {
        match events {
            Ok(events) => {
                if events.iter().any(|e| e.event.need_rescan()) {
                    let _ = self.tx.send(Event::Rescan);
                }
                let mut changed = vec![];
                let mut removed = vec![];
                for e in &events {
                    classify(e, &mut changed, &mut removed);
                }
                if !changed.is_empty() {
                    let _ = self.tx.send(Event::Changed(changed));
                }
                if !removed.is_empty() {
                    let _ = self.tx.send(Event::Removed(removed));
                }
            }
            Err(err) => tracing::warn!("watch error: {err:?}"),
        }
    }
}

fn classify(e: &DebouncedEvent, changed: &mut Vec<PathBuf>, removed: &mut Vec<PathBuf>) {
    use notify::EventKind;
    match e.event.kind {
        EventKind::Remove(_) => removed.extend(e.event.paths.iter().cloned()),
        EventKind::Create(_) | EventKind::Modify(_) => {
            changed.extend(e.event.paths.iter().cloned())
        }
        _ => {}
    }
}

/// Watching handle; dropping it stops watching.
pub struct Watcher {
    _debouncer: notify_debouncer_full::Debouncer<notify::RecommendedWatcher, RecommendedCache>,
}

/// Start watching `roots` recursively. Debounce window 600 ms.
///
/// Watches as many roots as possible; one unavailable root (inotify watch
/// limit, ENOSPC) must not leave every root unwatched. Errors are logged, and
/// the call fails only when no root is watchable, so the caller can fall back.
pub fn start_watching(roots: &[PathBuf], tx: Sender<Event>) -> Result<Watcher, notify::Error> {
    if roots.is_empty() {
        return Err(notify::Error::generic("no roots to watch"));
    }
    let mut debouncer = new_debouncer(Duration::from_millis(600), None, Relay { tx })?;
    let mut succeeded = 0usize;
    let mut last_err = None;
    for root in roots {
        match debouncer.watch(root, notify::RecursiveMode::Recursive) {
            Ok(()) => succeeded += 1,
            Err(err) => {
                tracing::warn!("watch root unavailable {}: {err}", root.display());
                last_err = Some(err);
            }
        }
    }
    if succeeded > 0 {
        Ok(Watcher { _debouncer: debouncer })
    } else {
        Err(last_err.unwrap_or_else(|| notify::Error::generic("no roots to watch")))
    }
}
