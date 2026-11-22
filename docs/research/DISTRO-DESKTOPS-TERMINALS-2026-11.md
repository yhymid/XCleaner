# Desktops, distributions and terminals — research for v0.2.0 (2026-11-22)

**English** | [Polski](DISTRO-DESKTOPS-TERMINALS-2026-11.pl.md)

Scope: `$HOME` on popular distributions and desktops, and starting `sudo <cmd>` in a new terminal window (`root.rs`).
Method: upstream source code (shallow clone or raw from GitHub/GitLab), freedesktop specifications (the `xdg/xdg-specs` repo) and man pages. Nothing was tested on another distribution. Items marked **[NV]** are unverified: based on knowledge or reasoning, without a primary source.
No GPL code or data (BleachBit, Stacer) was copied. Only facts from upstream code are described.

Risk markers are the same as in `rules.rs`: **Safe** (may be selected by default), **Low** (offer, but do not select), **Review** (only show, the user decides), **Never** (a block in `protect.rs`).

---

## 0. Key conclusions for the owner

1. **`~/.cache` is not all cache.** The owner decided that we clean it almost entirely. There are, however, a few directories of desktops and services that keep **state or user data** in `~/.cache`. I confirmed all of them in the sources. They must be added to `CACHE_STATEFUL` (Review) or get their own rules:
   - `sessions`: saved XFCE sessions (`xfce4-session-<display>`) and session thumbnails.
   - `xfce4`: clipman clipboard history, appfinder history and frecency.
   - `obexd`: **files received over Bluetooth** (obexd's default directory).
   - `ibus`: `ibus/libpinyin` and `ibus/libbopomofo` are the **user's learned dictionary** (Chinese IME).
   - `JetBrains`: the IDE's "system" directory also holds **Local History**.
   - `tracker3`: the LocalSearch index. It can be rebuilt, but reindexing takes a lot of CPU and I/O, and tags come on top (see §1.1). So Low, not Safe.
   - `kioexec`: local copies of remote files being edited right now (KIO). See §1.2.
   - `evolution`: an offline copy of mail, calendar and contacts. **[NV]** Changes made offline and not yet synced may be waiting there.
2. **Steam protection is pinned to one path.** `protect.rs` protects only `.local/share/Steam/{userdata,config,…}`. On Debian/Ubuntu Steam's root is `~/.steam/debian-installation`, and in the Flatpak `~/.var/app/com.valvesoftware.Steam/...`. The protection does not work there. The Steam root must be detected and the same lists applied relative to every detected root (§2.3).
3. **Terminal: the current mechanism "wait when the terminal returned in < 1.5 s" is fragile.** Some terminals return immediately by default when an instance is already running: gnome-terminal, kgx, ptyxis `--new-window`, xfce4-terminal, terminator, tilix, konsole with `UseSingleInstance`, `wezterm start`, footclient. If such a terminal returns after 1.6 s, the GUI considers the action cancelled. If the user closes the window, the GUI waits 600 s. Recommendation in §4.3: a blocking flag per terminal plus an own wrapper that writes the PID and exit code to the report file.

---

## 1. Desktops

### 1.1 GNOME

| Path | What it is | Rebuilt the same? | While running | Risk / recommendation |
|---|---|---|---|---|
| `~/.cache/tracker3/files/` (+`errors/`) | LocalSearch index database (formerly tracker-miner-fs-3) | Yes, through a full reindex (CPU/IO, minutes–hours) | **No**: `localsearch reset` first kills the indexer (`SIGKILL` of the process owning `org.freedesktop.LocalSearch3`), only then deletes the directory | **Low**. Upstream equivalent: `localsearch reset -s` (= kill + delete `tracker3/files/errors` and `tracker3/files`). Guard: `localsearch-3`, `tracker-miner-fs-3`. The man page says "Other than tags, actual data … should not be affected", so **tags in the index are lost** |
| `~/.cache/tracker/` | Tracker 2 (old) | Yes | – | Safe when tracker2 is not installed (leftover) [NV] |
| `~/.local/share/nautilus/tags` | **Nautilus stars ("Starred")** | No | – | **Never**. Nautilus keeps them in its own database, not in the tracker3 index (`nautilus-tag-manager.c`) |
| `~/.local/share/gvfs-metadata/` | GVfs metadata: emblems, custom icons, folder view settings, positions | **No** (this is user data, not cache) | – | **Never** (add to `NEVER_INSIDE`) [NV as to the full list of keys] |
| `~/.cache/gnome-software/` | appstream, icons, ODRS ratings, fwupd metadata | Yes (downloads again) | gnome-software runs in the background as a service | Low/Safe. Guard: `gnome-software` [NV: content by the directory layout] |
| `~/.cache/evolution/` | Offline copies of IMAP/EWS/CalDAV/CardDAV | Yes, from the server | **No**: `evolution-source-registry`, `evolution-calendar-factory`, `evolution-addressbook-factory` are running | **Review**. POP and "On This Computer" are kept in `~/.local/share/evolution` (Never). Offline changes may be unsynced [NV] |
| `~/.local/share/recently-used.xbel` | GTK recent files list (recent-files spec) | An empty list | GTK programs watch the file and rewrite it | Review (privacy). A running program may write an entry back from memory [NV] |
| `~/.local/share/gnome-shell/application_state` | "Frequently used" statistics | No | – | Never [NV] |
| `~/.cache/thumbnails` | Thumbnails (§3.1) | Yes | Yes | Safe (already there) |

### 1.2 KDE Plasma (5/6)

| Path | What it is | Rebuilt the same? | While running | Risk / recommendation |
|---|---|---|---|---|
| `~/.local/share/baloo/index` (+`index-lock`) | Baloo index (LMDB, often several GB) | Yes, through a reindex | **No**. `balooctl purge` waits for the D-Bus service to unregister, deletes **only the `index` file** and restarts the daemon if it was running | **Low**. Recommended: call `balooctl6 purge` (Plasma 6; in Plasma 5 `balooctl`) instead of deleting it ourselves. Guard: `baloo_file`, `baloo_file_extractor`. Settings are in `~/.config/baloofilerc`; KDE tags and ratings live in file xattrs, not in the index [NV as to xattr] |
| `~/.cache/ksycoca6_*` (Plasma 5: `ksycoca5_*`) | Service and MIME database cache (KService) | Yes, kbuildsycoca6 rebuilds it automatically | Yes (processes have it mapped, and unlink does not break the mapping) | **Safe**. Path: `GenericCacheLocation + "/ksycoca6" + suffix` (`ksycoca.cpp`) |
| `~/.cache/<app>/qmlcache`, e.g. `plasmashell/qmlcache`, `kwin/qmlcache` | Compiled QML (`.qmlc`) | Yes | Yes | Safe [NV: Qt behaviour based on the QML disk cache documentation] |
| `~/.cache/plasma_theme_*.kcache`, `icon-cache.kcache`, `plasma-svgelements*` | KSharedDataCache/KImageCache | Yes | Yes (mmap) | Safe [NV] |
| `~/.cache/kioexec/krun/<pid>_<n>/<file>` | **A local copy of a remote file opened through KIO.** After the program closes, kioexec asks "upload the changes?" | No, if the copy is modified | **No** | **Review**. Skip a directory with a live `<pid>`. For a dead PID mark it Low and warn that it may be the only copy of the changes (`kio/src/kioexec/main.cpp`) |
| `~/.local/share/akonadi/` (`db_data`, `file_db_data`, `search_db`) | Akonadi database (KMail, KOrganizer) | Partly, through a resync | No | **Never**. Offer `akonadictl vacuum`/`fsck` instead [NV] |
| `~/.local/share/kactivitymanagerd/resources/database` | Activity history **and Kickoff favourites** | No | – | Never [NV] |
| `~/.local/share/RecentDocuments/*.desktop` | KDE recent documents | Empty | Yes | Review (privacy) [NV] |
| `~/.local/share/klipper/history2.lst` | Clipboard history | No | – | Review (privacy) [NV] |
| `~/.cache/krunner/`, `~/.cache/drkonqi/` | Copies of browser bookmark databases for krunner; crash reports | Yes | Yes | Safe [NV] |
| `~/.local/share/kwalletd/` | Wallet | – | – | Never (already there) |

### 1.3 XFCE

| Path | What it is | Risk / recommendation |
|---|---|---|
| `~/.cache/sessions/xfce4-session-<display>` and `sessions/thumbs-<display>/*.png` | **Saved sessions** ("save session on logout") and their previews. `xfsm-util.c` and `xfsm-logout-dialog.c` write through `XFCE_RESOURCE_CACHE` | **Review / CACHE_STATEFUL**. Deleting removes the saved session |
| `~/.cache/sessions/<client>-*` | State files of session clients (`xfce-session-client.c`) | Review |
| `~/.cache/xfce4/clipman/` | **Clipboard history** (`clipman/plugin.c`: `XFCE_RESOURCE_CACHE "xfce4/clipman/"`) | Review (privacy, may hold passwords) |
| `~/.cache/xfce4/appfinder/{history,frecency}` | appfinder history and ranking (`appfinder-model.c`) | Review |
| `~/.cache/thumbnails` | Tumbler uses the standard directory | Safe |
| `~/.config/xfce4/**` | Configuration, desktop icon layout | Never |

### 1.4 Cinnamon, MATE, LXQt, Budgie, COSMIC

| Desktop | Path | Verdict |
|---|---|---|
| Cinnamon | `~/.cache/cinnamon/` (downloaded "spices" indexes and archives) | Safe [NV] |
| Cinnamon | `~/.local/share/cinnamon/{applets,desklets,extensions,search_providers}` | **Never**: these are installed add-ons [NV] |
| Cinnamon/Mint | Nemo uses `~/.local/share/gvfs-metadata` and thumbnails like GNOME | as §1.1 |
| MATE | Caja: standard thumbnails, `~/.config/caja` is configuration | Never for config; no specific cache [NV] |
| LXQt | pcmanfm-qt: standard thumbnails; `~/.cache/lxqt*` | Safe [NV] |
| Budgie | No large caches of its own; uses GNOME and GTK | as GNOME [NV] |
| COSMIC | Configuration in `~/.config/cosmic/` (cosmic-config, versioned keys), state in `~/.local/state/cosmic/` | Never [NV: I did not check the COSMIC cache] |

### 1.5 Other entries in `~/.cache` that are not cache (desktop-independent)

| Path | Evidence | Recommendation |
|---|---|---|
| `~/.cache/obexd/` | bluez `obexd/src/main.c`: `option_root = g_build_filename(g_get_user_cache_dir(), "obexd", …)`, option "Default $XDG_CACHE_HOME" | **CACHE_STATEFUL, Review**: these are files received over Bluetooth |
| `~/.cache/ibus/libpinyin`, `~/.cache/ibus/libbopomofo` | ibus-libpinyin `PYLibPinyin.cc`: `userdir = g_get_user_cache_dir()/ibus/libpinyin` | **Review** (user dictionary). `~/.cache/ibus/bus/registry` is a normal cache (`ibusregistry.c`) |
| `~/.cache/JetBrains/<product><ver>/` | JetBrains documentation: "system directory contains caches and **local history** files", on Linux `~/.cache/JetBrains/<product><version>` | **Review** for the whole. Safer to clean only the `caches/`, `index/`, `log/` subdirectories (Low), not `LocalHistory/` [NV as to the subdirectory names]. Directories of older IDE versions, when a newer one exists, are leftovers (Review) |
| `~/.cache/keepassxc` | Already in `CACHE_STATEFUL` | no change |

---

## 2. Distributions and package formats

### 2.1 Snap (Ubuntu and derivatives)

| Path | Facts (source) | Recommendation |
|---|---|---|
| `~/snap/<app>/<rev>/` | `SNAP_USER_DATA`, i.e. the app's `$HOME`. On refresh snapd **copies the old revision's data to the new one** (`snapdata.go: copySnapData`), and when removing a revision it deletes its directory (`RemoveSnapData`) | Old revisions serve `snap revert`. **Review** and never select. A certain leftover is only a `<rev>` directory that is not in `/snap/<app>/` (the revision is not installed). The proper clean is `snap remove --revision` (root, Phase 2) |
| `~/snap/<app>/<rev>.old` | snapd's trash during an operation (`trashPath = path + ".old"`, restored by `untrash` on error) | Skip while `snapd` runs and a change is in progress. Mark an orphaned `.old` as Low |
| `~/snap/<app>/common/.cache/` | The desktop helpers set `XDG_CACHE_HOME=$SNAP_USER_COMMON/.cache` (`snapcraft-desktop-helpers/common/desktop-exports`, also `snapcraft-desktop-integration`) | Treat **like `~/.cache`**: one item per entry, the same `CACHE_STATEFUL` exceptions, `media_check`. Guard: a process with `snap.<app>.` in `/proc/<pid>/cgroup` [NV: cgroup format] |
| `~/snap/<app>/<rev>/.cache` | Snaps without desktop-helpers (HOME = SNAP_USER_DATA) | As above, but only for the `current` revision |
| `~/snap`, `~/snap/<app>`, `~/snap/<app>/common`, `~/snap/<app>/current` | Containers | `NEVER_EXACT` (add) |

### 2.2 Flatpak

- `~/.var/app/<id>/cache` is mounted in the sandbox as `/var/cache`, and `cache/tmp` as **`/var/tmp`** (`flatpak-run.c`, the `mounts[]` table). So the `flatpak_cache_items` rule is right in principle.
- Fixes:
  1. **`CACHE_STATEFUL` exceptions must also be applied inside** `~/.var/app/<id>/cache/` (e.g. `org.keepassxc.KeePassXC`, apps with ibus or obexd in the sandbox). Today the whole `cache` directory is one Safe item.
  2. A guard based on the last part of the id (`steam`, `firefox`) is weak. A better way without starting subprocesses: the `/run/user/$UID/.flatpak/<instance>/info` files contain `[Application] name=<id>`. A running instance means the item is skipped. This is the same mechanism `flatpak ps` uses [NV: file name by the known layout].
  3. Clean `cache/tmp` (= the app's `/var/tmp`) only when the app is not running.
- `~/.local/share/flatpak/repo/objects`: never (hard links). No change from the HOME-JUNK catalogue.

### 2.3 Steam: different root locations

| Installation | Steam root | Notes |
|---|---|---|
| Arch/CachyOS/Fedora RPM/Bazzite/SteamOS | `~/.local/share/Steam` (`~/.steam/root` and `~/.steam/steam` are symlinks) | SteamOS and Bazzite have native Steam [NV for Bazzite] |
| Debian/Ubuntu (`steam-installer`/`steam-launcher`) | `~/.steam/debian-installation` | Seen in user reports [NV: no Valve document] |
| Flathub `com.valvesoftware.Steam` | `~/.var/app/com.valvesoftware.Steam/...` | The manifest has `--persist=.`, and the wrapper creates symlinks `~/.steam/{root,steam}` → `.` relative to the data directory (`steam_wrapper.py: STEAM_SYMLINKS`). In practice `.../.local/share/Steam` or `.../data/Steam`. **Detect through the symlink `~/.var/app/com.valvesoftware.Steam/.steam/root`** [NV as to the final path] |
| Snap `steam` | `~/snap/steam/common/.local/share/Steam` | [NV] |

**Recommendation:** add `steam_roots()`. It collects candidates (`~/.steam/root`, `~/.steam/steam`, `~/.steam/debian-installation`, `~/.local/share/Steam`, `~/.var/app/com.valvesoftware.Steam/{.steam/root,.local/share/Steam,data/Steam}`, `~/snap/steam/common/.local/share/Steam`), resolves symlinks for each **within HOME**, drops candidates without `steamapps/` and removes duplicates by `(dev, ino)`. Rules S1–S9 and the protection lists from `protect.rs` (`userdata`, `config`, `steamapps/common`, `compatdata`, `workshop`) must then be applied **relative to every root**. Today the protection covers only `.local/share/Steam/…`. The process guard is the same (`steam`, `steamwebhelper`). For the Flatpak the instance check comes on top (§2.2).

### 2.4 Ubuntu and Mint

- Ubuntu ≥ 25.04 chooses the terminal through `xdg-terminal-exec` with the file `~/.config/ubuntu-xdg-terminals.list` (Ubuntu 26.04 documentation, "change the default terminal"). See §4.
- Mint: Cinnamon and Nemo cache as in §1.4. I found no extra directories in HOME [NV].
- On Ubuntu Snap is the default (Firefox and Thunderbird as snaps). The Firefox cache is in `~/snap/firefox/common/.cache/mozilla/firefox/<profile>`, and the profile in `~/snap/firefox/common/.mozilla/firefox/`. The latter is **Never**, like `~/.mozilla` [NV as to the exact paths].

---

## 3. freedesktop specifications

### 3.1 Thumbnails (Thumbnail Managing Standard)
- Directories: `$XDG_CACHE_HOME/thumbnails/{normal,large,x-large,xx-large}` and `fail/<program>/`. The x-large and xx-large sizes came in a newer version, so the whole `thumbnails/` must be covered.
- `fail/` holds records of failed generation. Deleting them only causes a retry, so **Safe**.
- **Shared `.sh_thumbnails/` repositories** in directories with files (e.g. on removable media) are **read-only** by the spec ("A program should never…"). XCleaner should not touch them.
- Spec, section "Deleting Thumbnails": a thumbnail should be deleted when the original does not exist. Optionally, in the future: an "orphaned thumbnails only" mode (reading `Thumb::URI` from the PNG). Expensive, so today it stays a clean of everything.
- The old `~/.thumbnails` directory (pre-XDG): Safe, a leftover [NV].

### 3.2 Recent files
`recently-used.xbel` (GTK/GIO, `~/.local/share`). The old recent-files spec talks about `~/.recently-used`, but that file is obsolete. Both paths: Review (privacy).

### 3.3 Trash (Trash spec 1.0)
- Home trash: `$XDG_DATA_HOME/Trash/{files,info}` and an optional `directorysizes`. `expunged/` is a GVfs extension.
- **`directorysizes`** is a cache of the sizes of directories in the trash. It may be written only through a temporary file and an atomic `rename()`. When emptying the whole trash **also delete `directorysizes`**, because today the `trash` rule skips it and stale entries remain.
- The spec recommends deleting directories from the trash as a whole, not file by file. Delete the pair `files/X` + `info/X.trashinfo`.
- **Trashes on other media** (`$topdir` = mount point):
  - (1) `$topdir/.Trash/$uid`. The implementation **MUST** check that `.Trash` has the sticky bit set and **is not a symlink**. If the test fails, the directory **must not be used** (also for emptying).
  - (2) `$topdir/.Trash-$uid`, when `.Trash` does not exist.
  - Both variants must be shown when listing.
  - CIFS and FTP shares are not covered by the spec.
- **Recommendation for XCleaner:** `fsops::Home` deliberately does not cross mounts, so other disks are not covered today. Plan:
  1. Read `/proc/self/mountinfo` and skip `nfs`, `cifs`, `smb3`, `fuse.sshfs`, `ro` mounts, `/proc`, `/sys`, `/run` (except `/run/media`) and `/`.
  2. For each `$topdir` open an fd of the root and work further through `openat2(RESOLVE_BENEATH|RESOLVE_NO_SYMLINKS|RESOLVE_NO_XDEV)`. Check `.Trash` (sticky, not a symlink, `$uid` belongs to the user) and `.Trash-$uid` (owner = uid, not a symlink).
  3. Show it as a separate item "Trash on <disk label>": **Review**, with a list of names and dates from `.trashinfo`. Paths in `info` on removable media are relative to `$topdir`.
  4. Skip an item from removable media if it disappeared between the scan and the deletion (compare `st_dev`).
  5. This goes beyond Phase 1's "HOME-only" rule, so a separate owner decision is needed.

---

## 4. Terminals: `sudo <cmd>` in a new window

### 4.1 argv table (verified in upstream code or man pages)

`T` = title, `CMD…` = program and arguments as separate argv elements.

| Terminal | argv (recommended) | Title | Blocks until the command ends? | Source |
|---|---|---|---|---|
| **xdg-terminal-exec** | `xdg-terminal-exec [--title=T] CMD…` | `--title=` (when the terminal has `X-TerminalArgTitle`) | It `exec`s the chosen terminal, so it **blocks only when the chosen terminal blocks**. The spec does not define this | README/spec (status: **proposal**, `X-` keys) |
| **x-terminal-emulator** (Debian) | `x-terminal-emulator -T T -e CMD…` | `-T` | The policy says nothing about waiting | Debian Policy 11.8.3: `-e` takes multiple arguments "as though passed directly to execvp" |
| **gnome-terminal** | `gnome-terminal --wait --title=T -- CMD…` | `--title`/`-t` | **Only with `--wait`** ("Wait until the terminal's child exits"). Without it the client returns immediately, because the server opens the window | man `gnome-terminal.xml`; `-e`/`-x` are deprecated |
| **kgx** (GNOME Console) | `kgx -T T -- CMD…` (or `-e CMD…`) | `-T`/`--title` | **No**. `--wait` is defined but described as "(TODO)" and not supported. With a running instance (GApplication unique) it returns immediately | `kgx-application.c`, `kgx-utils.c: kgx_filter_arguments` |
| **ptyxis** | `ptyxis -T T -- CMD…` | `-T`/`--title` ("for new tab") | **Yes** in the current main: `--` (or `-e`, turned into `--`) without `--tab`/`--new-window` enables standalone mode (`G_APPLICATION_NON_UNIQUE`). **Do not add `--new-window`**, as it uses the existing instance. Older releases (≤ 47) may behave differently [NV] | `ptyxis-early-options.c`, `main.c` |
| **konsole** | `konsole --separate -p tabtitle=T -e CMD…` | `-p tabtitle=…` (`-T` is accepted and **ignored**: "Debian policy compatibility, not used") | Yes by default (`UseSingleInstance` = false). When the user enabled "single process" and there is no tty (started from a GUI), it returns immediately. **`--separate`/`--nofork` forces a new process** | `main.cpp: shouldUseNewProcess`, `konsole.kcfg`, `Application.cpp`, `Profile.cpp` |
| **xfce4-terminal** | `xfce4-terminal --disable-server -T T -x CMD…` | `-T` | Only with `--disable-server`. Without it the running instance opens the window through D-Bus | docs.xfce.org (`-x` = the rest of the line, `-e` = one string) |
| **mate-terminal** | `mate-terminal --disable-factory -t T -x CMD…` (or `--`) | `-t`/`--title` | Only with `--disable-factory` | `terminal-options.c` |
| **lxterminal** | `lxterminal --no-remote -t T -e CMD…` | `-t`/`-T`/`--title=` | Yes with `--no-remote` [NV as to the default remote mode] | `lxterminal.c`: `-e` collects the rest of argv; `--command=` is parsed by `g_shell_parse_argv` |
| **qterminal** | `qterminal -e CMD…` | **no option** | Probably yes [NV] | `main.cpp`: `-e`, the first argument through `parse_command` (split into words!), the rest of argv raw |
| **terminator** | `terminator -u -T T -x CMD…` | `-T`/`--title` | Only with `-u`/`--no-dbus`. Otherwise, if a master exists, it sends a request over D-Bus and exits | `optionparse.py`, the `terminator` script |
| **tilix** | `tilix --new-process -t T -e CMD…` | `-t`/`--title` | Only with `--new-process` ("Not Recommended"). GApplication unique by default | `app.d`: `-e`/`-x` joined into **one string** (arguments with a space are quoted), then split again; `application.d` |
| **alacritty** | `alacritty -T T -e CMD…` | `-T`/`--title` | Yes (new process) | man alacritty(1): `-e` "must be last argument" |
| **kitty** | `kitty -T T CMD…` | `-T`/`--title` | Yes, unless `-1/--single-instance` is given | `kitty --help` |
| **foot** | `foot -T T CMD…` | `-T`/`--title=` | Yes. `footclient` also waits, unless it gets `-N/--no-wait` | man foot(1), footclient(1) |
| **wezterm** | `wezterm start --always-new-process -- CMD…` (or `wezterm -e CMD…`) | no CLI option (`--class` changes the app_id) | `start` **without** `--always-new-process` delegates to the running GUI and returns. `-e` is a hidden `BlockingStart` (always a new process) | `wezterm-gui/src/main.rs`, `wezterm-gui-subcommands` |
| **ghostty** | `ghostty --title=T -e CMD…` | `--title=` (**before** `-e`) | Yes. `-e` forces `gtk-single-instance=false` and `quit-after-last-window-closed=true` | `src/config/Config.zig` |
| **cosmic-term** | `cosmic-term -e CMD…` (or `--`) | **none** (unknown flags are ignored with a warning) | Yes, as long as stdin is **not** a tty: with a tty it does `fork::daemon` and returns. XCleaner gives `stdin(Stdio::null())`, so OK. There is also `--no-daemon` | `cosmic-term/src/main.rs` |
| **xterm** | `xterm -T T -e CMD…` | `-T` | Yes | xterm(1), the model for Debian Policy |

**Terminal Intent (D-Bus `org.freedesktop.Terminal1.LaunchCommand`)** is in the xdg-specs repo as "0.1 (DRAFT)". It passes `exec` as an argv vector, but **has no completion signal**, so a report file is needed anyway. Do not implement for now.

### 4.2 Default terminals

| Distribution / desktop | Default |
|---|---|
| Fedora Workstation 41+ | Ptyxis |
| Ubuntu 25.10+ (GNOME) | Ptyxis (+ `xdg-terminal-exec`, `ubuntu-xdg-terminals.list`). ≤ 25.04: gnome-terminal |
| GNOME upstream (≥ 42), Arch `gnome` | GNOME Console (kgx) |
| Debian 13 GNOME | gnome-terminal or kgx [NV: I did not check `task-gnome-desktop`] |
| KDE (Neon, Kubuntu, Fedora KDE, SteamOS desktop) | konsole |
| Bazzite | Ptyxis [NV] |
| Mint Cinnamon | gnome-terminal [NV] |
| XFCE / Xubuntu | xfce4-terminal |
| MATE | mate-terminal |
| LXQt / Lubuntu | qterminal |
| LXDE | lxterminal |
| Ubuntu Budgie | Tilix or gnome-terminal [NV] |
| COSMIC (Pop!_OS 24.04) | cosmic-term |
| CachyOS/Hyprland (the owner's) | kitty (depends on the edition) [NV] |

### 4.3 Recommendations for `root.rs`

1. **Fix the argv** in `terminal_args`:
   - `gnome-terminal`: add `--wait`.
   - `kgx`: the current `--title T --` is correct (`-T` and `--title` are the same option). kgx does not block, so point 3 is needed.
   - `ptyxis` (new entry): `-T T --`.
   - `konsole`: `--separate -p tabtitle=T -e`.
   - `xfce4-terminal`: `--disable-server -T T -x`.
   - `terminator`: `-u -T T -x`.
   - `wezterm`: `start --always-new-process --`.
   - New entries: `mate-terminal --disable-factory -t T -x`, `tilix --new-process -t T -e`, `lxterminal --no-remote -t T -e`, `ghostty --title=T -e`, `cosmic-term -e`, `qterminal -e`, `x-terminal-emulator -T T -e`.
   - The default branch `_ => -e` follows Debian Policy, so it may stay.
2. **Order of choice:**
   - `$TERMINAL`, then `xdg-terminal-exec` (only if a `*xdg-terminals.list` file exists in `~/.config` or `/etc/xdg`; without configuration the choice is undefined).
   - Then the terminal matching `XDG_CURRENT_DESKTOP` (GNOME → ptyxis/kgx/gnome-terminal, KDE → konsole, XFCE → xfce4-terminal, MATE, LXQt, COSMIC, X-Cinnamon → gnome-terminal).
   - Then `x-terminal-emulator`, and finally the current list.
3. **Reliable end detection** (most important):
   - Instead of `sudo …` run an **own wrapper** in the terminal as the user, e.g. `<self> --term-wrap --report R -- sudo env … <self> --root …`.
   - The wrapper (a) appends `wrapper\t<pid>` to the report file, (b) runs `sudo` as a child, (c) after it ends appends `exit\t<code>`.
   - The GUI always polls the report, regardless of when the terminal returned, and finishes when `done` or `exit` appears, or when the wrapper's PID disappears from `/proc` (closed window: SIGHUP kills the wrapper).
   - When the wrapper has not registered within e.g. 30 s, the terminal did not start.
   - This solves: gnome-terminal without `--wait`, kgx, single-instance modes, waiting 600 s after the window is closed, and a false "cancelled" after a slow start.
4. **Environment:**
   - Terminals with a server (gnome-terminal-server, kgx, tilix, terminator, xfce4-terminal without `--disable-server`, single-instance konsole) run the command in the **server's environment**, not XCleaner's. `env_remove(FOREIGN_ENV)` has no effect on them. This is not critical, as `sudo` resets the env, and `APPIMAGELAUNCHER_DISABLE` goes through `env` in argv. That is why "new process" flags are preferred.
   - **qterminal and tilix join or split arguments through a shell.** Paths with `"`, `'`, `$` or a space (an AppImage in `~/Downloads/My Programs/`) may fall apart. For them check whether `me` and the report path contain only safe characters, and otherwise skip that terminal.
   - lxterminal `-e` passes raw argv (OK).

---

## 5. Proposed new rules and changes (summary)

| # | Change | Risk | Evidence |
|---|---|---|---|
| 1 | `CACHE_STATEFUL += sessions, xfce4, obexd, ibus, JetBrains, kioexec, evolution, tracker3` | Review | §1.1–1.5 |
| 2 | `kde-sycoca` rule: `~/.cache/ksycoca5_*`, `ksycoca6_*` | Safe | `ksycoca.cpp` |
| 3 | `kde-qmlcache` rule: `~/.cache/*/qmlcache`, `plasma_theme_*.kcache`, `icon-cache.kcache` | Safe | [NV partly] |
| 4 | `baloo-index` rule: the `~/.local/share/baloo/index` file (guard `baloo_file`; ideally `balooctl6 purge`) | Low (reindex) | `purgecommand.cpp` |
| 5 | `localsearch-index` rule: `~/.cache/tracker3/files` (guard `localsearch-3`, `tracker-miner-fs-3`; ideally `localsearch reset -s`) | Low (reindex, tags) | `tracker-reset.c`, man |
| 6 | `NEVER_INSIDE += .local/share/gvfs-metadata, .local/share/nautilus, .local/share/akonadi, .local/share/kactivitymanagerd, .local/share/cinnamon, .local/share/evolution, .var/app/*/data, .var/app/*/config` (the last two need pattern matching) | Never | §1, §2.2 |
| 7 | `NEVER_EXACT += snap, snap/<app>, snap/<app>/common, snap/<app>/current` | – | §2.1 |
| 8 | Snap cache `~/snap/<app>/common/.cache/*` as a second "~/.cache" | Safe with exceptions | desktop-exports |
| 9 | Old Snap revisions (a `<rev>` directory without `/snap/<app>/<rev>`) | Review | `snapdata.go` |
| 10 | Flatpak: stateful exceptions inside `cache/`, guard through `/run/user/$UID/.flatpak/*/info` | – | `flatpak-run.c` |
| 11 | `steam_roots()` + protection relative to the root (Debian, Flatpak, Snap) | – | §2.3 |
| 12 | Trash: also delete `Trash/directorysizes`; `.Trash-$uid` trashes on other disks as Review (owner decision) | Review | Trash spec |
| 13 | Thumbnails: do not touch `.sh_thumbnails`; add the old `~/.thumbnails` | Safe | Thumbnail spec |
| 14 | Privacy (Review): KDE `RecentDocuments`, klipper, XFCE clipman | Review | §1.2–1.3 |
| 15 | `root.rs`: argv from §4.3, a wrapper for end detection, terminal choice by desktop | – | §4 |

---

## Sources

- Debian Policy 11.8.3 x-terminal-emulator: https://www.debian.org/doc/debian-policy/ch-customized-programs.html
- xdg-terminal-exec (spec proposal): https://github.com/Vladimir-csp/xdg-terminal-exec (README); MR: https://gitlab.freedesktop.org/terminal-wg/specifications
- Terminal Intent (draft 0.1): https://gitlab.freedesktop.org/xdg/xdg-specs/-/tree/master/terminal-intent
- gnome-terminal man: https://github.com/GNOME/gnome-terminal/blob/master/man/gnome-terminal.xml
- GNOME Console: https://github.com/GNOME/console/blob/main/src/kgx-application.c , `src/kgx-utils.c`
- Ptyxis: https://gitlab.gnome.org/chergert/ptyxis/-/blob/main/src/ptyxis-early-options.c , `src/main.c`, `src/ptyxis-application.c`; man: https://man.archlinux.org/man/extra/ptyxis/ptyxis.1.en
- Konsole: https://github.com/KDE/konsole/blob/master/src/main.cpp , `src/Application.cpp`, `src/settings/konsole.kcfg`, `src/profile/Profile.cpp`; https://docs.kde.org/stable_kf6/en/konsole/konsole/command-line-options.html
- xfce4-terminal: https://docs.xfce.org/apps/xfce4-terminal/command-line
- mate-terminal: https://github.com/mate-desktop/mate-terminal/blob/master/src/terminal-options.c
- lxterminal: https://github.com/lxde/lxterminal/blob/master/src/lxterminal.c
- qterminal: https://github.com/lxqt/qterminal/blob/master/src/main.cpp
- Terminator: https://github.com/gnome-terminator/terminator/blob/master/terminatorlib/optionparse.py , the `terminator` script
- Tilix: https://github.com/gnunn1/tilix/blob/master/source/app.d , `source/gx/tilix/application.d`
- Alacritty: https://man.archlinux.org/man/alacritty.1.en ; foot/footclient: https://man.archlinux.org/man/foot.1.en , https://man.archlinux.org/man/footclient.1.en ; kitty: `kitty --help` (local)
- WezTerm: https://github.com/wezterm/wezterm/blob/main/wezterm-gui/src/main.rs , `wezterm-gui-subcommands/src/lib.rs`
- Ghostty: https://github.com/ghostty-org/ghostty/blob/main/src/config/Config.zig
- COSMIC Terminal: https://github.com/pop-os/cosmic-term/blob/master/src/main.rs
- Ubuntu and Ptyxis: https://www.omgubuntu.co.uk/2025/05/ubuntu-25-10-to-include-new-image-viewer-terminal-apps , https://ubuntu.com/desktop/docs/en/26.04/how-to/change-the-default-terminal/ , https://phoronix.com/news/Ubuntu-Ptyxis-Recommended
- LocalSearch: https://github.com/GNOME/localsearch (`src/cli/tracker-reset.c`, `docs/man1/localsearch-reset.1.txt`, `src/indexer/meson.build`)
- Nautilus tags: https://github.com/GNOME/nautilus/blob/main/src/nautilus-tag-manager.c
- Baloo: https://github.com/KDE/baloo (`src/tools/balooctl/purgecommand.cpp`, `src/engine/global.cpp`)
- KService ksycoca: https://github.com/KDE/kservice/blob/master/src/sycoca/ksycoca.cpp
- kioexec: https://github.com/KDE/kio/blob/master/src/kioexec/main.cpp
- xfce4-session: https://github.com/xfce-mirror/xfce4-session (`libxfsm/xfsm-util.c`, `xfce4-session/xfsm-logout-dialog.c`)
- xfce4-clipman: https://github.com/xfce-mirror/xfce4-clipman-plugin/blob/master/panel-plugin/plugin.c ; appfinder: https://github.com/xfce-mirror/xfce4-appfinder/blob/master/src/appfinder-model.c
- BlueZ obexd: https://github.com/bluez/bluez/blob/master/obexd/src/main.c
- ibus-libpinyin: https://github.com/libpinyin/ibus-libpinyin/blob/master/src/PYLibPinyin.cc ; ibus registry: https://github.com/ibus/ibus/blob/main/src/ibusregistry.c
- JetBrains directories: https://www.jetbrains.com/help/idea/directories-used-by-the-ide-to-store-settings-caches-plugins-and-logs.html
- snapd: https://github.com/canonical/snapd/blob/master/overlord/snapstate/backend/snapdata.go ; desktop helpers: https://github.com/ubuntu/snapcraft-desktop-helpers/blob/master/common/desktop-exports , https://github.com/canonical/snapcraft-desktop-integration
- Flatpak: https://github.com/flatpak/flatpak/blob/main/common/flatpak-run.c
- Flathub Steam: https://github.com/flathub/com.valvesoftware.Steam (`com.valvesoftware.Steam.yml`, `steam_wrapper/steam_wrapper.py`)
- Trash spec 1.0 and Thumbnail spec: https://gitlab.freedesktop.org/xdg/xdg-specs (`trash/index.rst`, `thumbnail/thumbnail-spec.xml`, `recent-files/recent-file-spec.xml`)
