# XCleaner

Fast, precise disk cleaner for Linux, written in Rust + GTK4/libadwaita.

XCleaner finds caches, old logs, build artifacts and leftovers of uninstalled apps,
shows **why** every item is junk, and deletes only what you tick. Your configuration
and personal data are never selected for you.

[![Latest release](https://img.shields.io/github/v/release/yhymid/XCleaner)](https://github.com/yhymid/XCleaner/releases/latest)
![License: MIT](https://img.shields.io/badge/license-MIT-blue)

> English and Polish interface — switch **EN / PL** in the window's header bar.
> Deletion is permanent — there is no quarantine, and the program says so before every clean.

## Download

Get the files from the [latest release](https://github.com/yhymid/XCleaner/releases/latest):

| File | For |
|---|---|
| `XCleaner-<ver>-x86_64.AppImage` | any distribution with glibc 2.39+, no installation |
| `xcleaner_<ver>-1_amd64.deb` | Ubuntu 24.04+, Debian 13, Linux Mint 22, Pop!_OS |
| `xcleaner-<ver>-1.x86_64.rpm` | Fedora 41+, openSUSE Tumbleweed |
| `xcleaner-<ver>-1-x86_64.pkg.tar.zst` | Arch, CachyOS, EndeavourOS, Manjaro |
| `SHA256SUMS` | checksums of all files |

```sh
sha256sum -c --ignore-missing SHA256SUMS

sudo apt install ./xcleaner_<ver>-1_amd64.deb        # Debian / Ubuntu / Mint
sudo dnf install ./xcleaner-<ver>-1.x86_64.rpm       # Fedora
sudo zypper install --allow-unsigned-rpm ./xcleaner-<ver>-1.x86_64.rpm   # openSUSE
sudo pacman -U xcleaner-<ver>-1-x86_64.pkg.tar.zst   # Arch / CachyOS
chmod +x XCleaner-<ver>-x86_64.AppImage              # AppImage
```

Debian 12 and Ubuntu 22.04 are too old (glibc and GTK).

## Usage

```sh
xcleaner                      # window (from the menu: XCleaner)
xcleaner --scan [--details]   # report in the terminal, deletes nothing
xcleaner --usage [dir]        # what takes space in HOME, like ncdu (read-only)
xcleaner --lang pl --scan     # language for one run: en | pl
```

Run it **without sudo**. System cleaning opens a terminal and asks for the password
there (`sudo`, `doas` or `su`); there is no polkit agent and no background service.

## What it cleans

**Home directory**
- `~/.cache`, one item per program; directories that hold your data (music, session
  state, clipboard history…) are never preselected.
- Browsers and Electron apps: only caches — logins, messages and settings stay.
- Steam (native, deb, Flatpak, Snap): per-game shader caches, orphaned shaders of
  uninstalled games, logs, dumps. Game data is protected.
- Developer caches (cargo, npm, pnpm, pip/uv, go, yay/paru) and build artifacts
  (`target/`, `node_modules`, … about 30 project types).
- Flatpak and Snap app caches, old Snap revisions, Trash, recent-files list.

**Leftovers of uninstalled apps** in `~/.config`, `~/.local/share`, `~/.local/state`
and `~/.var/app`, from the package history of pacman (Arch) and dpkg (Debian/Ubuntu).
Three facts must agree: the name, no installed owner, and the directory existing
before the package was removed. Each item is rated *certain / almost certain / possible*,
with a warning if it contains personal data.

**System** (password in a terminal)
- Package caches through the manager's own command: `paccache`, `apt-get autoclean`,
  `dnf clean packages`, `zypper clean`, `xbps-remove -O`, `apk cache clean`, `eclean`.
- Old systemd journal entries, rotated logs in `/var/log`, coredumps, apport/ABRT
  crash reports.
- Unused Flatpak runtimes, disabled Snap revisions, `rpm-ostree cleanup`.
- Modules of removed kernels and old kernels (package manager only, never the running one).
- Nothing runs while a package manager is busy; on immutable systems (SteamOS,
  Silverblue/Bazzite, NixOS, MicroOS) the system image is left alone.

## Safety

- Deletion goes through file descriptors (`openat2` with `RESOLVE_BENEATH`): no symlink
  following, no leaving the target directory, no other filesystems, no files of other users.
- A hard "never" list: keys, keyrings, desktop configuration, Steam game data, `~/.local/bin`…
- Items of running programs are blocked.
- The window never runs as root; only the selected system tasks do, in a terminal.
- History of every clean: `~/.local/state/xcleaner/history.log`.

## Building

Needs Rust 1.85+, GTK 4.10+ and libadwaita 1.5+ development files.

```sh
cargo build --release
./target/release/xcleaner
```

Release files (Podman or Docker):

```sh
./packaging/build-in-container.sh   # AppImage, built on Ubuntu 24.04 (glibc 2.39)
./packaging/build-native.sh         # .deb (Ubuntu 24.04) and .rpm (Fedora 41)
./packaging/build-pkg.sh            # Arch package (makepkg)
```

## Project layout

- `crates/xcleaner-core` – rules, parallel scanner, safe deletion, system tasks (no GTK).
- `crates/xcleaner` – GTK4/libadwaita window and terminal mode.
- `docs/ROADMAP.md` – plan; `docs/research/` – research notes behind the rules.
- `CHANGELOG.md` – changes per release.

## License

MIT — see [LICENSE](LICENSE).
