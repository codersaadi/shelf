# AGENTS.md

Shelf: local-first file intelligence for Linux. Rust workspace + Tauri 2 + Svelte 5 UI.
UI-free core: every capability must be reachable from `crates/cli`, never only from the Tauri layer.

## Commands

| Task | Command |
|---|---|
| Build (all crates) | `cargo build` |
| Run UI in dev | `pnpm --dir app/ui tauri dev` |
| Run the app against the built UI | `pnpm --dir app/ui build && cargo run -p shelf-app --features custom-protocol` |
| Build release bundle | `pnpm --dir app/ui tauri build` |
| All Rust tests | `cargo test --workspace` |
| One crate's tests | `cargo test -p shelf-query` |
| Lint | `cargo clippy --workspace --all-targets -- -D warnings` |
| Format | `cargo fmt --all` |
| Frontend build | `pnpm --dir app/ui build` |
| Frontend typecheck | `pnpm --dir app/ui check` |
| Index a directory from CLI | `cargo run -p shelf-cli -- index ~/Downloads` |
| Run a query from CLI | `cargo run -p shelf-cli -- query "pdf in august over 5 MB"` |
| Explain a parse | `cargo run -p shelf-cli -- explain "screenshots last week"` |
| Open a file | `cargo run -p shelf-cli -- open ~/Downloads/invoice.pdf` |
| List handlers | `cargo run -p shelf-cli -- apps ~/Downloads/clip.mp4` |
| Resolve an app icon | `cargo run -p shelf-cli -- app-icon ~/Downloads/App.AppImage` |
| Copy or move files | `cargo run -p shelf-cli -- copy <files…> <dest>` / `move <files…> <dest>` |
| Open a terminal | `cargo run -p shelf-cli -- terminal <file>` |
| Set a default app | `cargo run -p shelf-cli -- default <file> <app.desktop>` |
| Render a thumbnail | `cargo run -p shelf-cli -- thumb ~/Downloads/clip.mp4 --out preview.png` |
| Read a text/Markdown card snippet | `cargo run -p shelf-cli -- text ~/Downloads/notes.md --html` |
| Reveal in file manager | `cargo run -p shelf-cli -- reveal ~/Downloads/invoice.pdf` |
| Cleanup findings | `cargo run -p shelf-cli -- cleanup` |
| Trash receipts / undo | `cargo run -p shelf-cli -- receipts` / `undo <id>` |
| Inspect one path | `cargo run -p shelf-cli -- inspect ~/Downloads/invoice.pdf` |

First checkout: `cargo fetch && pnpm --dir app/ui install`.

## Layout

- `crates/core` — SQLite index (rusqlite, WAL), migrations, types, FTS5 search, `Store`. Owns the schema; other crates never run SQL.
- `crates/query` — natural-language parser → `Filter` IR + chip descriptions. Pure, no I/O, table-tested (100 cases in `tests/queries.rs`).
- `crates/watcher` — notify debouncer + full scan; exclusion rules; writes through `shelf_core::Store`.
- `crates/media` — kind detection, screenshot detection, freedesktop thumbnail cache lookup, thumbnail generation.
- `crates/platform` — freedesktop Trash with exact receipt restore, origin/tag xattrs, native app-icon lookup (desktop entries, icon themes, AppImage siblings), D-Bus FileManager1 and OpenURI. One implementation per OS; Linux only for now.
- `crates/cleanup` — reviewable cleanup findings and copy; no root commands or installed-package guesses.
- `crates/cli` — `shelf` binary: index, query, explain, stats, trash, cleanup, receipts, undo, inspect, tags, saved searches, open, apps, reveal, thumb, text. Mirrors what the UI can do.
- `app/src-tauri` — thin Tauri adapter: typed commands, coalesced watcher events, `thumb://` protocol, startup scan. No business logic.
- `app/ui` — Svelte 5 + TS + Vite. `src/lib/stores.svelte.ts` owns view state; components live in `src/lib/components`; tokens in `src/tokens.css` mirror `ui-ux.md` §2; do not invent colors or spacing. The filled icon set is copied from `common-icons/` into `public/icons/` and tinted with CSS masks by `CommonIcon.svelte`.
- `ui-ux.md` is the authoritative UI spec. `product-plan.md` is the product/architecture source.

## Local development

| Variable | Effect |
|---|---|
| `SHELF_DB` | Index path (default `$XDG_DATA_HOME/shelf/index.db`). |
| `SHELF_INDEX_ROOTS` | Colon-separated scan/watch roots instead of `$HOME`. |
| `SHELF_AUTOINDEX=0` | Skip the automatic first-run scan. |

Use a scratch database and roots for demos so a real home directory is not indexed.

## Conventions

- Edition 2024, MSRV 1.90. `unsafe` is forbidden workspace-wide.
- Errors: `thiserror` in library crates, `anyhow` only in `cli` and `app/src-tauri`.
- Database access only through `shelf_core::Store`. Full scans use a scan generation: every seen row is stamped, and unseen old rows under the root are pruned after a clean walk.
- Destructive UI actions go through the preflight dialog; Trash restore matches the exact receipt entry, not just the original path.
- IPC payloads are keyset-paginated; never send unbounded lists to the UI.
- Tauri command arguments are camelCase on the JS side (`receiptId`), snake_case in Rust.
- Comments only where intent is non-obvious; no narrating comments.
- Test naming: `function_under_test_condition_expected` in `query`'s table suites.
- New parser behavior requires a row in `crates/query/tests/queries.rs` (target: 100 cases).

## Definition of done for a change

`cargo fmt --all --check`, clippy with `-D warnings`, `cargo test --workspace`, `pnpm --dir app/ui check`, and `pnpm --dir app/ui build` all pass. UI-visible changes also need the screenshot set in `ui-ux.md` §17 updated if a covered surface changed.
