# Instructions for AI coding agents (XCleaner)

Read README, docs/ROADMAP.md and docs/research/ first.

## Owner decisions (binding)
- Polish UI; code, commits and docs may be English.
- Precise over aggressive: every rule names exact paths. Never offer a whole top-level
  dir (`~/.config`, `~/.local`, `~/.cache`, Steam root). A path in `~/.cache` is NOT
  automatically junk (real example: `~/.cache/music-opus` = 8.5 GB of music).
- Only `Risk::Safe` items are preselected. Trash, history, personal data, prefixes and
  leftover profiles are never preselected.
- Deletion is permanent (no quarantine); the confirmation dialog says so.
- Phase 1 is HOME-only and refuses to run as root. Phase 2 (`/`) will reuse the
  XUninstaller root broker; the GUI never runs as root.
- No GPL code or data: BleachBit/Stacer cleaners must not be copied (facts verified on a
  real system are fine). See docs/research/UPSTREAM-CLEANERS-2026-11.md §1.5.

## Safety rules (mandatory)
- All deletion goes through `xcleaner_core::fsops::Home` (openat2 beneath HOME, no
  symlink following, no mount crossing, owner check). No path-string `rm -rf`, no shell.
- Rules with a running app get a process guard and/or a SingletonLock check.
- State files inside cache dirs go into `Action::ClearDirKeep` (e.g. yay `vcs.json`).

## Workflow
Before each commit: `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace`. Verify on the real machine with `xcleaner --scan` (read-only).
