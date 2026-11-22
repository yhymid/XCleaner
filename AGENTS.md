# Instructions for AI coding agents (XCleaner)

**English** | [Polski](AGENTS.pl.md)

Read README, docs/ROADMAP.md and docs/research/ first.

## Owner decisions (binding)
- UI in English (default) and Polish, switch EN/PL in the window (owner 2026-11-22).
  Every user-facing text has both: `tr!("en", "pl")`, `trf!("en {x}", "pl {x}")`,
  `i18n::T::new(en, pl)` in static tables, `i18n::count(n, [en1, enN], [pl1, pl2-4, pl5+])`.
  Never add a text in one language only. Code and commits are English.
- Docs come in pairs: `X.md` (English) and `X.pl.md` (Polish), linked to each other at the
  top. Change both together.
- Precise over aggressive: every rule names exact paths. Never offer a whole top-level
  dir (`~/.config`, `~/.local`, `~/.cache`, Steam root) — enforced by `protect.rs`.
- Owner 2026-11-22: `~/.cache` can be cleaned practically whole — every top-level entry is
  its own preselected item, EXCEPT mostly-media dirs (auto re-rated to Review, real example
  `~/.cache/music-opus` = 8.5 GB music) and known stateful ones (`CACHE_STATEFUL`).
- Owner 2026-11-22: leftovers of uninstalled apps in `~/.config` / `~/.local` must be found
  (uninstallers always leave remnants) — design in docs/research/LEFTOVERS-DETECTION-2026-11.md.
- Only `Risk::Safe` items are preselected, plus (owner 2026-11-22) leftovers with no
  personal-data markers (`leftovers::PERSONAL_MARKERS`/`PERSONAL_EXT`). Trash, history,
  personal data, prefixes and leftover profiles are never preselected.
- Owner 2026-11-22: use every evidence source and measure its effect on real data before
  deciding (e.g. `pacman -Fl` aliases), instead of guessing. New mechanisms keep a switch
  back (`--no-files-db`).
- Deletion is permanent (no quarantine); the confirmation dialog says so.
- The GUI never runs as root (it refuses). System tasks run in a terminal through the own
  root helper `xcleaner --root-clean` (sudo → doas → su, no polkit); root gets only task
  names and works out the targets itself.
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
