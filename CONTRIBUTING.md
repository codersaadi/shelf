# Contributing to Shelf

Shelf is a Rust workspace with a Tauri 2 shell and a Svelte 5 frontend. The
core is UI-free: every capability should be reachable from `crates/cli`, not
only from the desktop app.

## Setup

```sh
cargo fetch
pnpm --dir app/ui install
```

Linux needs WebKitGTK 4.1, GTK 3, `ffmpeg` and `pdftoppm` for the full preview
experience. They are optional; missing tools fall back to type icons.

## Before opening a pull request

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
pnpm --dir app/ui check
pnpm --dir app/ui build
```

Keep the tree clean and include a focused test for parser, store, watcher,
platform or cleanup behavior changes.

## Conventions

- Rust edition 2024; `unsafe` is forbidden workspace-wide.
- Database access goes through `shelf_core::Store`; other crates do not run SQL.
- Errors use `thiserror` in libraries and `anyhow` only in the CLI and Tauri
  adapter.
- Parser changes require a case in `crates/query/tests/queries.rs`.
- Destructive UI actions keep the preflight confirmation and receipt-backed
  undo path.
- Comments explain intent, not syntax.

## Commit messages

Use a short imperative subject, optionally scoped:

```text
fix(icons): keep reverse-DNS icon names intact
feat(search): recognise bare extensions as type filters
docs: add contribution guide
```

Separate unrelated changes. Do not rewrite public history to simulate age.

## Architecture notes

`AGENTS.md` contains the task-oriented command map. `ui-ux.md` is the
interaction and token spec; `product-plan.md` is the architecture and roadmap.
