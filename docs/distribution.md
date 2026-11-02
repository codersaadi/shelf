# Distribution

## Supported platform today

Shelf targets **Linux only**, built on GTK 3/WebKitGTK 4.1 through Tauri 2.

- Desktop: X11 and Wayland sessions through GTK.
- Architecture: the host architecture of the build; x86_64 is the primary
  target, aarch64 should build but is not continuously tested.
- Not supported: Windows and macOS. The platform layer uses freedesktop Trash,
  Linux xattrs, D-Bus FileManager1/OpenURI, `xdg-open`, and Linux terminal
  discovery.

## Package formats

Configured in `app/src-tauri/tauri.conf.json`:

- **AppImage** — portable, no install, recommended for early users.
- **.deb** — Debian/Ubuntu and derivatives.
- **.rpm** — Fedora/RHEL and derivatives.

A Flatpak is the next preferred channel because it can pin WebKitGTK and work
across distributions. Snap and AUR packages are optional downstream work.

## Release checklist

1. Update the version in `Cargo.toml`, `app/src-tauri/tauri.conf.json` and
   `app/ui/package.json`.
2. Update `CHANGELOG.md`.
3. Run the full check matrix from `CONTRIBUTING.md`.
4. Build on a clean Linux host and test on X11 and Wayland.
5. Tag `vX.Y.Z`; the release workflow builds the three bundles and opens a
   draft GitHub release.
6. Download the draft artifacts, install each format, and verify:
   first-run scan, search, previews, Open with, Trash/Restore, permanent purge,
   Locations, duplicates and theme reload.
7. Publish checksums and, for .deb/.rpm, sign repository metadata when a
   repository exists.

## Signing and updates

- No auto-updater is enabled yet; releases are manual downloads.
- AppImage signing and GPG-signed `.deb`/`.rpm` repositories should be added
  before calling releases production-ready.
- Secrets belong in CI secret storage, never in the repository.

## Licensing and assets

Shelf is dual-licensed MIT OR Apache-2.0. Bundled Inter fonts are OFL-1.1.
The icon-set provenance is recorded as unresolved in `THIRD_PARTY_NOTICES.md`;
resolve it before shipping binaries. Audit dependency licenses before release.
