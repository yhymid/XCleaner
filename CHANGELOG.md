# Changelog

**English** | [Polski](CHANGELOG.pl.md)

## Unreleased

### Language
- The program is in English (default) and Polish. The **EN / PL** switch is in the window's
  header bar; the choice is saved in `~/.config/xcleaner/settings.txt`. In a terminal: `--lang en|pl`.
- The password terminal and the root helper speak the window's language.
- Sizes in English with a dot (`1.5 GiB`), in Polish with a comma (`1,5 GiB`).
- Documentation in two languages: `X.md` in English, `X.pl.md` in Polish.

## 0.2.0 - Open Road

XCleaner goes beyond Arch: it works on Ubuntu, Debian, Mint, Fedora, openSUSE
and their derivatives. Every new item has a source in the code or documentation of that system,
and anything doubtful is review-only.

### On every distribution
- `.deb` (Ubuntu 24.04+, Debian 13, Mint 22, Pop!_OS) and `.rpm` (Fedora 41+,
  openSUSE Tumbleweed) packages next to the AppImage and the pacman package. The AppImage now needs
  glibc 2.39 instead of 2.44, so it also runs outside Arch.
- Leftovers of uninstalled apps on Debian and Ubuntu (`dpkg.log`, including
  old compressed logs, and "rc" packages). On Fedora and openSUSE the package history is not
  readable without root, so the program guesses nothing there.
- The password terminal is chosen by desktop (Konsole, Ptyxis, GNOME Terminal, xfce4-terminal…),
  with the right arguments for each. The program knows when the window was closed or the password was wrong.
- Password through `sudo`, `doas` or `su` (Debian with a root password). Still no polkit.

### System
- Package caches through the managers' own commands: `apt-get autoclean` (the full `apt-get clean` is
  review-only), `dnf clean packages`, `zypper clean`, `xbps-remove -O`, `apk cache clean`,
  `eclean distfiles`.
- Old systemd journal entries (`journalctl --vacuum-time=4weeks`), rotated logs
  in `/var/log` (never active ones, audit or login history).
- Unused Flatpak runtimes, disabled Snap revisions, `rpm-ostree cleanup`.
- apport (Ubuntu) and ABRT (Fedora) crash reports.
- Old kernels — only with the package manager, review-only; the running one stays.
- Modules of removed kernels on Debian and Fedora too.
- On immutable systems (SteamOS, Silverblue/Bazzite, NixOS, MicroOS) the program does not
  touch the system image.
- pacman: empty and abandoned `download-*` directories (`pacman -Sc` used to fail on them),
  databases of repositories removed from `pacman.conf`.
- Every task waits while a package manager runs (apt, dnf, zypper, xbps, apk, pacman).

### Home directory
- Steam from the deb package, Flatpak and Snap: game data protection and shaders as in a regular install.
- Snap app caches and data of old Snap revisions; Flatpak and Snap are blocked while the app runs.
- Trash: `directorysizes` too.

### Safety
- Never preselected: directories in `~/.cache` that hold your data (XFCE sessions, clipboard history,
  Bluetooth files, ibus dictionaries, JetBrains Local History, Evolution mail copies, tracker3 index).
- New blocks: Nautilus stars, GVfs metadata, Akonadi, KDE favourites, Cinnamon extensions,
  Evolution local mail, `data` and `config` of Flatpak apps, Firefox profiles in Snap.
- Programs started from the AppImage (terminal, file manager, pacman) do not get its
  libraries or variables.

### Fixes
- Cleaning history: every system task has its own line with the real freed space.

## 0.1.0 - First Sweep

The first release. XCleaner looks for junk in the home directory and in the system,
shows proof for every item and deletes only what you tick.

### Home directory
- The `~/.cache` cache. Every program is a separate item. Directories with music
  or videos are recognised and never preselected.
- Browsers (Chrome, Chromium, Firefox, Zen) and Electron apps (Discord, VSCodium…):
  cache only. Logins, messages and settings stay.
- Steam: shaders of every game separately with the game's name, orphaned shaders of uninstalled
  games, logs and dumps, store cache. Launchers: Prism, Bedrock.
- Development: yay/paru (without `vcs.json`), cargo, npm, pnpm, pip/uv, go,
  project build artifacts (`target/`, `node_modules`… — about 30 project types).
- Old Discord versions after auto-update, dead menu shortcuts, Flatpak cache.
- Trash and the recently opened files list (always review-only).

### Leftovers of uninstalled apps
- Directories in `~/.config`, `~/.local/share`, `~/.local/state`, `~/.var/app` of packages removed
  according to `pacman.log`. 3 facts must agree: the name, no installed owner
  and a creation date before the package was removed.
- "certain / almost certain / possible" tiers with a reason.
- A warning when your data is inside (browser profiles, game saves, screenshots).

### System (sudo password in a terminal)
- Crash dumps (coredumps), modules of removed kernels (never the running one),
  pacman cache: `paccache -rk2`, and the full clean only when you explicitly tick it.

### Window
- Categories, sizes, details of every item, opening in the file manager.
- "Keep this" (permanent keep list), "Events" log (copy / save as .md),
  cleaning history in `~/.local/state/xcleaner/history.log`.
- "What takes space" — browsing directory sizes like ncdu.
- Terminal mode: `xcleaner --scan [--details]`, `xcleaner --usage [dir]`.

### Safety
- Deletion through descriptors (no following symlinks, no leaving the directory,
  no other filesystems, no files of other users).
- A hard "never" list: keys, keyrings, desktop configuration, Steam game data, `~/.local/bin`…
- Items of running programs are blocked.
- Deletion is permanent — the program says so before every clean.

### Performance
- Scanning everything takes about 1 s; "What takes space" for the whole HOME (440 GB) about 3 s.
- Large directories (e.g. game shaders) are deleted with multiple threads.
