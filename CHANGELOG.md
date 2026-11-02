# Changelog

## Unreleased

### Added

- User-defined Locations with breadcrumbs, in-app folder navigation and
  persisted indexing roots.
- Duplicate detection by size and SHA-256 content hash, in the app and CLI.
- Smart `SKILL.md` recognition using the containing skill folder name, with the
  real filename shown alongside.
- Codebase detection from Git/manifest/build signals.
- Tag search (`tag:work`, `#work`) and Inspector tag editing.
- Year and size-range query phrases.
- Theme file, theme switching and text-size controls.
- Tracking controls, local open-count reset and Recent filtering.
- Permanent Trash purge, rename, terminal launch and system Trash entries.
- Application discovery outside document roots.

### Fixed

- Source metadata is read for all indexed files, with browser-host aliases and
  messaging-app filename inference.
- Frecency now uses a time-decayed open score.
- Cleanup reports measured system-cache sizes and truncation.
- Stale-row pruning keeps unreadable subtrees instead of freezing the whole
  root.
- Reverse-DNS app icons resolve correctly.
- Shared AppImage extraction directories no longer assign the wrong icon.
- Platform executables are no longer misclassified as applications.
