# XCleaner

Fast, lightweight and precise disk cleaner for Linux (Arch / CachyOS first), written in Rust + GTK4/libadwaita.

Status: early development (v0.0.x). Scope today: the user's home directory. System-wide (`/`) cleaning comes later.

## Goals
- **Fast** – parallel, metadata-only scanning; no shelling out.
- **Light** – small binary, no background services.
- **Precise** – every rule names exact paths and explains why they are junk;
  nothing is deleted that the rule cannot prove is a cache, log, dump or leftover.
  Configuration and personal data are never offered by default.

License: MIT

## Build & run
```sh
cargo build --release
./target/release/xcleaner          # GUI
./target/release/xcleaner --scan   # read-only report in the terminal
```
Requires GTK ≥ 4.12 and libadwaita ≥ 1.6.

AppImage: `./packaging/build-appimage.sh` → `dist/XCleaner-<version>-x86_64.AppImage`.

## Layout
- `crates/xcleaner-core` – rules, parallel scanner, descriptor-based safe deletion (no GTK).
- `crates/xcleaner` – GTK4/libadwaita GUI and CLI report.
- `docs/ROADMAP.md` – task list (small → large); `docs/research/` – research reports.
