# Upstream cleaners study (2026-11)

**English** | [Polski](UPSTREAM-CLEANERS-2026-11.pl.md)

Research for XCleaner (Rust + GTK4/libadwaita, MIT). Phase 1 = `$HOME` only, phase 2 = `/` via sudo root broker.
All facts come from shallow clones of the upstream repos (HEAD as of 2026-11-22), crates.io API and official docs.
Statements marked **(observed)** were measured on the dev machine (CachyOS, btrfs, 8 threads, `$HOME` = 441 GiB, 1.14 M entries).
Statements marked **(unverified)** are assumptions.

Sources (main):

- BleachBit: https://github.com/bleachbit/bleachbit (GPL-3.0-or-later, pushed 2026-11-19)
- czkawka/krokiet: https://github.com/qarmin/czkawka (v12.0.2, 2026-09-09)
- kondo: https://github.com/tbillington/kondo (MIT)
- cargo-sweep: https://github.com/holmgr/cargo-sweep (MIT, README says unmaintained)
- npkill: https://github.com/voidcosmos/npkill (MIT, v1.0.0)
- Stacer: https://github.com/oguzhaninan/Stacer (GPL-3.0, last push 2024-02-10, last release v1.1.0 2019)
- KDE Sweeper: https://github.com/KDE/sweeper (mirror of invent.kde.org, LGPL-2.1-or-later per `LICENSES/`)
- dust: https://github.com/bootandy/dust (Apache-2.0), gdu: https://github.com/dundee/gdu (MIT)
- ncdu manual: https://dev.yorhel.nl/ncdu/man (2.9.x)
- dua-cli / dua-core: https://github.com/Byron/dua-cli
- jwalk: https://github.com/Byron/jwalk (archived)
- walkdir: https://github.com/BurntSushi/walkdir, ignore: https://github.com/BurntSushi/ripgrep/tree/master/crates/ignore
- Rust std (`DirEntry::metadata`): https://github.com/rust-lang/rust/blob/main/library/std/src/sys/fs/unix.rs
- CCleaner: https://www.ccleaner.com/legal/products-policy , https://support.ccleaner.com/articles/en_US/Master_Article/what-is-health-check , https://www.ccleaner.com/ccleaner/version-history
- btrfs subvolumes: https://btrfs.readthedocs.io/en/latest/Subvolumes.html

---

## 1. BleachBit

### 1.1 CleanerML format

Schema: `doc/cleaner_markup_language.xsd`
(https://github.com/bleachbit/bleachbit/blob/master/doc/cleaner_markup_language.xsd). Structure:

- `<cleaner id=.. os=..>`
  - `<label translate=..>` (app name), optional `<description>`
  - `<running type="exe|pathname" same_user="true|false" os=..>` (0..n): process name or lock-file glob
  - `<var name=..><value search="glob|winreg" os=..>` (0..n): expands `$$name$$` in paths (e.g. `$$profile$$` = every Firefox profile)
  - `<option id=.. os=..>` (0..n)
    - `<label>`, `<description>`
    - `<warning>` (0..1) — **an element, not an attribute**; shown when the user ticks the option
    - `<action command=.. search=.. path=.. ...>` (1..n)
  - `<localizations>` with nested `<path location= filter= directoryregex=>` and `<regexfilter prefix= postfix=>`
- `os` grammar: comma list with optional `!` negation, values `bsd|darwin|linux|macos|netbsd|openbsd|freebsd|unix|windows`.
- Booleans must be literally `true`/`false` (case-insensitive), no `1/0`.

`action@command` enumeration (full list from XSD):

| Group | Commands |
|---|---|
| File ops | `delete`, `truncate`, `shred` |
| Structured edit | `json`, `ini`, `sqlite.vacuum` |
| Browser DB | `chrome.*`, `mozilla.*`, `cookie` |
| Package mgr | `apt.*`, `dnf.*`, `yum.clean_all`, `pacman.cache`, `snap.disabled` |
| System | `journald.clean`, `process`, `process.terminate` |
| Windows | `winreg`, `win.shell.change.notify`, `office_registrymodifications` |
| macOS | `macos.orphaned_framework_versions` |

`action@search` values:

| Value | Meaning |
|---|---|
| `file` | single path |
| `glob` | glob expansion |
| `walk.files` | recursive, files only |
| `walk.all` | recursive, files + dirs |
| `walk.top` | recursive + top dir |
| `deep` | user-chosen roots, regex |

Other action attributes: `path`, `regex`/`nregex` (basename), `wholeregex`/`nwholeregex` (full path), `type="f|d"`,
`cmd`, `wait`, `address`, `name`, `parameter`, `section` (for ini/json), `same_user`, `skip_open` (deep only).

### 1.2 Linux-relevant cleaners in `cleaners/*.xml`

Extracted by parsing each XML and dropping `os="windows"`/`macos` parts. `!` = option has a `<warning>`. Commands in
brackets when not plain `delete`. `run=` is the running-process guard.

| Cleaner | Options (Linux) | Guard |
|---|---|---|
| Firefox / LibreWolf / Waterfox / Zen / Pale Moon | backup, cache, cookies, crash_reports, forms, passwords!, session, site_data, site_preferences, url_history, vacuum | exe + profile `lock` |
| Chromium / Chrome / Brave / Edge / Vivaldi / Opera / Helium | cache, cookies, crash_reports, form_history, history, search_engines, session, site_data, statistics, sync, passwords!, vacuum, site_preferences (Chrome also `ai`) | exe + `SingletonLock` |
| Epiphany | cache, cookies, passwords!, places!, site_data | exe + lock |
| Thunderbird | cache, cookies, crash_reports, index!, sessionjson, passwords!, vacuum | exe |
| Evolution / Geary | cache; Geary also attachments, sync_conflict, vacuum! | exe |
| Discord / Slack | cache, cookies, history, vacuum | exe |
| VS Code + forks | backup, cache, crash_reports, logs | code, codium, cursor, windsurf... |
| Claude (CLI) | backup, cache, file_history, logs, memory!, session!, shell_snapshots, tmp | claude |
| Android Studio | gradle_cache!, ide_cache, logs | studio |
| Docker Desktop | cache, logs, session | exe |
| Zoom | cache, logs, recordings | zoom |
| Thumbnails | cache (`~/.thumbnails/`, `$XDG_CACHE_HOME/thumbnails/`) | none |
| KDE | cache, tmp, recent_documents (`~/.kde*/cache-*`) | none |
| GNOME | run, search_history | none |
| Nautilus / Rhythmbox | history! | exe |
| Shells / REPLs | bash, zsh, fish, python, sqlite3, octave, vim, mc: history | none |
| Java | cache (`~/.java/deployment/cache/`, icedtea) | none |
| Wine / winetricks | tmp / temporary_files | none |
| X11 | debug_logs | none |
| GIMP | tmp, cache | gimp-2.x |
| LibreOffice / OpenOffice | history / cache, recent_documents | soffice.bin |
| VLC / Audacious / xine | mru / cache, log | vlc |
| Transmission | blocklists!, torrents, history | daemon/gtk/qt |
| pacman | cache (`paccache -rk0`) | pacman, paccache, `db.lck` |
| journald | clean (`journalctl --vacuum-size=1`) | journalctl |
| apt / dnf / yum / snap | clean, autoclean, autoremove!, disabled | apt/dnf |
| deepscan | angular!, backup!, ds_store!, node_modules!, venv!, pycache!, thumbs_db!, tmp!, vim_swap_user! | none |
| Legacy apps | aMSN, aMule, Beagle, d4x, emesene, gFTP, Miro, Pidgin, Skype, RealPlayer, Second Life, gl-117, Nexuiz, Tremulous, Warzone2100... | mostly none |

Built-in `System` cleaner (Python, `bleachbit/Cleaner.py` class `System`), Linux options:

- `cache`: everything under `~/.cache/` (`children_in_directory(..., same_device=True)`, keep-list checked)
- `tmp`: `/tmp`, `/var/tmp` — only regular files, owned by current uid, not keep-listed, not open by any process
- `trash`: `~/.Trash`, `$XDG_DATA_HOME/Trash`, snap trash, per-mount `.Trash/$uid` (only if sticky, not a link) and `.Trash-$uid`
- `recent_documents`: `~/.local/share/recently-used.xbel*` (shredded) + GTK RecentManager purge
- `rotated_logs`: `/var/log` files matching `\.(\d+|bz2|gz|lz4|xz|zst|old)$` or `-YYYYMMDD`, anchored so live logs survive
- `desktop_entry`: broken `.desktop` files in menu dirs; `localizations`: unwanted locale files
- `custom`: user-listed files/folders
- `memory` (swap + free RAM wipe, flagged experimental), `empty_space` (free-disk wipe), `dns_cache` flush

### 1.3 How BleachBit avoids deleting the wrong things

- **Hard floor** `_is_system_critical_posix()` (`FileUtilities.py`): `/`, `/proc`, `/sys`, `/run` (except `/run/media`) are
  never deleted even if the keep list is empty or a cleaner is buggy.
- **Keep list ("whitelist")** `whitelisted_posix()`: checks both the symlink name and its `realpath`; file and folder
  entries; prefix match for folders.
- **Protected paths** (`share/protected_path.xml`, schema `doc/protected_path.xsd`): not auto-kept, but warn when a user
  adds them to the custom delete list. Has `depth` (`0`, `1`, `2`, `any`) and `case`. Linux entries include `~`,
  `~/Documents`, `~/.config` (depth 1), `~/.local/share` (depth 0), `~/.mozilla`, `~/.ssh`, `~/.gnupg`, `~/.aws`, `~/.kube`,
  `~/.password-store`, `~/.docker`, `~/.nvm`, `~/.vscode`, `~/Pictures`, `~/Music`, `~/Videos`, `~/projects`, `~/backup(s)`,
  `~/Nextcloud`, `~/VMs`, and any `.git/.hg/.svn/.bzr`. The file header explicitly says `~/.cache`, `.cargo`, `.npm`,
  `.venv` do **not** belong there (regenerable).
- **Running-process guard** (`Process.py` `is_process_running(exename, require_same_user)`; `Cleaner.is_process_running()`):
  - process enumeration via psutil, fallback `/proc/*/exe`, fallback `ps aux`
  - matches `comm`, `basename(exe)` **and** `basename(argv[0])` (versioned binaries)
  - `pathname` guards glob a lock file; dangling lock symlinks (`IP:+PID` Gecko, `host-PID` Chromium) are resolved and the
    PID checked (`_lock_link_is_live`), same-user only (Flatpak PIDs may map to host kernel threads)
- **Open-file check** (`FileUtilities.open_files_linux()`, scans `/proc/*/fd`): used for `/tmp` and deep scan
  (`skip_open`); if the open-file list cannot be read, the file is **kept** (fail-safe).
- **Delete primitive** `FileUtilities.delete()` / `_delete()`:
  - `lstat`-based type check; symlinks and FIFOs are `unlink`ed, never followed
  - directories removed with `rmdir` only (contents deleted bottom-up first); `ENOTEMPTY` and `EBUSY` (mount point) are logged and skipped
  - special files (sockets, devices) refused
  - `dir_fd`-relative operations and `children_below(..., same_device=True, top=...)` so links planted under a tmp dir cannot redirect deletion
- **Deep scan** (`DeepScan.py`): `os.walk` over user-chosen roots, basename/full-path regex with negative regex
  (e.g. `node_modules` excluded under `~/.<hidden>/` and `AppData`); every deepscan option carries a warning.
- **Preview mode**: every command can be listed without deleting (GUI "Preview" button, CLI `--preview`).

### 1.4 Features XCleaner will NOT do

- Memory/swap wipe (`Memory.py` `wipe_memory`, `wipe_swap_linux`) — experimental, root, risky.
- Free-disk-space wipe (`empty_space`), `shred` overwrite — meaningless on SSD/CoW btrfs, slow.
- DNS flush, Windows registry, `process.terminate`.

### 1.5 License (GPL-3.0-or-later) — what an MIT repo may reuse

Not legal advice; conservative reading.

- **Can use:** facts — that app X stores cache in path Y, lock-file names, process names, the idea of a keep list,
  protected-path depth levels, preview mode, warnings. Re-verify each path on a real system and write our own rule text.
- **Can use:** general algorithms described in prose (e.g. "check lock symlink PID", "rmdir only after children").
- **Cannot:** copy `cleaners/*.xml`, `protected_path.xml`, regexes verbatim, Python code, translations (`po/`), or
  mechanically convert the XML into our TOML. That makes a derivative work requiring GPL.
- **Cannot:** ship BleachBit's XML as optional data inside the MIT binary. (A separate, user-installed importer that reads
  the user's own BleachBit install at runtime is a grey area; skip it.)
- Same applies to Stacer (GPL-3.0) and krokiet/cedinia apps (`LICENSE_GPL_APP`). `czkawka_core` is MIT (reusable with
  notice). Sweeper is LGPL-2.1+. kondo, npkill, cargo-sweep, gdu are MIT; dust is Apache-2.0.

---

## 2. Other cleaners and analyzers

### 2.1 Stacer (C++/Qt, GPL-3.0, effectively dead since 2019 release)

- System Cleaner (`stacer-core/Info/system_info.cpp`, `stacer/Managers/tool_manager.cpp`):
  - package cache (APT, pacman; note: YUM/DNF branch calls the pacman function — bug)
  - crash reports: all files in `/var/crash`
  - app logs: **all files** in `/var/log` (top level, including live logs)
  - app caches: every entry in `~/.cache`
  - trash
- No running-process checks, no keep list. Lesson: avoid "delete whole /var/log" style rules.

### 2.2 KDE Sweeper (C++/Qt, LGPL-2.1+)

- Privacy-only, 11 actions (`sweeper.cpp` `InitActions`): clipboard history, recent documents, run-command history,
  recent applications, thumbnails, cookies, favicons, web history, web cache, form completion, cookie policies.
- Single confirm dialog: "You are deleting data that is potentially valuable to you." Mostly KDE/Konqueror-specific.

### 2.3 czkawka / krokiet (Rust; core MIT, GUI apps GPL)

- Workspace: `czkawka_core` (lib), `czkawka_cli`, `czkawka_gui` (GTK4, legacy), `krokiet` (Slint, current GUI),
  `cedinia` (mobile). Core `rust-version = 1.94.1`, edition 2024.
- Tools (`czkawka_core/src/tools/`): duplicate, empty_files, empty_folder, big_file, temporary, similar_images,
  similar_videos, same_music, invalid_symlinks, broken_files, bad_extensions, bad_names, exif_remover, video_optimizer,
  disk_usage.
- **Walk** (`common/dir_traversal.rs`): no walker crate. Level-by-level BFS: `folders_to_check.into_par_iter().with_max_len(2)`
  over `std::fs::read_dir`, using `DirEntry::file_type()` (d_type, no extra stat) and stat only for files that pass filters.
  Stop flag checked per entry. Disk usage tool (`tools/disk_usage/scan/portable.rs`) is recursive `par_extend` per subdir,
  sizes = `metadata.len()` (apparent size), skips other filesystems.
- Crates: `rayon 1.12`, `crossbeam-channel`, `blake3`, `xxhash-rust` (xxh3), `crc32`, `trash 5.3` (optional `ashpd`
  portal trash for Flatpak), `file-id` (hardlink filter).
- **Temporary files**: extension list `#, thumbs.db, .bak, ~, .tmp, .temp, .ds_store, .crdownload, .part, .cache, .dmp,
  .download, .partial`.
- **Empty folders**: folder is empty if it contains only empty folders; deletion via
  `remove_folder_if_contains_only_empty_folders` (re-checks at delete time).
- **Invalid symlinks**: reports non-existent target and infinite recursion.
- **Duplicates**: size groups -> hardlink filter by file id -> prehash first `2 * PREHASHING_BUFFER_SIZE` bytes -> full hash
  (Blake3 default, Crc32, Xxh3); prehash + full-hash results cached on disk keyed by path/size/mtime.
- Default excludes: dirs `/proc /dev /sys /snap`; items `*/.git/*, */node_modules/*, */lost+found/*, */Trash/*, */.Trash-*/*`.
- Option "move to trash" instead of delete.

### 2.4 kondo (Rust, MIT) — project artifact cleaner

- Walk: `walkdir` (`follow_links`, `same_file_system` options); `ignore` used for last-modified scan. Option `--older`.
- Project markers -> artifact dirs (`kondo-lib/src/lib.rs`):

| Marker | Type | Artifacts |
|---|---|---|
| `Cargo.toml` | Cargo | `target`, `.xwin-cache` |
| `package.json` | Node | `node_modules`, `.angular` |
| `package.json` + RN | React Native | `android/build`, `ios/Pods`, `.expo`... |
| `*.csproj` + `Assembly-CSharp.csproj` | Unity | `Library`, `Temp`, `Obj`, `Logs`, `Build(s)` |
| `*.csproj`/`*.fsproj` | .NET | `bin`, `obj` |
| `project.godot` | Godot 4 | `.godot` |
| `stack.yaml` | Stack | `.stack-work` |
| `cabal.project` | Cabal | `dist-newstyle` |
| `build.sbt` | SBT | `target`, `project/target` |
| `pom.xml` | Maven | `target` |
| `build.gradle(.kts)` | Gradle | `build`, `.gradle` |
| `CMakeLists.txt` | CMake | `build`, `cmake-build-*` |
| `*.uproject` | Unreal | `Binaries`, `Saved`, `Intermediate`... |
| `*.ipynb` | Jupyter | `.ipynb_checkpoints` |
| `*.py` | Python | `__pycache__`, `.mypy_cache`, `.tox`... |
| `pixi.toml` | Pixi | `.pixi` |
| `composer.json` | Composer | `vendor` |
| `pubspec.yaml` | Dart/Flutter | `build`, `.dart_tool` |
| `mix.exs` | Elixir | `_build`, `.elixir_ls` |
| `Package.swift` | Swift | `.build`, `.swiftpm` |
| `build.zig` | Zig | `zig-cache`, `zig-out` |
| `turbo.json` | Turborepo | `.turbo` |
| `.terraform.lock.hcl` | Terraform | `.terraform` |
| `Podfile` | CocoaPods | `Pods` |

- Issues: #149 Unity misdetected as .NET; #55 `kondo ~` did nothing; #127 asks for dry-run.

### 2.5 cargo-sweep (Rust, MIT, unmaintained per README; suggests `cargo-clean-all`)

- Selective `target/` cleanup: `--time N` (older than N days), `--installed` (artifacts not built by installed toolchains,
  via rustc version hash matching Cargo's `StableSipHasher128`), `--toolchains`, `--maxsize 10GiB`, `--dry-run`, recursive.
- Idea for XCleaner: "keep artifacts newer than N days" and "shrink to size" knobs for `build-artifact` rules.

### 2.6 npkill (TypeScript, MIT, v1.0.0)

- Profiles (`src/core/constants/profiles.constants.ts`): node, python, java, android, dotnet, rust, elixir, haskell, scala,
  cpp, unity, unreal, godot, clojure, elm, php, dart, swift, zig, terraform.
- Target matchers: `{ target: 'target', nextTo: ['Cargo.toml'] }`, `{ target: 'obj', contains: ['project.assets.json'] }`,
  `{ target: 'Library', nextTo: ['Assets', 'ProjectSettings'] }` — more precise than kondo.
- `GLOBAL_IGNORE`: never descend `.git .svn .hg .Trash .nvm .rustup .pyenv .asdf .deno .volta .bun .vscode .idea
  node_modules __pycache__ .cache .venv venv` (still reported if they match a target).
- Sensitivity marking (`files.service.ts`): results under `~/.config`, `~/.local/share`, `~/.cache`, or any top-level hidden
  `~/.*` (except `.npm`, `.pnpm`) are flagged `isSensitive` with a reason. Worker threads for size calculation.

### 2.7 Disk usage tools: ncdu, gdu, dust, dua

| Tool | Parallel | Size basis | Hardlinks | Mounts |
|---|---|---|---|---|
| ncdu 2.9 | `-t N` (default 1) | blocks; `--apparent-size` | counted once | `-x`, `--exclude-kernfs` |
| gdu (Go) | 2×GOMAXPROCS slots | `st_blocks*512` | `(dev,ino)` if nlink>1 | `-x`; ignores `/proc /dev /sys /run` |
| dust (Rust) | rayon | blocks; `-s` apparent | `(ino,dev)` FxHashSet | `-x` via dev set |
| dua-core (Rust) | work-stealing pool | per caller | caller | caller |

- ncdu also has `--exclude-caches` (skip dirs with `CACHEDIR.TAG`, https://bford.info/cachedir/) and marks hardlinks `H`.
- dust: dedups on (inode, device) only for disk-usage mode; prefetches metadata to avoid double stat on FS without d_type.
- Lessons: report **allocated** size (`st_blocks * 512`) as "frees on disk", apparent size as secondary; dedup hardlinks by
  `(st_dev, st_ino)` when `st_nlink > 1`; never descend into other filesystems by default.
- **btrfs caveat (observed):** `/` (`@`) and `/home` (`@home`) on the same partition report different `st_dev` (34 vs 53).
  Per the btrfs docs each subvolume has its own inode namespace, so nested subvolumes (snapshots, container storage) will
  also look like "another filesystem" to `st_dev` checks (unverified for nested, unmounted subvolumes). Also reflinked /
  snapshot-shared extents are counted fully by `st_blocks`, so "space freed" can be overstated on btrfs (unverified;
  `compsize`/`btrfs fi du` give exact numbers).

---

## 3. CCleaner (Windows) mapped to Linux

Modules named in CCleaner's Products Policy and Health Check article: Health Check (Junk / Privacy / Speed / Update tabs),
Custom Clean, Smart Clean, Scheduled Cleaning, Registry Cleaner, Disk Analyzer, Drive Wiper, Uninstaller, Startup Manager,
Browser Plugin Manager, Software Updater, Driver Updater, Performance Optimizer, Duplicate Finder, Cloud Drive Cleaner.
(A community thread disputes whether Drive Wiper still exists in v7; not verifiable from official pages.)

| CCleaner feature | Linux equivalent | XCleaner |
|---|---|---|
| Temp files | `~/.cache`, `/tmp` own files | Phase 1 |
| Browser cache | profile `cache2`, `Cache` | Phase 1 |
| Cookies / history | browser SQLite DBs | Phase 1 opt-in |
| Recycle bin | XDG Trash | Phase 1 |
| Thumbnail cache | `~/.cache/thumbnails` | Phase 1 |
| App logs (user) | `~/.local/state`, app logs | Phase 1 |
| Crash dumps (user) | app crash dirs | Phase 1 |
| Crash dumps (system) | `coredumpctl`, `/var/lib/systemd/coredump` | Phase 2 |
| System logs | journald vacuum, rotated `/var/log` | Phase 2 |
| Package cache | `paccache`, AUR helper caches | Phase 2 (`~/.cache/yay` phase 1) |
| Orphan packages | `pacman -Qdtq` | Phase 2 report |
| Disk Analyzer | treemap / size tree | Phase 1 |
| Duplicate Finder | hash compare | Later (optional) |
| Health Check | one-click safe preset | Phase 1 (as "Quick clean") |
| Smart/Scheduled | systemd user timer | Later |
| Startup Manager | `~/.config/autostart`, user units | Never (out of scope) |
| Uninstaller | pacman/flatpak | Never (XUninstaller) |
| Browser plugins | extensions | Never |
| Registry Cleaner | n/a | Never |
| Drive Wiper | `shred`, free-space wipe | Never |
| Software/Driver Updater | pacman, fwupd | Never |
| Performance Optimizer | n/a | Never |
| Cloud Drive Cleaner | n/a | Never |

---

## 4. Fast scanning in Rust

### 4.1 Crate status (crates.io API, 2026-11-22)

| Crate | Version | MSRV | Status |
|---|---|---|---|
| walkdir | 2.5.1 (2026-11-21) | none set (edition 2018) | maintained |
| jwalk | 0.9.0 (2026-08-05) | none set | **archived**, "use dua-core" |
| dua-core | 6.1.0 (2026-11-03) | 1.88 | new (2026-08), maintained |
| ignore | 0.4.34 (2026-10-27) | 1.88 | maintained (ripgrep) |
| rayon | 1.12.0 | 1.80 | maintained |
| rustix | 1.1.6 | 1.65 | maintained |
| nix | 0.32.0 | 1.69 | maintained |
| libc | 0.2.191 | 1.65 | maintained |
| globset | 0.4.21 | 1.88 | maintained |
| trash | 5.4.0 | 1.87.0 | maintained |
| crossbeam-channel | 0.5.17 | 1.60 | maintained |

Local toolchain: rustc 1.100.0 (Arch), so all MSRVs are satisfied.

### 4.2 Comparison

| Approach | Parallel | Stat cost | Notes |
|---|---|---|---|
| walkdir | no | full-path `lstat` | simple, sorted, `same_file_system` |
| jwalk | rayon | full-path stat | archived; avoid |
| dua-core | work-stealing | std (dirfd) | new API, young |
| ignore WalkParallel | yes, `threads(n)` | full-path stat | gitignore logic unneeded |
| std::fs + rayon | yes | dirfd `statx` | full control, ~150 LOC |
| raw getdents64/statx | yes | minimal | unsafe, little gain |

Verified details:

- `walkdir::DirEntry::metadata()` on Unix = `fs::symlink_metadata(&self.path)` (path re-resolved from root each time).
- `ignore` parallel `DirEntry` metadata also uses `fs::metadata/symlink_metadata(&self.path)`. API:
  `WalkBuilder::new(p).standard_filters(false).same_file_system(true).threads(n).build_parallel().run(|| Box::new(|e| WalkState::Continue))`.
- `std::fs::DirEntry::metadata()` on Linux uses `dirfd(dir)` + name (`metadata_c`, `statx`/`fstatat` relative), i.e.
  already the "openat + statx" pattern without unsafe code. `DirEntry::file_type()` uses `d_type` (no syscall) when the
  FS provides it.
- dua-core: "Symbolic links are reported but never followed", `descend` predicate, `#![deny(unsafe_code)]`, on Linux uses
  std metadata.
- Raw `getdents64` (rustix `fs::Dir`/`RawDir`) only saves libc readdir buffering; stat calls dominate.

Rough data point **(observed, warm cache, 1.14 M entries in `$HOME`)**:

| Command | Time |
|---|---|
| `du -sx ~` (single, stats all) | 3.7 s |
| `fd` 8 threads (no stat) | 0.57 s |
| `fd -j1` (no stat) | 2.6 s |

So parallelism gives ~4.5× on readdir with 8 threads; cold-cache NVMe numbers still need measuring.

### 4.3 Recommendation

- Write a small in-house walker: **`std::fs::read_dir` + `DirEntry::file_type()` + `DirEntry::metadata()` (dirfd-relative
  statx) on a rayon pool**, recursive `par_iter` over subdirectories (dust/czkawka pattern). Dependencies: `rayon` only.
- Per entry use `std::os::unix::fs::MetadataExt`: `blocks()*512` (allocated), `len()` (apparent), `dev()`, `ino()`, `nlink()`.
- Hardlink dedup: global `HashSet<(dev, ino)>` only for files with `nlink > 1` (cheap, rare).
- Do not cross `st_dev` of the scan root; record skipped mount points (and btrfs subvolumes) so the UI can show them.
- Never follow symlinks; count the link itself only.
- "~50 known directories": run them as independent rayon tasks on the same pool (`par_iter` over roots); sizes are
  independent and many dirs are tiny, so one shared pool beats per-root threads. Early-exit roots that don't exist.
- "Full `$HOME` breakdown": same walker, aggregate bytes per directory bottom-up, keep only top-N children per node
  in memory (avoid a 1 M-node tree), stream progress via a `crossbeam-channel`/`async-channel` to the GTK main loop
  (`glib::spawn_future_local`), throttled to ~10 Hz. Cancellation via `AtomicBool`.
- Reuse: known-dir results come from the same pass when the breakdown runs (match paths during the walk).
- Re-evaluate `dua-core` once it has a few releases; keep `ignore`/`globset` only for rule glob matching (globset is
  fine; walk engine not needed).

---

## 5. Recommendations for XCleaner

### 5.1 Rule format (own TOML, embedded via `include_str!`)

Write rules from scratch, verified on real systems; never transcribe BleachBit XML.

```toml
[[rule]]
id          = "firefox.cache"
app         = "Firefox"
category    = "browser"          # browser | dev | system | app | trash | logs
kind        = "cache"            # cache | log | dump | trash | build-artifact | history | thumbnail
regenerates = true               # app rebuilds it automatically
risk        = "low"              # low | medium | high (high = off by default + warning)
paths       = ["$XDG_CACHE_HOME/mozilla/firefox/*/cache2",
               "~/.var/app/org.mozilla.firefox/cache/mozilla/firefox/*/cache2"]
match       = "contents"         # contents (keep root dir) | self | files-only
older_than  = "0d"               # optional age filter (mtime)
exclude     = ["**/.parentlock"]
warning     = ""                 # shown when ticked; required for risk=high

[rule.guard]
processes   = ["firefox", "firefox-bin"]       # comm / exe basename / argv0
lock_files  = ["~/.mozilla/firefox/*/lock"]    # dangling symlink -> check PID
on_running  = "skip"             # skip | warn
```

Build-artifact rules (npkill-style precision, not plain directory names):

```toml
[[project]]
id        = "cargo"
marker    = { next_to = ["Cargo.toml"] }
targets   = [{ dir = "target", contains_any = ["CACHEDIR.TAG", ".rustc_info.json"] }]
kind      = "build-artifact"
regenerates = true
risk      = "low"
min_age   = "14d"                # skip recently-built projects by default
```

Engine rules:

- Expand `~`, `$XDG_*` (with spec defaults), Flatpak `~/.var/app/<id>/cache`, Snap `~/snap/<app>/common/.cache`.
- Every resolved target must be inside `$HOME` after `realpath` of its **parent** (phase 1 hard check).
- Hard deny list (compiled in, not configurable): `~` itself, `~/.ssh`, `~/.gnupg`, `~/.password-store`, `~/.local/share/keyrings`,
  XDG user dirs (Documents, Pictures, Music, Videos, Desktop, Downloads — Downloads only via explicit rule), any `.git`,
  `/`, `/proc`, `/sys`, `/run`, `/dev`.
- User keep list (paths + globs), checked on both link path and resolved path.
- CACHEDIR.TAG-aware: a dir with a valid tag is a cache candidate (Cargo `target`, ccache etc.).
- Respect `.nobackup`? No — not a deletion signal.

### 5.2 Deletion semantics (lessons from BleachBit `FileUtilities.delete`)

- Scan once, store `(dev, ino, path, size)`; at delete time re-`lstat` and abort the item if inode/dev changed (TOCTOU).
- Delete with `openat`/`unlinkat`-relative ops on a dirfd opened with `O_NOFOLLOW|O_DIRECTORY` (rustix `fs::openat`,
  `unlinkat`, `AT_REMOVEDIR`); never follow symlinks at **any** level including the top (BleachBit #1889: a link in the
  Recycle Bin's first level deleted `C:\Windows`).
- Do not cross `st_dev` while deleting; skip mount points (`EBUSY`) silently.
- Only remove dirs bottom-up after children; keep the rule's root directory (`match = "contents"`) so apps with fixed
  permissions/ACLs don't break.
- Skip files open by any process (scan `/proc/*/fd` once per run) for `tmp`/`log` kinds; fail-safe = keep.
- Optional "Move to Trash" for medium/high risk (gio trash via `gio::File::trash`, already in GTK deps); permanent
  delete for regenerable caches.
- Log every deleted path to `$XDG_STATE_HOME/xcleaner/log/<timestamp>.jsonl` (user can audit).

### 5.3 UX worth copying

- Preview before delete, always (BleachBit Preview, kondo/cargo-sweep `--dry-run`); "Clean" button disabled until a scan.
- One "Quick clean" preset = only `risk=low, regenerates=true` rules (CCleaner Health Check idea, but transparent and editable).
- Per-app grouping with a single checkbox per app and expandable options; **one entry per app** (no duplicate Firefox
  categories — BleachBit #1737).
- Badges: "Regenerates", "Running — skipped", "May log you out", "Warning" (BleachBit `<warning>`, npkill `isSensitive` reason).
- Show allocated size freed and item count; show skipped mounts/subvolumes.
- Disk breakdown view (ncdu/gdu): sortable list with "open in Files" and "add to keep list".
- Project artifacts page (kondo/npkill): last modified, project type icon, "older than N days" filter.
- `adw::Toast` with summary + "Open log"; `adw::AlertDialog` confirm listing high-risk items explicitly.

### 5.4 Pitfalls from issue trackers

| Issue | What happened | Lesson |
|---|---|---|
| BleachBit #1737 | two Firefox entries, 200 tabs lost | one entry per app |
| BleachBit #1838 | Chrome profiles disappeared | never touch profile roots |
| BleachBit #2321 | deep scan broke ComfyUI venv | venv = high risk, opt-in |
| BleachBit #1616 | valid Wine/MenuLibre `.desktop` removed | no "broken" heuristics phase 1 |
| BleachBit #1798 | thumbnails also in System→cache | non-overlapping rules |
| BleachBit #1889 | trash link deleted `C:\Windows` | never follow links |
| BleachBit #1433 | `.exe` files deleted (Windows) | narrow globs, audit log |
| BleachBit #2392 | links to other drives question | same-device + no-follow |
| kondo #149 | Unity seen as .NET | marker + contents check |
| kondo #55 | `kondo ~` did nothing | test `$HOME` as root |
| Stacer source | deletes all `/var/log` files | rotated-only in phase 2 |

More specific traps:

- `~/.cache` wholesale delete (BleachBit `System.cache`, Stacer) also removes things users care about: Flatpak/Steam
  shader caches (slow first launch), `~/.cache/yay` PKGBUILD clones, KDE/GNOME search indexes, Mesa shader cache. Offer
  per-subdir rules instead of one "everything in ~/.cache" option; keep an "Other ~/.cache" option at medium risk.
- Browser `session`, `cookies`, `site_data`, `passwords`, `history`: data loss, not "junk". Off by default, `risk=high`.
- VS Code `User/History` (BleachBit "backup") is local file history — not junk.
- pacman `paccache -rk0` (BleachBit) removes all cached packages and prevents downgrades; phase 2 should default to
  `paccache -rk2` style. `journalctl --vacuum-size=1` deletes all journals; prefer `--vacuum-time=2weeks`.
- Steam: never touch `steamapps/` (games, saves); only `shadercache`/`depotcache` optionally, with warning.
- Dev caches (`~/.cargo/registry`, `~/.npm/_cacache`, `~/.cache/pip`, `~/.gradle/caches`, Go `~/.cache/go-build`) are
  regenerable but cost bandwidth/time: `risk=medium`, show re-download size.
- Flatpak/Snap duplicates of app paths (`~/.var/app`, `~/snap`) must be part of the same app entry.
- btrfs snapshots: deleting files may free nothing if snapshots keep the extents; say "space may not be freed if
  snapshots exist" when Snapper/Timeshift config is detected (unverified detection method).

### 5.5 Summary of choices

1. Own TOML rules embedded at build time; fields: id, app, category, kind, regenerates, risk, paths, match, guard, warning.
2. Facts may be learned from BleachBit; never copy its XML/regex/code (GPL-3.0).
3. Walk engine: std `read_dir` + rayon, `d_type` first, dirfd-relative `statx`, `st_blocks*512`, `(dev,ino)` dedup, no mount
   crossing, no symlink following.
4. Deletion: dirfd-relative, `O_NOFOLLOW`, re-validate inode before delete, bottom-up, keep rule roots, skip open files.
5. Running-app guard: process name (comm/exe/argv0) + lock-file PID check; default "skip".
6. Hard deny list + user keep list + `$HOME` containment check after realpath.
7. Default preset = low-risk regenerable caches only; browser data, venvs, dev caches opt-in with warnings.
8. Build artifacts with npkill-style `next_to`/`contains` markers, CACHEDIR.TAG, age filter.
9. Preview mandatory, audit log, optional move-to-trash.
10. Out of scope: memory/swap/free-space wipe, shredding, registry, startup manager, updaters.
