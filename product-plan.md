Product Plan: Local-First File Intelligence for Linux (Rust)
Working name: Shelf (placeholder). Platforms: Linux first, then macOS, then Windows. Language: Rust.
1. Vision
A fast, clean, private app that shows people their files the way they think about them (recent downloads, screenshots, documents, media), finds anything with plain-English search, and safely frees disk space. It works alongside the system file manager, not instead of it. No cloud, no account, no third-party AI services.
2. Principles
• Fast by default. The window opens instantly, scrolling never stutters, and the background indexer is nearly invisible.
• Efficient. Strict CPU and RAM budgets. Idle cost is close to zero.
• Private. Everything runs on the device. The only network calls are optional update and license checks, plus opt-in model downloads.
• Safe. Nothing is ever permanently deleted. Everything goes to the Trash, with a preview before any action.
• Intelligent without a cloud. Deterministic rules and small optional on-device models.
• Explainable. Every search interpretation and every cleanup suggestion shows why.
• Reversible. Only Trash and explicit organize rules move a file; every batch has a receipt and an Undo.
3. Target users
• General desktop users who lose files in Downloads and screenshots.
• Power users and developers with large, messy home directories.
• People who left macOS and miss Homy-style browsing and search.
4. Problems we solve
Problem
What the user expects
Our answer
Where did my download go?
See newest files immediately
Downloads-first home, "Just added" strip, recent timeline, tray list of last 10 downloads
Where did this come from?
Know the source site or app
Download source tracking (browser history, xattrs) with a From filter
I can't find that file
Search by memory, not filename
Plain-English search shown as editable filter chips
I can't find that screenshot
Search the words inside images
On-device OCR index of screenshots, photos and PDFs
My disk is full
See what's safe to remove
Clean Up cards with sizes and reasons
Too many copies
Remove duplicates safely
Exact and look-alike duplicate detection
Linux clutter
Old packages, caches, build output
Linux-specific cleanup rules (see section 6)
Browsing is a list of names
Big, clear thumbnails
Virtualized thumbnail grid per category
I just saved it and can't find it
See new files immediately
"Just added" strip plus live new-file events
I remember the picture, not the name
Find it by what it looks like
On-device image embeddings and search-by-example (v0.4)
The words are inside a screenshot
Search the text inside images
On-device OCR over screenshots and PDFs (v0.2)
I want to look without opening an app
Preview in place
Space opens Quick Look at full size
I need to undo that cleanup
Undo the whole batch
Trash receipts with one-click Undo
I want to know what leaves my machine
Nothing does, and I can verify it
Privacy dashboard lists every network call; all are opt-in

5. Feature scope by version
v0.1: Foundation (MVP)
• Single-process background indexer with file watcher and SQLite metadata index. Service-trait shaped so the daemon split is a deployment change, not a rewrite.
• Home with a "Just added" strip and category shelves: Images, Videos, Music, Documents, Archives, Apps, plus Recent.
• Virtualized thumbnail grid and list view, sort by name, date, size, kind and frecency; video duration in the file model. PDF first-page covers and safe text/Markdown card snippets.
• Fast filename search (SQLite FTS5 trigram) with the natural-language parser for type, date, size, source and place.
• Inspector with preview and metadata; Spacebar Quick Look for a full-size preview; image, video and PDF thumbnails cached locally.
• Right-click actions: Open, Open with, Show in folder, Copy path, Move to Trash; destructive actions show a preflight and keep an Undo receipt.
• Move to Trash, Copy Path, and Show in Files through the platform layer; toast Undo backed by a receipt.
• Reuse the freedesktop thumbnail cache before generating our own.
• Basic Clean Up findings from the indexed metadata (installers in Downloads, archives, old screenshots, files over 1 GB) with review and dry run. The full rule engine lands in v0.2.
• Tauri 2 + Svelte 5 shell over a UI-free core, with a CLI that mirrors every action.
v0.2: Intelligence
• On-device OCR and full-text index for documents and images.
• Duplicate finder: exact (hash) and look-alike photos (perceptual hash).
• Full Clean Up rule engine with review pages, plus a treemap of disk usage.
• Linux-specific cleanup rules.
• Saved searches in the sidebar.
• Trash receipts that survive restarts, with Undo from the Trash view.
• Shared tags through user.xdg.tags, and tag filters.
• Timeline view with hover-scrub previews.
• Privacy dashboard: one page listing the only network calls (update and license checks, plus opt-in model downloads).
v0.3: Everywhere
• Quick Search launcher panel, with a Wayland portal shortcut and a documented fallback command.
• Tray icon with the last 10 downloads, draggable into other apps.
• Download source tracking and the From filter.
• Auto-organize rules (off by default): move, tag or file by rule, always previewed and undoable. Replaces automatic cleanup rules.
• KRunner / desktop search-provider integration.
• Command palette for every action.
• Settings, themes (light, dark, minimal) and keyboard shortcuts.
v0.4: Semantic and cross-platform
• Search by example: point at a photo, find similar ones with on-device image embeddings.
• Semantic text search over documents using local text embeddings.
• Speech transcription with whisper.cpp for audio and video files.
• File stacks: related files grouped as one shelf item.
• macOS port (FSEvents, Trash, Spotlight coexistence).
• Packaging: Flatpak, AppImage, .deb and .rpm.
Later
• Windows port, plugin API for cleanup and organize rules, optional encrypted multi-device license handling, archive-content search (inside zip/tar).

6. Cleanup and organization rules (differentiator)
• Installers already installed: .deb, .rpm, AppImage and archives in Downloads.
• Package manager caches: apt, dnf and pacman caches (report only, since these usually need root).
• Unused Flatpak runtimes and old Snap revisions.
• Large ~/.cache directories, with per-app attribution.
• Stale build output: node_modules, target/, .venv, __pycache__ in projects untouched for N months.
• Old Docker and Podman images and volumes (report with a command hint, never delete automatically).
• Old logs and core dumps in the user's home.
• Old screenshots and screen recordings.
• Organization rules: file Downloads by kind, project or date; tag screenshots by app; archive stale projects. Organization moves files, never deletes them.
Rule-engine requirement: every rule declares an estimate of reclaimable or repositioned bytes, a plain-language explanation, a risk level and a safe action. Safe actions are Trash (with a receipt and Undo) or an organize step (move/tag, previewed as a dry run). System-level items are report-only with a copyable command.

7. Intelligence design (no third-party AI)
Deterministic layer (ships in v0.1 and v0.2)
• Query grammar and parser for type (pdf, screenshot, video), dates (last week, in August), sizes (over 5 MB), source (from Gmail), and location. Output is a structured filter that renders as editable chips. No LLM is needed.
• Frecency ranking from the user's own opens and searches, stored locally.
• Classifiers (rule-based): screenshot detection by filename, dimensions and directory; installer detection; burst photo grouping by time; "never opened" detection.
• Organize and cleanup rules: each rule is a small, testable function over indexed metadata, with a dry-run preview.
Optional model layer (v0.2 and later, opt-in download)
• OCR: start with ocrs (pure Rust) or Tesseract, and benchmark both on screenshots.
• Perceptual hashing for near-duplicate images.
• Image embeddings (CLIP-class) via ort or candle for semantic photo search and search-by-example (v0.4).
• Text embeddings over extracted text for semantic search (v0.4).
• Whisper.cpp transcription for audio and video (v0.4).
• Models are downloaded on demand, versioned, hash-verified and stored under the XDG data directory.

8. Architecture
A Cargo workspace with a UI-free core, so ports only replace the platform and UI layers. v0.1 runs one process: the UI, indexer and watcher share a `Store`; a service trait marks the seam. A daemon split is planned once Quick Search and the tray need to run with the window closed.
Crate
Responsibility
core
Data model, SQLite index (rusqlite), migrations, shared types, FTS5 search
search
Full-text index (tantivy), ranking, filter execution. Deferred to v0.2; v0.1 uses SQLite FTS5
query
Natural-language parser, produces structured filters and chip descriptions
watcher
notify-based file watching, debouncing, incremental indexing, scan scheduling
media
Thumbnails, image metadata, PDF preview, OCR pipeline
dedupe
Size grouping, partial then full BLAKE3 hash, perceptual hash, hardlink and reflink awareness
cleanup
Rule engine: cleanup and organize rules, estimates, risk levels, dry-run previews
platform
Trash, origin metadata, D-Bus file manager calls, global shortcuts, autostart. One implementation per OS
daemon
Long-running background process owning the index (split after v0.1)
app
Thin Tauri adapter: typed commands, coalesced events, thumb:// protocol
ui
Svelte 5 desktop frontend
cli
Debug and power-user interface to the same core
Process model: v0.1 is one process. The window owns the SQLite connection and the watcher thread, and every capability has a CLI path. The daemon split (v0.2 or v0.3) moves the same `Store` behind a local socket without changing the schema or the UI payloads.
Data model (core tables): files (path, inode, size, mtime, duration, type, hash state, scan generation), sources (origin url, app, host), tags (file, tag, via user.xdg.tags), receipts (action, moved paths, undo state), organize rules (scope, action, risk, enabled), text content (OCR and extracted text), transcripts (whisper output), duplicate groups, cleanup findings, saved searches, user activity (opens, last open, for frecency).

Timestamp contract: `created` is filesystem birth time when the filesystem provides it, otherwise mtime; `mtime` is content modification; `added_at` is the discovered creation time (falling back to mtime) and never scan time. "Just added" and Recent use `added_at`. "Downloaded" is browser-history data and is separate from all three.

9. UI toolkit decision
Option
Pros
Cons
Tauri (webview)
Fastest path to a polished grid
Higher RAM and startup cost, conflicts with the efficiency goal
Slint
Native, light, good performance, cross-platform
Smaller ecosystem, check license terms for a paid product
iced
Pure Rust, active community
More custom work for virtualized grids and polish
GTK4/libadwaita
Best native look on GNOME
Weak on macOS and Windows
Decision (October 2026): Tauri 2 + Svelte 5. A like-for-like 50,000-item virtualized grid measured 100–180 MB PSS on Linux, against 30–60 MB for Slint. The extra memory buys a mature grid, text rendering and development speed; it fits the "quiet instrument" bar if the UI stays under 200 MB. The core and CLI are UI-free, so this is reversible: `app/src-tauri` and `app/ui` can be replaced by a Slint shell behind the same service calls without touching the schema. Reversal trigger: sustained PSS over 250 MB or a visible frame-budget miss on the reference machine. Plan a two-week port spike if triggered.

10. Performance and quality budgets
These are targets, not current measurements. v0.1 ships with the data structures and thumbnail pipeline that can meet them; benchmark fixtures and CI gates are part of v0.1 hardening.
• Window visible in under 200 ms when the index is ready.
• UI process PSS under 200 MB on the reference desktop with 50,000 items.
• Single-process v0.1 stays under 200 MB PSS; the daemon milestone targets about 50 MB idle.
• New files appear in the UI within 1 second.
• Grid scrolls at 60 fps with 50,000 items (virtualization, lazy decode, thumbnail cache).
• Initial index of 500,000 files in a few minutes on an SSD. Throttling, cancellation and resume are v0.1 hardening work.
• Search results in under 50 ms for filename and metadata queries.
• Target: indexer yields to the user (nice/ionice, battery pause) once the scan scheduler lands.
• Benchmarks become a CI gate before v0.2.
11. Safety and trust
• Trash only, per the freedesktop Trash spec. Never rm.
• Every destructive action shows what and why, plus the space gained.
• Only sanctioned mutations: files leave their place through Trash or an explicit organize rule, and both record a receipt with Undo.
• Receipts are durable: each batch records original paths, Trash paths and undo state, so Undo works after a restart.
• Skip network mounts, FUSE cloud mounts and cloud placeholder files by default. Dotfiles are indexed (they are user files); dot-directories and OS metadata are not.
• Never touch system directories. System-level findings are report-only with a copyable command.
• Hardlinks and reflinks are not counted as reclaimable space.
• No telemetry by default. If added later, it is opt-in and documented.
• The privacy dashboard lists every network call the app can make; there are only two, both opt-in.

12. Linux platform considerations
• Wayland: global hotkeys need the xdg-desktop-portal GlobalShortcuts API. Provide a fallback command (shelf --quick) users can bind in their desktop settings.
• Sandboxing: Flatpak limits filesystem access. Decide whether the primary build is native packages with Flatpak as secondary.
• Desktop environments: test GNOME, KDE Plasma, and one lightweight environment (XFCE or Sway) from day one.
• Filesystems: test ext4, btrfs, XFS and ZFS for hashing, inode and space accounting behavior.
• Download source tracking: read user.xdg.origin.url and user.xdg.referrer.url xattrs where present, then Firefox places.sqlite and Chromium History databases (copy before reading, since they are locked while the browser runs).
• Shared tags: read and write user.xdg.tags (comma-separated), so Nautilus and other Freedesktop apps see the same tags.
13. Roadmap and milestones
Assuming one to two full-time developers. Adjust to your team. Phases run in order; durations are estimates.
Phase
Scope
Estimate
Starter kit
Workspace, core schema, query parser (100 cases), watcher, media, platform, CLI, Tauri adapter, Svelte shell
Done
v0.1 hardening
Index 500k files, grid performance, Trash/Undo edge cases, packaging spike
3 weeks
v0.2 intelligence
OCR, duplicates, Clean Up review, treemap, tags, receipts, timeline
8–10 weeks
v0.3 everywhere
Quick Search, tray, source tracking, organize rules, KRunner, command palette, settings
6–8 weeks
v0.4 semantic
Search-by-example, semantic text, whisper.cpp, file stacks, macOS port
10–12 weeks

14. Business model (to decide)
• Option A: Paid, closed source (like Homy): one-time license, simple, funds development. Linux users are more skeptical of closed-source tools that touch all their files.
• Option B: Open core: the engine and CLI are open source and the polished GUI, OCR bundle and cleanup packs are paid.
• Option C: Fully open source with donations and sponsorship.
Given the trust demands of an app that indexes everything, I recommend considering Option B for Linux.
15. Risks and mitigations
Risk
Mitigation
Indexing large home directories hurts performance
Throttling, incremental scans, exclusion rules, benchmarks in CI
Wayland shortcut limitations
Portal support plus documented fallback
UI toolkit memory or polish misses the budget
UI-free core, measured reversal trigger, two-week Slint port plan
OCR quality or speed disappoints
Benchmark two engines, run OCR lazily in the background, make it optional
Fragmented Linux distribution
Pick one primary package format, automate others in CI
Scope creep into a full file manager
Hard rule: open in the user's file manager for file operations beyond Trash and copy
Auto-organize moves a file to the wrong place
Off by default, dry-run preview, narrow rule scope, receipts with Undo, never overwrite an existing path
Data-loss fear
Trash-only, previews, dry-run mode, extensive tests on cleanup rules
16. Success metrics
• Time from "I need that file" to opening it (target: under 5 seconds).
• Daemon idle memory and CPU, measured on real machines.
• Reclaimable space found per user on first scan.
• Zero reports of unintended data loss.
• Retention: weekly active users after 30 days.
• Organize adoption: share of active users with at least one enabled organize rule after 30 days (target: 15%).
17. Open decisions
1. Business model and license (section 14).
2. Primary distribution format (native packages versus Flatpak).
3. Product name and branding.
4. Whether the macOS and Windows ports share the same UI crate or get native shells.
5. Daemon split timing: v0.2 with Quick Search, or v0.3 with the tray.

18. Immediate next steps
1. Harden v0.1: benchmark a 500,000-file scan and a 50,000-item grid on ext4 and btrfs.
2. Run the OCR benchmark on 200 real screenshots.
3. Design duplicate grouping and tune the perceptual-hash threshold on real photo sets.
4. Add .deb and AppImage packaging to CI.
5. Instrument UI PSS and frame time; compare against the Tauri reversal trigger.
