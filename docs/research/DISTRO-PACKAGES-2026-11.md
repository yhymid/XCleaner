# App leftovers on other distributions: package managers (2026-11-22)

**English** | [Polski](DISTRO-PACKAGES-2026-11.pl.md)

Goal of v0.2.0: extend the detection from `LEFTOVERS-DETECTION-2026-11.md` (today pacman only) to
popular distributions. All detection must work **without root** and **without a process per package**.
The three-fact model stays unchanged:

1. **removed**: the name matches a package removed according to the package manager's log,
2. **no installed identity**: the local package database (names, provides, files), PATH,
   `.desktop`, Flatpak, Snap, AppImage, `/proc`,
3. **the directory's btime is not later than the removal**.

On top of that, **aliases of the removed package**, i.e. the names of its binaries and `.desktop` files. On Arch
`pacman -Fl` gives them.

Markers: **[S]** = checked in the source or in the official documentation (link in §9).
**[NV]** = unverified, from memory or second-hand; check on a real machine
or in a VM. No GPL code was copied; only the formats are described.

---

## 0. TL;DR: recommended design

| Family | Fact 1: removals | Fact 2: installed | Aliases of removed | Lock (manager running) |
|---|---|---|---|---|
| pacman (Arch, CachyOS, Manjaro, EndeavourOS, SteamOS) | `/var/log/pacman.log` (exists) | `/var/lib/pacman/local` (exists) | `pacman -Fl` (exists) | `/var/lib/pacman/db.lck` exists |
| dpkg/apt (Debian, Ubuntu, Mint, Pop!_OS, Zorin, elementary, MX) | `/var/log/dpkg.log` + `dpkg.log.N[.gz]` (status `config-files`/`not-installed`); in addition `apt/history.log*` | `/var/lib/dpkg/status` + `info/*.list` | 1) `info/<pkg>.list` of `rc` packages (files left after removal); 2) `Contents-*.lz4` from `/var/lib/apt/lists`, if present; 3) `.deb` in `/var/cache/apt/archives` | fcntl on `/var/lib/dpkg/lock-frontend` / `lock` → `/proc/locks` |
| rpm + dnf4 (RHEL/Alma/Rocky 8–9, older Fedora) | `/var/lib/dnf/history.sqlite` (`trans_item.action=8`) | `rpmdb.sqlite` (indexes `Name`, `Providename` + headers) or a single `rpm -qa --qf` | `filelists.xml.gz` in the dnf cache, if present; usually absent | `/var/lib/rpm/.rpm.lock` (fcntl), `/run/dnf/rpmtransaction.lock` |
| rpm + dnf5 (Fedora ≥ 41, Nobara) | `/usr/lib/sysimage/libdnf5/transaction_history.sqlite` (`action_id=5`) | as above | as above (dnf5 does not download filelists by default) | as above |
| zypper (openSUSE TW/Leap, Aeon) | `/var/log/zypp/history` (`remove` in CSV with `\|`), permissions [NV] | rpmdb ndb (`/usr/lib/sysimage/rpm/Packages.db`) → a single `rpm -qa --qf` | `filelists.xml.gz` in `/var/cache/zypp/raw/…` [NV] | `/run/zypp.pid` (PID in the file) |
| xbps (Void) | no log → "unknown" only | `/var/db/xbps/pkgdb-0.38.plist` + `.<pkg>-files.plist` | none | flock `/var/db/xbps/lock` |
| apk (Alpine) | no log | `/lib/apk/db/installed` | none | `/lib/apk/db/lock` |
| portage (Gentoo) | `/var/log/emerge.log` (0660 portage → usually unreadable) | `/var/db/pkg/*/*/CONTENTS`, `PROVIDES` | none (CONTENTS disappears with the package) | `/var/db/.pkg.portage_lockfile` |
| eopkg (Solus) | `/var/lib/eopkg/history/*.xml` | `/var/lib/eopkg/package/*/{metadata,files}.xml` [NV] | none | [NV] |
| Nix (NixOS) | **skip**: no "removals" as events | profiles (`/run/current-system/sw`, `~/.nix-profile`, `/etc/profiles/per-user/$USER`) as fact 2 | — | — |
| Flatpak | journal (`MESSAGE_ID=c7b39b1e…`), optional | `app/<id>/current` in the installations | name = ID | — |
| Snap | no file log | `/snap/<name>`, `/var/lib/snapd/snaps/<name>_*.snap` | — | — |

Design rules:
- Introduce a trait `PackageBackend { removed_events(), installed_identities(),
  removed_aliases(), is_busy() }`. Choose the backend by **the existence of databases** (`/var/lib/pacman/local`,
  `/var/lib/dpkg/status`, `rpmdb.sqlite`/`Packages.db`, `/var/db/xbps`, `/lib/apk/db/installed`,
  `/var/db/pkg`, `/var/lib/eopkg`), not by `/etc/os-release`. Several backends may work on one system
  at once, e.g. Debian with the `rpm` package (it has its own, empty database).
- A backend without a removal log (xbps, apk, Nix, Gentoo without access to `emerge.log`) **does not
  produce fact 1**. Its directories go to "unknown" and are never offered. In line with
  the "precise over aggressive" rule.
- All reads are file-based. Exceptions, with a switch like `--no-files-db`:
  - a single `rpm -qa --qf …` on the rpm family, while parsing rpm header blobs is not
    ready yet or the database is in the ndb format;
  - optionally a single `journalctl` for the Flatpak history.
- When the manager's lock is active, the scan skips the leftovers section and shows the message
  "the package manager is working". The database may be half-written then.

---

## 1. dpkg / apt (Debian and derivatives)

### 1.1 `/var/log/dpkg.log` (+ `dpkg.log.1`, `dpkg.log.N.gz`): the main source of fact 1

- **Format** [S dpkg(1) `--log`]. Four line types:
  ```
  YYYY-MM-DD HH:MM:SS startup archives|packages <command>
  YYYY-MM-DD HH:MM:SS status <state> <pkg>[:arch] <installed-version>
  YYYY-MM-DD HH:MM:SS <action> <pkg>[:arch] <installed-version> <available-version>   # install|upgrade|remove|purge|configure|trigproc
  YYYY-MM-DD HH:MM:SS conffile <file> install|keep
  ```
  Example [NV, shape matching the man page]:
  ```
  2026-10-21 18:02:11 remove discord:amd64 0.0.70 <none>
  2026-10-21 18:02:11 status half-configured discord:amd64 0.0.70
  2026-10-21 18:02:12 status config-files discord:amd64 0.0.70
  2026-10-21 18:02:12 status not-installed discord:amd64 <none>
  ```
- **Removal semantics** [S `src/main/remove.c`]:
  - `dpkg -r` logs the `remove` action and ends with the `config-files` status (that is "rc");
  - `--purge` logs `purge` and ends with `not-installed`;
  - a package without conffiles and without postrm goes straight to purge (`not-installed`).
- **Parser rule**: the last `status installed` for a given pkg is the installation, and the last
  `status config-files|not-installed` is the removal. Any later `status installed`
  cancels the removal, like `installed`/`upgraded` in pacman. Names: strip `:arch`.
  Note: an upgrade also produces `status half-installed`/`unpacked`. These must not be
  treated as a removal.
- **Rotation** [S `debian/dpkg.logrotate`]: `monthly`, `rotate 12`, `compress`,
  `delaycompress`, `create 644 root root`. So the history goes back at most about 12 months;
  `dpkg.log.1` is plain text, older ones are `.gz` (needs `flate2`).
- **Reliability**: high. It records every dpkg call, also from apt, aptitude,
  PackageKit, unattended-upgrades and Synaptic.
- **Cost**: about 1–5 MB of text after decompression, linear parsing, below 100 ms [NV].
- **Root**: no (0644).

### 1.2 `/var/log/apt/history.log` (+ `.N.gz`): a supplement

- **Format** [S `apt-pkg/deb/dpkgpm.cc`]. Blocks separated by an empty line:
  ```
  Start-Date: 2026-10-21  18:02:05
  Commandline: apt remove discord
  Requested-By: anon (1000)
  Remove: discord:amd64 (0.0.70), libfoo:amd64 (1.2-3)
  End-Date: 2026-10-21  18:02:14
  ```
  Keys: `Install`, `Reinstall`, `Upgrade`, `Downgrade`, `Remove`, `Purge`, optionally
  `Error`. The list has the form `name:arch (version[, automatic])`, and elements are separated by `), `.
- **Permissions**: apt does `chmod 0644` on history.log [S]; `term.log` is 0640. Rotation
  `monthly`, `rotate 12` [S `debian/apt.logrotate`].
- **Value**: gives the `Requested-By` field and tells a removal by the user from
  an autoremove (`Commandline: apt autoremove`). It does not see operations done with plain `dpkg -r`.
  Role: optional, for the UI ("removed by apt remove").

### 1.3 `/var/lib/dpkg/status`: fact 2 and the "rc" list

- **Format**: deb822, stanzas separated by an empty line; fields `Package`, `Status`,
  `Architecture`, `Provides`, `Version`, `Conffiles` (indented lines `/etc/x md5`) etc.
- `Status: <want> <flag> <state>`, e.g. `install ok installed` or `deinstall ok config-files`.
  Here `state` takes the values: not-installed, config-files, half-installed, unpacked,
  half-configured, triggers-awaited, triggers-pending, installed [S dpkg(1)].
- **Installed** are the states from `unpacked` upwards, and more cautiously everything except `not-installed`
  and `config-files`. `config-files` packages (`rc` in `dpkg -l`) are **evidence of a removal without
  a date**. The date comes from dpkg.log, and if the log is too short, it is fact 1 without a date (a lower
  tier, without the btime rule).
- `Provides: foo (= 1.0), bar`: split by commas and strip the version in brackets.
- **Cost**: a file of about 2–5 MB for about 2–3 thousand packages, one read [NV]. **Root**: no
  (0644 [NV, standard]).

### 1.4 `/var/lib/dpkg/info/<pkg>[:arch].list`: files

- One absolute path per line, directories included. For `Multi-Arch: same` packages
  the file name has a `:arch` suffix, e.g. `libc6:amd64.list` [NV, known behaviour].
- For installed packages this gives the same as `local/*/files` in pacman: `usr/bin/*`,
  `usr/share/applications/*.desktop`, `opt/*`, `etc/xdg/*`.
- **Key for aliases** [S `remove.c`]: on `remove` (without purge) the `.list` file **is not
  deleted**. dpkg writes it again, keeping only what it did not remove: conffiles and
  non-empty or shared directories. On `purge` the `.list` is deleted. For `rc` packages
  this gives aliases like `/etc/<app>/…`, `/etc/xdg/autostart/<app>.desktop`. The binaries are
  no longer there.
- **Cost**: thousands of small files, but we read them for the installed ones anyway, like `files` in
  pacman. Parallel time of the order of 100–300 ms on an SSD [NV].

### 1.5 Files of a REMOVED package (the counterpart of `pacman -Fl`)

Order by value:

1. **`.list` of rc packages** (§1.4): always available, but only conffiles and directories.
2. **`Contents-<arch>` in `/var/lib/apt/lists/`** [S apt-file(1), Debian repo format]:
   - Downloaded only when `apt-file` is installed. That package adds the
     `Acquire::IndexTargets::deb::Contents-deb` targets in `/etc/apt/apt.conf.d/50apt-file.conf`.
     On a fresh Ubuntu, Debian and Mint it is **absent** by default [NV as to the default installs].
   - The name on disk has the form
     `<host>_<path>_dists_<suite>_<comp>_Contents-<arch>.lz4`. Apt keeps the lists
     recompressed to lz4 [NV: check the extension on a machine; it may also be `.gz`].
   - Format [S wiki.debian.org/DebianRepository/Format]: an optional header, then
     `path-without-leading-/␣+[area/]section/name[,…]`. The path may contain spaces, so
     split by the **last** run of spaces, as package names have no spaces.
     ```
     usr/bin/discord                                     net/discord
     usr/share/applications/discord.desktop              net/discord
     ```
   - **Cost**: tens of MB after decompression per suite. With streaming reading it is enough
     to keep lines with the prefixes `usr/bin/`, `usr/share/applications/`, `opt/` and `etc/xdg/`
     for names from the removed set. Needs the `lz4_flex` crate (MIT) [NV crate choice]. Read without
     root (0644). Cache keyed by the mtime of the Contents files, as in pacman.
   - **Limitation**: Contents describes the versions available in the repo. Packages from outside the repo (`.deb` from
     the web: Discord, Chrome, Steam) and PPAs without Contents have no entries.
3. **`/var/cache/apt/archives/*.deb`**: the archive of the removed package, if the cache was not
   cleaned. The `apt` command deletes downloaded `.deb` files after installation by default
   (`APT::Keep-Downloaded-Packages "false"` for the `apt` binary) [NV]. Reading a `.deb` means
   an ar container with `data.tar.{xz,zst,gz}` inside. Low gain, can be skipped in v0.2.
4. We skip `apt-file list`: it is a process per package and needs Contents.

Recommendation: v0.2 = 1 + 2 (if the files exist). Without Contents the package name, btime and
XU records work, just as for AUR.

### 1.6 Locks (apt/dpkg running)

- `/var/lib/dpkg/lock-frontend` (apt and frontends, dpkg ≥ 1.19.1, the variable
  `DPKG_FRONTEND_LOCKED` [S dpkg(1)]), `/var/lib/dpkg/lock` (dpkg), `/var/lib/apt/lists/lock`,
  `/var/cache/apt/archives/lock`. All of them are **fcntl locks** on files that always
  exist, so the mere existence of the file means nothing [NV: paths per practice; man dpkg
  does not list them].
- The files are usually 0640 root, so a regular user cannot do `F_GETLK` (it needs
  opening). **Method without root**:
  - `stat()` of the file (needs only the search right on the directory) gives `st_dev`/`st_ino`;
  - we look for a line in `/proc/locks` (0444, checked here: `-r--r--r--`) with the field
    `MAJ:MIN:INODE` (MAJ:MIN in hex) equal to that file;
  - the same method works for rpm (`.rpm.lock`), xbps (flock) and dnf5.

---

## 2. rpm (Fedora, RHEL/Alma/Rocky, Nobara, openSUSE)

### 2.1 The rpm database

- **Path**: `%_dbpath = %{_var}/lib/rpm` upstream [S `macros.in`]. Fedora ≥ 36
  moved it to `/usr/lib/sysimage/rpm` and left a `/var/lib/rpm` link [NV, Fedora
  change]. openSUSE uses `/usr/lib/sysimage/rpm` (the `usr-lib-sysimage-rpm.patch` in
  openSUSE's rpm package, visible in the package's file list). rpm-ostree keeps the database in
  `/usr/share/rpm` with a link `L /var/lib/rpm → ../../usr/share/rpm` [S
  `rpm-ostree-0-integration.conf`].
  Recommendation: check `/usr/lib/sysimage/rpm`, `/var/lib/rpm` and `/usr/share/rpm` in turn.
- **Backends** [S `lib/backend/`]: `sqlite` (`rpmdb.sqlite`), `ndb` (`Packages.db`,
  `Index.db`), `bdb_ro` (the old BerkeleyDB `Packages`, read-only).
  - Fedora ≥ 33 and RHEL 9: sqlite [NV: dates from changelogs];
  - RHEL 8: bdb;
  - openSUSE: ndb [NV, the spec could not be fetched from src.opensuse.org because of a PoW challenge].
- **sqlite schema** [S `lib/backend/sqlite.cc`]:
  - main table `Packages(hnum INTEGER PRIMARY KEY AUTOINCREMENT, blob BLOB NOT NULL)`, in
    which the blob is an rpm header;
  - index tables named after the tag, e.g. `Name`, `Providename`, `Basenames`, `Dirnames`,
    `Installtid`… Each has the columns `(key TEXT|BLOB, hnum INTEGER → Packages, idx INTEGER)` and
    the indexes `<table>_key_idx`, `<table>_hnum_idx`.
  - When opening RW, rpm sets `journal_mode = WAL` and `SQLITE_FCNTL_PERSIST_WAL`, with
    a comment in the source that this is needed to support unprivileged readers.
    So the `-wal`/`-shm` files stay on disk and a reader without root opens the database
    `SQLITE_OPEN_READONLY`.
  - `busy_timeout` 10 s.
- **Reading without librpm**:
  - Names: `SELECT key FROM Name`. Provides: `SELECT key FROM Providename`. This is enough for
    the core of fact 2.
  - Files: `Basenames` has only base names without the directory, which is not precise enough. Full
    paths come from tags in the blob: `DIRNAMES` (1118), `BASENAMES` (1117), `DIRINDEXES` (1116),
    `NAME` (1000), `PROVIDENAME` (1047), `INSTALLTIME` (1008) [NV: tag numbers from
    `rpmtag.h`, from memory].
  - Header format [S rpm.org format_header]: big-endian, `il` (number of entries), `dl`
    (data size), then `il` × `{tag u32, type u32, offset i32, count u32}`, then the data.
    Types: 6 = STRING, 8 = STRING_ARRAY, 4 = INT32 etc. In the database the blob starts with `il`/`dl`
    **without** the 8-byte magic [NV, check on a fixture]. The documentation warns that
    the "proper" access is the library, so this solution is fragile, but it has been stable for
    years.
- **Dependency**: `rusqlite` with `bundled` (MIT, SQLite is public domain) adds about 1 MB to
  the binary. Alternative: **one** process
  `rpm -qa --qf '[%{NAME}\t%{FILENAMES}\n]'` plus `--qf '[%{NAME}\t%{PROVIDES}\n]'`.
  Cost: about 1–3 s for about 2–3 thousand packages with a file list [NV, not measured, no rpm
  on this machine]. Cache the result keyed by the mtime of `rpmdb.sqlite`/`Packages.db`.
- **ndb (openSUSE)**: an own binary format (`rpmpkg.c`, `rpmidx.c`, `rpmxdb.c`), so here
  a single `rpm -qa --qf` is recommended (rpm is always present, since the manager uses it).
- **rpm keeps no removal log**. `INSTALLTIME` exists only for installed packages.
  Fact 1 comes from dnf or zypp (§2.2, §3).
- **Lock**: `%_rpmlock_path = %{_dbpath}/.rpm.lock`, an fcntl transaction lock [S
  `macros.in`]. Check through `/proc/locks`.

### 2.2 dnf4 history: `/var/lib/dnf/history.sqlite`

- `persistdir` defaults to `/var/lib/dnf` [S dnf `const.py.in`, conf_ref], file
  `history.sqlite` (libdnf swdb).
- **Schema** [S libdnf `transaction/sql/create_tables.sql`]:
  - `trans(id, dt_begin unix, dt_end, rpmdb_version_begin/end, releasever, user_id, cmdline, state)`;
  - `item(id, item_type)`;
  - `trans_item(id, trans_id, item_id, repo_id, action, reason, state)`;
  - `rpm(item_id, name, epoch, version, release, arch)`;
  - `item_replaced_by`, `repo`, `console_output`, comps tables.
- **Actions** [S libdnf `Types.hpp`]: 1 INSTALL, 2 DOWNGRADE, 3 DOWNGRADED, 4 OBSOLETE,
  5 OBSOLETED, 6 UPGRADE, 7 UPGRADED, **8 REMOVE**, 9 REINSTALL, 10 REINSTALLED,
  11 REASON_CHANGE.
- **Query** (sketch): `SELECT r.name, MAX(t.dt_begin) … WHERE ti.action=8` and the last
  `INSTALL`/`UPGRADE`/`REINSTALL` for the same name. The removal is valid when there was no
  later installation and the name is not in the rpmdb. The `trans_item.state` status should be OK
  (enum value [NV]).
- **Note**: OBSOLETED (5) is a replacement by another package. Treat it as a "rename",
  not as a removal.
- **Permissions**: `dnf history` works as a regular user, so the file is most likely 0644 [NV].
- **Text log**: `/var/log/dnf.rpm.log` [S `LOG_RPM`]. Lines
  `%Y-%m-%dT%H:%M:%S%z SUBDEBUG <Action>: <nevra>`, where for a removal the action is `Erase` [S
  `dnf/transaction.py`]. Rotation is **by size** (`log_size`/`log_rotate`), so the
  log is short. A fallback only.

### 2.3 dnf5 history: `/usr/lib/sysimage/libdnf5/transaction_history.sqlite`

- `transaction_history_dir` = `system_state_dir` = `/usr/lib/sysimage/libdnf5`, file
  `transaction_history.sqlite` [S dnf5 `const.hpp`, `db.cpp`, dnf5.conf(5)].
- **Schema** [S dnf5 `create_tables.sql`]:
  - `trans(id, dt_begin, dt_end, …, description, …)`;
  - `trans_item(id, trans_id, item_id, repo_id, action_id, reason_id, state_id)`;
  - `rpm(item_id, name_id → pkg_name.id, epoch, version, release, arch_id → arch.id)`;
  - `pkg_name(id, name)`.
  - Dictionaries stored in the database itself: `trans_item_action` = 1 Install, 2 Upgrade,
    3 Downgrade, 4 Reinstall, **5 Remove**, 6 Replaced, 7 Reason Change;
    `trans_item_state` = 1 Started, 2 Ok, 3 Error.
  - **Read the action names from the `trans_item_action` table, not constants**, as that survives changes.
- Fedora 41+ (dnf5 default) may have **both** databases. The old `history.sqlite` stays after
  the migration from dnf4 [NV], so merge both.
- **dnf5 lock**: `/run/dnf/rpmtransaction.lock` [S `const.hpp`]. dnf4 keeps
  `rpmdb_lock.pid`, `metadata_lock.pid` and `download_lock.pid` in the `locks/` directories [S
  `dnf/lock.py`].
- Other frontends: PackageKit (GNOME Software/Discover) on Fedora uses libdnf (dnf4) or
  libdnf5, so it goes into the same history [NV for the dnf5 version].

### 2.4 Files of a removed package (rpm-md)

- An rpm-md repo has `repodata/*-filelists.xml.gz` (or `.zst`), with entries
  `<package pkgid name arch><version/><file>/usr/bin/x</file>…`.
- dnf5: `optional_metadata_types` defaults to `comps,updateinfo`, so **filelists are not
  downloaded** [S dnf5.conf(5)]. dnf4 (newer versions): an empty list of extra types by default,
  i.e. no filelists either, unless a command asks for them, e.g. `dnf provides /path` [S dnf
  conf_ref].
- Cache: `/var/cache/libdnf5/<repo>-<hash>/repodata/` (dnf5, `system_cachedir`),
  `/var/cache/dnf/<repo>-<hash>/repodata/` (dnf4). For a regular user dnf5 uses
  `~/.cache/libdnf5` [S]. The `.solv`/`.solvx` files are in libsolv's binary format, so we skip them.
- Conclusion: **read filelists if they exist** (a gzip/zstd XML stream, a prefix filter).
  On a typical Fedora they will not be there. Then the package name, btime and XU records remain.

---

## 3. zypper (openSUSE Tumbleweed / Leap / Aeon)

- **History**: `/var/log/zypp/history`, changeable through `history.logfile` in
  `/etc/zypp/zypp.conf` [S openSUSE wiki Libzypp/Package_History].
  - Format: CSV with `|`, one event per line; lines starting with `#` are comments.
  - Actions [S libzypp `HistoryLogData.cc`]: `install`, `remove`, `radd`, `rremove`, `ralias`,
    `rurl`, `command`, `patch`.
  - Fields [S `HistoryLogData.h`]:
    ```
    install: date|install|name|edition|arch|reqby|repoalias|checksum|userdata
    remove : date|remove |name|edition|arch|reqby|userdata
    ```
    In the file the action name is padded with spaces to 7 characters (`"remove "`), so
    trim the field. The date has the format `YYYY-MM-DD HH:MM:SS` [NV date format].
  - `install` also covers upgrades, as zypp logs the new version as an install [NV]. Rule:
    the last event per name wins, as in pacman.
- **Permissions**: [NV]. An openSUSE bug report (bsc#1225962, a thread about rootless) says that
  `/var/log/zypper.log` is not readable for the user. The permissions of `history` must be checked on
  TW. If the file is unreadable, fact 1 does not exist and the directories go to "unknown",
  with a message in the UI.
- **Installed**: rpmdb ndb (§2.1), so a single `rpm -qa --qf`.
- **Filelists**: `/var/cache/zypp/raw/<alias>/repodata/*filelists.xml.*` [NV, whether zypp
  keeps them by default]. Solv cache: `/var/cache/zypp/solv/<alias>/solv` (libsolv, we skip it).
- **Lock** [S `ZYppFactory.cc`]: the `zypp.pid` file in `lockfileDir()`, i.e. `/run/zypp.pid`
  (formerly `/var/run`, [NV] exact directory), and `zypp-rpm.pid`. They contain a PID; the lock
  is active when `/proc/<pid>` exists.
- **Aeon/MicroOS** (transactional-update): package changes go into a new snapshot, but `/var`
  is shared, so `history` is written normally [NV]. A "removed" package takes effect only after
  a reboot, so fact 2 must use the rpmdb of the **current** system.

---

## 4. Other managers (briefly)

| PM | Installed (file, format) | Removal log | Files | Lock | Root |
|---|---|---|---|---|---|
| **xbps** (Void) | `/var/db/xbps/pkgdb-0.38.plist` (XML plist, dictionary `pkgname → {pkgver, provides, …}`) [S `xbps.h.in`] | no default file; xbps writes to syslog [NV] | `/var/db/xbps/.<pkg>-files.plist` [NV name] | `flock` on `/var/db/xbps/lock` [S `pkgdb.c`] | no |
| **apk** (Alpine) | `/lib/apk/db/installed`: blocks `P:name V:ver p:provides F:dir R:file`, an empty line between packages [S `database.c` for the path; field letters NV] + `/etc/apk/world` | none | `F:`/`R:` in `installed` | `/lib/apk/db/lock` [S] | no (0644 [S]) |
| **portage** (Gentoo) | `/var/db/pkg/<cat>/<pf>/` with `CONTENTS` (`obj /usr/bin/x <md5> <mtime>`, `dir`, `sym a -> b <mtime>`), `PROVIDES`, `PF` [NV CONTENTS format] | `/var/log/emerge.log`: `<epoch>:  >>> unmerge success: cat/pkg-ver` [NV text]. **Created 0660 portage:portage** [S `emergelog.py`], so readable only for the portage group | none after unmerge | `lockdir(/var/db/pkg)` → `/var/db/.pkg.portage_lockfile` [S `vartree.py`, `locks.py`] | the log needs the portage group |
| **eopkg** (Solus) | `/var/lib/eopkg/package/<name>-<ver>-<rel>/{metadata.xml,files.xml}` [NV layout] | `/var/lib/eopkg/history/NNN_<op>.xml` with `<Package operation="remove|install|upgrade…"><Name>…</Name><Before/><After/>` [S `history_dir`, `history.py`; file names NV] | none | `lock_dir = /var/lock/subsys` [S], file name [NV] | no [NV] |
| **Nix** | `/run/current-system/sw/{bin,share/applications}`, `/etc/profiles/per-user/$USER`, `~/.nix-profile`, `~/.local/state/nix/profiles` | profile generations, not removal events | `/nix/store/*` (everything stays until GC) | — | — |

Conclusion:
- xbps, apk and Nix provide only fact 2. Without fact 1 their users get leftovers
  only from Flatpak, Snap, XU records and (for Nix) from nothing. This is acceptable: we prefer
  no results to guessing.
- Gentoo: if `emerge.log` is readable (the user in the portage group), treat it like
  pacman.log.

---

## 5. Immutable systems

| System | Host packages | Fact 1 | Notes |
|---|---|---|---|
| Fedora Silverblue/Kinoite, Bazzite, Aurora, Bluefin (rpm-ostree / bootc) | rpmdb in the image's `/usr/share/rpm` (link `/var/lib/rpm`) [S] | no removal log. Layers: the deployment's origin file, keys `requested-packages`, `requested-base-removals` [S rpm-ostree administrator-handbook]. The difference between the previous and current deployment's origin gives only the last change [NV: readability of `/ostree/deploy/*/deploy/*.origin` without root] | user apps are mainly Flatpak, Distrobox/toolbox and Homebrew (`/home/linuxbrew/.linuxbrew/Cellar`, Bazzite/Bluefin) → fact 2. Image changes (rebase) do not go through dnf, so the dnf history is empty or misleading |
| SteamOS 3 | pacman (`/var/lib/pacman/local` on a read-only rootfs) | A/B updates swap the image **without** pacman, so pacman.log does not see image changes [NV] | treat pacman as fact 2; fact 1 from pacman.log only with developer mode unlocked. Main source: Flatpak (Discover) |
| openSUSE Aeon/Kalpa | zypp + transactional-update | `/var/log/zypp/history` (§3) | as above |
| NixOS | §4 | none | skip |

Detection: `/run/ostree-booted` exists for rpm-ostree/bootc [NV];
`/etc/os-release` `ID=steamos`; `VARIANT_ID` on Fedora Atomic. Used for UI messages, not for
choosing the backend.

---

## 6. Flatpak

- **Installations** [S flatpak(1)]: the user's `~/.local/share/flatpak/`, the system one
  `/var/lib/flatpak/`, extra ones in `/etc/flatpak/installations.d/*.conf`. An app is
  installed when `<inst>/app/<id>/current` exists (a link to the active branch)
  [NV, directory layout, matching the implementation in leftovers.rs].
- **Data after uninstalling**: `flatpak uninstall` **leaves** `~/.var/app/<id>`. Only
  `--delete-data` removes it ("Remove app data in ~/.var/app and in the permission store") [S].
  This is the main source of leftovers on Mint, SteamOS and Bazzite.
- **Evidence that the app is gone**: no `app/<id>` in any installation and no process
  (`/proc/*/root` or `flatpak ps`). Already implemented in `flatpak_orphan`.
- **Removal history**: `flatpak history` reads the **systemd journal** [S]. It filters by
  `MESSAGE_ID=c7b39b1e006b464599465e105b361485` [S `flatpak-utils-private.h`] and the fields
  `OPERATION`, `REF`, `INSTALLATION`, `REMOTE`, `COMMIT`, `_SOURCE_REALTIME_TIMESTAMP`
  [S `flatpak-builtins-history.c`].
  - `--user` installations log to the user's journal (readable without privileges [NV]);
    system ones through flatpak-system-helper to the system journal (needs the
    `systemd-journal`/`wheel`/`adm` group [NV: depends on the distribution]).
  - The journal file format is complex, so if at all, then a single process
    `journalctl -o json MESSAGE_ID=… OPERATION=uninstall`. It gives the removal date, i.e. fact 3
    (btime ≤ removal). **Optional**: fact 1 for Flatpak already exists, as the ID is exact.
- Portal permissions: `~/.local/share/flatpak/db/` (permission store), a separate item
  for the report only (as in LEFTOVERS §5.6).

## 7. Snap

- **Behaviour of `snap remove`** [S snapd `overlord/snapstate/backend/snapdata.go`, snapcraft
  docs]:
  - snapd removes the revision data and `common` in `~/snap/<name>` **for all users**
    whose directory is accessible, plus `/root/snap/<name>` and `/var/snap/<name>`; at
    the end it removes the base directory itself;
  - it skips users whose snap directory is inaccessible (e.g. NFS root_squash; [NV]
    encrypted, unmounted HOME);
  - before removing it takes an **automatic snapshot** (without `--purge`), kept for 31 days in
    `/var/lib/snapd/snapshots`.
  - Conclusion: `~/snap/<name>` **usually disappears** after a normal removal. If it remained, either
    the snap is still installed, or HOME was inaccessible during the removal, or
    `os.Remove` of the base directory failed because it was not empty (snapd logs an error).
- **Evidence of absence**: no `/snap/<name>/` (mount points, 0755) and no
  `/var/lib/snapd/snaps/<name>_*.snap`. `state.json` is 0600 root [NV], so we do not
  read it. Classic snaps (`--classic`) may write to `~/.config/<app>`. Add the snap name
  to fact 2 (installed), also from `/var/lib/snapd/desktop/applications/*.desktop`.
- **Removal log**: no file readable for the user. Changes are in
  `/var/lib/snapd/state.json` (root). Fact 1 for snaps: `~/snap/<name>` without
  `/snap/<name>` is a certain orphan from the structure alone (like Flatpak).
- Ubuntu installs Firefox and Thunderbird as snaps. `~/snap/firefox` must be taken into account
  as an installed profile, so that `~/.mozilla` from deb times is not taken for a leftover without
  a check [NV, profile path after the migration].

---

## 8. Limitations and unverified assumptions

- None of these distributions was run. Everything marked [NV] must be checked in a VM
  or container: Debian 13, Ubuntu 24.04/26.04, Fedora 43 (dnf5), openSUSE TW, Silverblue.
  Collect fixtures: `dpkg.log`, `status`, `*.list`, `history.sqlite`,
  `transaction_history.sqlite`, `rpmdb.sqlite`, `zypp/history`.
- In particular to check:
  - the permissions of `/var/log/zypp/history`;
  - the absence of magic in the rpmdb blob;
  - the Contents extension in apt lists;
  - whether dnf5 leaves the old `history.sqlite`;
  - the openSUSE rpm backend (ndb);
  - the readability of the origin in ostree.
- Containers (Distrobox/toolbox) have their own package databases, but a **shared HOME**. An app
  installed in a container and exported (`distrobox-export`) leaves a `.desktop` in
  `~/.local/share/applications` (fact 2 already catches it). A non-exported app from a container
  is "unknown" to the host, which is safe, as nothing is offered without fact 1.
  Container databases can be added later (`~/.local/share/containers/storage/…` → the container's
  rootfs → its `/var/lib/dpkg/status` etc.) [NV].
- Log rotation limits the history: dpkg/apt about 12 months, dnf.rpm.log a few MB.
  Removals older than the log are invisible, as with pacman. The dnf sqlite databases have the full
  history.

## 9. Sources

- dpkg(1), log format, states and `DPKG_FRONTEND_LOCKED`: https://man7.org/linux/man-pages/man1/dpkg.1.html
- dpkg `src/main/remove.c` (`.list` after remove/purge): https://salsa.debian.org/dpkg-team/dpkg/-/blob/main/src/main/remove.c (mirror https://github.com/guillemj/dpkg)
- dpkg logrotate: https://salsa.debian.org/dpkg-team/dpkg/-/blob/main/debian/dpkg.logrotate
- apt history.log (`apt-pkg/deb/dpkgpm.cc`): https://salsa.debian.org/apt-team/apt/-/blob/main/apt-pkg/deb/dpkgpm.cc
- apt logrotate: https://salsa.debian.org/apt-team/apt/-/blob/main/debian/apt.logrotate
- apt-file(1): https://manpages.debian.org/unstable/apt-file/apt-file.1.en.html
- Contents format: https://wiki.debian.org/DebianRepository/Format#A.22Contents.22_indices
- rpm sqlite backend: https://github.com/rpm-software-management/rpm/blob/master/lib/backend/sqlite.cc
- rpm backends: https://github.com/rpm-software-management/rpm/tree/master/lib/backend
- rpm `macros.in` (`_dbpath`, `_rpmlock_path`): https://github.com/rpm-software-management/rpm/blob/master/macros.in
- rpm header format: https://rpm-software-management.github.io/rpm/manual/format_header.html (source `docs/manual/format_header.md`)
- libdnf (dnf4) swdb schema: https://github.com/rpm-software-management/libdnf/blob/dnf-4-master/libdnf/transaction/sql/create_tables.sql
- libdnf actions: https://github.com/rpm-software-management/libdnf/blob/dnf-4-master/libdnf/transaction/Types.hpp
- dnf4 `const.py.in`, `logging.py`, `lock.py`, `transaction.py`: https://github.com/rpm-software-management/dnf/tree/master/dnf
- dnf4 conf_ref (`optional_metadata_types`, `persistdir`): https://github.com/rpm-software-management/dnf/blob/master/doc/conf_ref.rst
- dnf5 history schema: https://github.com/rpm-software-management/dnf5/blob/main/libdnf5/transaction/db/sql/create_tables.sql
- dnf5 `db.cpp`: https://github.com/rpm-software-management/dnf5/blob/main/libdnf5/transaction/db/db.cpp
- dnf5 `const.hpp` (paths, lock): https://github.com/rpm-software-management/dnf5/blob/main/include/libdnf5/conf/const.hpp
- dnf5.conf(5): https://github.com/rpm-software-management/dnf5/blob/main/doc/dnf5.conf.5.rst
- libzypp history: https://github.com/openSUSE/libzypp/blob/master/zypp/zypp/HistoryLogData.h and `HistoryLogData.cc`
- libzypp lock: https://github.com/openSUSE/libzypp/blob/master/zypp/zypp/ZYppFactory.cc
- openSUSE wiki: https://en.opensuse.org/Libzypp/Package_History
- Report about zypper log permissions: https://lists.opensuse.org/archives/list/bugs@lists.opensuse.org/thread/7237YRGFL7GO4ZZWHWQ2IYIHZQWQ6O3L
- rpm-ostree tmpfiles: https://github.com/coreos/rpm-ostree/blob/main/src/app/rpm-ostree-0-integration.conf
- rpm-ostree origin: https://github.com/coreos/rpm-ostree/blob/main/docs/administrator-handbook.md
- xbps: https://github.com/void-linux/xbps/blob/master/include/xbps.h.in, https://github.com/void-linux/xbps/blob/master/lib/pkgdb.c
- apk-tools: https://gitlab.alpinelinux.org/alpine/apk-tools/-/blob/master/src/database.c
- portage: https://github.com/gentoo/portage/blob/master/lib/_emerge/emergelog.py, https://github.com/gentoo/portage/blob/master/lib/portage/dbapi/vartree.py, https://github.com/gentoo/portage/blob/master/lib/portage/locks.py
- eopkg: https://github.com/getsolus/eopkg/blob/master/pisi/configfile.py, https://github.com/getsolus/eopkg/blob/master/pisi/history.py
- Flatpak: https://docs.flatpak.org/en/latest/flatpak-command-reference.html, https://github.com/flatpak/flatpak/blob/main/common/flatpak-utils-private.h, https://github.com/flatpak/flatpak/blob/main/app/flatpak-builtins-history.c
- Snap: https://snapcraft.io/docs/how-to-guides/manage-snaps/create-data-snapshots, https://github.com/canonical/snapd/blob/master/overlord/snapstate/backend/snapdata.go
- `/proc/locks`: proc(5) https://man7.org/linux/man-pages/man5/proc_locks.5.html (0444 permissions checked on this machine)
