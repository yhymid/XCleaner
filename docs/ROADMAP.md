# XCleaner — roadmap

**English** | [Polski](ROADMAP.pl.md)

## Version plan (owner decision 2026-11-22)
- **v0.1.0 — First Sweep:** works on the owner's hardware (Steam Deck OLED, CachyOS,
  Hyprland): HOME + system through sudo in a terminal. Release: AppImage and a pacman package.
- **v0.2.0:** all distributions, including the apt, dnf, zypper and flatpak/snap package managers,
  other desktops and directory layouts. Research by agents is mandatory, because this
  cannot be verified on one machine.
- **v0.3.0:** tests in virtual machines: Debian, Fedora, Arch and popular distributions
  from https://distrowatch.com/.
- **v0.4.0:** to be decided later together with the agents.

Every v0.X.0 version goes to a GitHub Release:
- title `<version> - <Name> Update`;
- Download / Install / Launch sections;
- changelog from `CHANGELOG.md`;
- files: AppImage + `.sha256` and a `.pkg.tar.zst` package. AUR (yay) comes once the repo is public.

Tasks are ordered from the smallest to the largest. Every stage ends with
a working build that can be tested on the Deck.

Research: [docs/research/](research/) (every report has a `.pl.md` version). HOME-JUNK-CATALOG
holds measurements from this machine, and UPSTREAM-CLEANERS describes BleachBit, czkawka,
kondo, CCleaner and others.

## v0.0.1 — skeleton (done)
- [x] Rust workspace: `xcleaner-core` (no GTK) + `xcleaner` (GTK4/libadwaita).
- [x] Safe deletion through descriptors:
  - openat2 BENEATH/NO_SYMLINKS/NO_XDEV;
  - no following symlinks, no crossing into other filesystems;
  - files of other owners are skipped.
- [x] 16 rules: thumbnails, fontconfig, GStreamer, Mesa shaders, browser caches,
  yay/paru (without `vcs.json`), cargo, npm, pnpm, pip/uv, go, trash, recently-used.
- [x] Automatic detection of Electron/Chromium apps in `~/.config` (cache directories only).
- [x] Directories in `~/.cache` with `CACHEDIR.TAG`.
- [x] Block while a program runs: /proc and SingletonLock with a PID.
- [x] GUI: scan → list with categories → one confirmation → clean.
- [x] `xcleaner --scan`: report in the terminal, deletes nothing.

## v0.0.2 — small rules (done)
- [x] Steam:
  - shaders per game, with the name from appmanifest (CS2 10.9 GB, Deadlock 6.5 GB);
  - orphaned shaders of uninstalled games (selected);
  - logs and dumps, store cache.
- [x] Launcher caches: `PrismLauncher/cache` (2.7 GB), `bedrock-on-linux/cache` (843 MB).
- [x] Discord: old `app-1.0.*` and `1.0.*` after auto-update (the newest stays).
- [x] Electron logs (`logs/`), Crashpad, `Service Worker/CacheStorage`.
- [x] The rest of `~/.cache`: every entry separate and selected.
  - Directories with music or videos are automatically marked "review first" (`music-opus`).
- [x] Hard "never" list (`protect.rs`), checked during discovery and before deletion,
  guarded by tests (`.ssh`, keyrings, Steam `userdata`/`compatdata`, `.local/bin`,
  top-level HOME directories).
- [x] Dead `.desktop` shortcuts: missing program, missing Flatpak, missing Wine prefix,
  broken link. On this machine: 7, among them osu-wine, BeatSaberModManager, old XUninstaller.
- [x] `xcleaner --scan --details`: paths and reasons.

## v0.0.3 — leftovers of uninstalled apps (done)
- [x] `leftovers.rs`: `~/.config`, `~/.local/share`, `~/.local/state`, `~/.var/app`.
  An item makes the list only when 3 facts agree:
  - the name matches a package removed according to pacman.log;
  - nothing installed uses that name;
  - the directory did not appear after the package was removed.
- [x] Evidence: the pacman database (names, provides, binaries, .desktop, /opt, /etc/xdg), PATH,
  .desktop files (dead ones skipped), Flatpak, processes.
- [x] Tiers: "certain" (directory created 0–30 min after installation) and "almost certain".
  Never preselected. A warning when personal data is inside.
- [x] The list of shared names (gtk, dconf, pulse, systemd…) always wins. psd copies `*-back-ovfs.base`.
- [x] On this machine: 20 items, about 9.8 GB. Matches the manual analysis, and the false
  alarm "Discord Key Storage" is rejected.
- [x] Aliases from `pacman -Fl <removed>` (binaries, .desktop, /opt, /etc/xdg), cached
  in `~/.cache/xcleaner` (first scan 3.9 s, later ones 0.5 s). Way back: `--no-files-db`.
  - On this machine it gave 0 new hits, but it catches renames on other systems.
- [x] Owner decision: leftovers without personal data are preselected ("low risk").
  - Data markers: browser profiles, saves, screenshots, game instances,
    .exe/.sav files, AI chat history.
  - Unselected on this machine: BSManager (game and maps), shadPS4, zed, browser profiles.
- [x] Hard links: the shared size is reported (less will actually be freed).
- [ ] Later:
  - matches by the first part of the name (evolution-data-server → evolution);
  - XUninstaller records, dot-dirs `~/.<name>`.

## v0.0.4 — medium (done)
- [x] Details window (⋮): every path with its size, opening in the file manager,
  a "Keep this — hide it for good" button.
- [x] Keep list `~/.config/xcleaner/keep.txt` (editable by hand), a
  "Restore hidden (N)" button.
- [x] "Events" log: scans, cleans, errors, copy and save as .md
  (the HOME path replaced with `~`). Permanent history in `~/.local/state/xcleaner/history.log`.
- [x] Flatpak: `~/.var/app/<id>/cache` of installed apps.
- [x] "Possible" leftovers (first part of the name); AppImages count as installed.

## v0.0.5 — large (done)
- [x] Project build artifacts (`artifacts.rs`), in the style of npkill:
  - a directory is an artifact only next to a project file (Cargo.toml→target,
    package.json→node_modules, CMake `build` only with CMakeCache.txt, about 30 types);
  - search the whole HOME without hidden directories, symlinks and other filesystems;
  - selected if the project was not built for 30 days. On this machine: 12 GB in 4 projects.
- [x] "What takes space" (disk icon, `xcleaner --usage [dir]`):
  - drilling down, share bars, opening in the file manager;
  - the whole HOME (442 GB) in 3.0 s, `du` about 3.7 s.
- [x] Fix: measuring hung on FIFO queues (`~/.steam/steam.pipe`).
- [x] Fix: `| head` in the CLI no longer panics or dumps core.
- [ ] Moved: rules in embedded TOML. Needs a new dependency (`toml`);
  do it together with user rules.

## v0.1.0 — First Sweep (release)
- [x] AppImage: `packaging/build-appimage.sh`.
  - linuxdeploy with a pinned SHA-256 and the GTK plugin at a pinned commit, as in XU.
  - 48 MB; tested on the Deck (CLI and window, Wayland).
- [x] SVG icon and .desktop file (PL/EN).
- [x] pacman package: `packaging/arch/PKGBUILD` + `packaging/build-pkg.sh` (484 KB).
- [x] Phase 2 in v0.1.0 (owner decision): coredumps, `paccache`, modules of old kernels,
  through `sudo` in a terminal (no polkit, like XU).
- [ ] Owner's test on the Deck: a real clean from the window, including the sudo password.
- [ ] Tests on a fresh Arch/CachyOS move to v0.3.0.

## Phase 2 — system (`/`, through sudo): part of it went into v0.1.0
- [x] `root.rs`: sudo in a terminal. Root gets only task names and works out the targets itself.
  The GUI never runs as root.
- [x] pacman cache: `paccache -rk2` + `-ruk0`. `-rk0` is a separate item, never preselected.
- [x] Coredumps and modules of removed kernels.
- [ ] journald: `--vacuum-time`, never "delete everything". Only rotated logs in `/var/log`.
- [ ] For v0.2.0: `/var/tmp`, unused Flatpak runtimes (`flatpak uninstall --unused`),
  journald by time, rotated logs in `/var/log`, coredumps from other distributions.

## v0.2.0 — all distributions (plan from the research of 2026-11-22)
Sources: `docs/research/DISTRO-*-2026-11.md`. Items marked there [NZ]/(unverified)
are checked in VMs in v0.3.0.

1. [x] Safety: stateful directories in `~/.cache` (XFCE sessions, xfce4 clipboard, obexd, ibus,
   JetBrains Local History, kioexec, evolution, tracker3) — never selected by default.
2. [x] Portable build (glibc 2.39, checked in Ubuntu 24.04, Debian 13, Fedora 43, openSUSE TW containers):
   - `gtk4 v4_10` + `adw v1_5`, `adw::Spinner` → `gtk::Spinner`;
   - AppImage built in an `ubuntu:24.04` container (glibc 2.39 instead of 2.44).
3. [x] The AppImage environment does not leak into system programs: `GI_TYPELIB_PATH`, `XDG_DATA_DIRS`,
   `APPDIR`, `APPIMAGE`, `ARGV0`, `OWD`, `FUSERMOUNT_PROG`, also when opening the file manager.
4. [x] Terminals: argv for each one (gnome-terminal `--wait`, konsole `--separate`, xfce4 `--disable-server`…),
   a wrapper that writes the PID and exit code to the report, the GUI checks whether the PID is alive.
5. [x] Elevation without polkit: sudo → doas → `su -c`; explicit `--uid` instead of `SUDO_UID`.
6. [x] Steam protection for every root (`~/.steam/debian-installation`, Flatpak, Snap); new blocks
   (`gvfs-metadata`, `nautilus/tags`, akonadi, kactivitymanagerd, evolution).
7. [x] Desktop rules: KDE ksycoca/QML/kcache, Snap `~/snap/*/common/.cache`, Flatpak per-app cache
   with stateful exceptions, `Trash/directorysizes`.
8. [x] Package manager locks through `/proc/locks` (dpkg, rpm, xbps, apk), `/run/zypp.pid`.
9. [x] Package manager module (`PackageBackend`) chosen by the existing databases:
   dpkg, rpm + dnf4/dnf5, zypper; xbps/apk/Nix only "installed" (no leftovers).
10. [x] System tasks: apt/dnf/zypper/xbps/apk/eclean cache, journald, rotated logs,
    crash reports (apport/ABRT), `flatpak uninstall --unused`, disabled Snap revisions,
    `rpm-ostree cleanup`, old kernel modules on other distributions.
    Old kernels only after a preview, never by default; no `autoremove`.
11. [x] Release 2026-11-22 (0.2.0 - Open Road Update): AppImage + `.deb` + `.rpm` + `.pkg.tar.zst` + `SHA256SUMS`.

## After v0.2.0 (not released yet)
- [x] English (default) and Polish interface, EN/PL switch in the window, `--lang en|pl`.
- [x] Documentation in two languages: `X.md` in English, `X.pl.md` in Polish.

Notes for v0.3.0 (VM): dnf gives no leftovers yet (history in SQLite), and doas, `su`,
apt, dnf, zypper, xbps, apk, Snap and rpm-ostree were not run on real systems.

## Never
- Wiping RAM, swap and free space, "shred".
- Autostart manager.
- Uninstalling apps: XUninstaller does that.
