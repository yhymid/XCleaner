# System (root) cleaning on all distributions — research for v0.2.0 (2026-11)

**English** | [Polski](DISTRO-ROOT-CLEANING-2026-11.pl.md)

Context: today `root.rs` handles only Arch (coredumps, paccache, orphaned
`/usr/lib/modules/<ver>`, abandoned `download-*`, sync DBs of removed repos). The GUI runs
`sudo xcleaner --root-clean <tasks>` in a terminal, root works out the targets itself, and every task
has a read-only probe without root.

Markers: **(source)** = checked in a man page, documentation or upstream code (URLs at the end).
**(obs.)** = checked on the dev machine (CachyOS, systemd 262, flatpak 1.18.4).
**(unverif.)** = general knowledge, check in a VM in v0.3.0.
GPL code (BleachBit, Stacer) was neither read nor copied.

## 0. General rules (conclusions)

1. **Always call the distribution's native tool** (`apt-get`, `dnf`, `zypper`, `xbps-remove`,
   `apk`, `eclean`, `flatpak`, `snap`, `journalctl`, `systemd-tmpfiles`). A manual `unlink` is done
   only where there is no tool (coredumps, crash reports, rotated logs, orphaned
   module directories). The tools take their own locks, and their own dry-run gives
   an exact probe.
2. **Locks are mostly fcntl/flock locks, not the presence of a file.** The files
   `/var/lib/dpkg/lock*`, `/var/db/xbps/lock` and `/lib/apk/db/lock` always exist. An Arch-style
   check (`db.lck` exists → stop) **does not work** outside pacman. Strategy:
   - on the root side: let the tool try itself (without waiting) and report an error;
   - optionally before that: `fcntl(F_OFD_GETLK)` / `flock(LOCK_EX|LOCK_NB)` on the open file
     (only as root, as the files are often 0640). See the table in §9.
3. **A probe without root** works when the directory has the `x` right (stat of entries) and `r` (listing).
   Most cache directories are 0755. Exceptions are described below. If the probe gets `EACCES`,
   the GUI shows "size unknown (needs root)" instead of 0.
4. **We never automate a full `autoremove`** (apt/dnf/zypper/xbps `-o`/pamac `-o`).
   Details in §2.
5. **Immutable systems** (ostree, SteamOS, NixOS, MicroOS) have a separate, heavily reduced
   task set (§7). Detection happens before anything else.

## 1. Package caches

| Manager | Command (root) | Dry-run / probe | Paths | Lock | Risk |
|---|---|---|---|---|---|
| apt (Debian, Ubuntu, Mint, Pop!_OS, Zorin, MX, elementary) | `apt-get autoclean` (default); `apt-get clean` (opt-in) | `apt-get -s autoclean` prints `Del <pkg> <ver> [<size>B]` and works without root (simulation disables locks) **(source: apt code)**. The size for `clean` is counted by stat of `archives/*.deb` | `/var/cache/apt/archives/*.deb`, `archives/partial/`, `/var/lib/apt/lists/partial/`, `/var/cache/apt/pkgcache.bin`, `srcpkgcache.bin` | `archives/lock` (fcntl, taken by apt); dpkg `lock-frontend` | low |
| dnf4 (Fedora ≤40, RHEL derivatives) | `dnf clean packages` | stat `/var/cache/dnf/*/packages/*.rpm` | `/var/cache/dnf` | `ProcessLock` (`*_lock.pid` + flock) in dnf's lock directory **(source)** | low |
| dnf5 (Fedora 41+) | `dnf5 clean packages` | stat `/var/cache/libdnf5/*/packages/*.rpm` | `system_cachedir=/var/cache/libdnf5` **(source)** | dnf5 locks itself (path **unverif.**) | low |
| zypper (openSUSE) | `zypper --non-interactive clean` (packages only). No `--all`/`--metadata`, as those also remove metadata and the next run has to download it again | stat `/var/cache/zypp/packages/<alias>/**.rpm` | `/var/cache/zypp/packages` (`packagesdir`) **(source)** | `/run/zypp.pid` (PID in the file, not mere presence) **(source)**; exit 7 = ZYPP locked (e.g. PackageKit) | low |
| xbps (Void) | `xbps-remove -O` (obsolete); `-OO` (also uninstalled, opt-in) | `xbps-remove -On` (`--dry-run`) **(source)** | `/var/cache/xbps` | flock `/var/db/xbps/lock`; xbps **waits** for the lock ("package database locked, waiting...") **(source: lib/pkgdb.c)**, so root must check `LOCK_NB` itself beforehand | low |
| apk (Alpine) | `apk cache clean` (packages outside the indexes); `apk cache purge` opt-in | `apk cache clean --simulate` **(source)** | `/etc/apk/cache` → usually a symlink; cache **only when enabled** | flock `/lib/apk/db/lock`, `LOCK_NB` without `--wait` → error **(source: database.c)** | low |
| portage (Gentoo) | `eclean distfiles` (default mode = keeps files of ebuilds present in the repo); `eclean packages` | `eclean --pretend distfiles` **(source)** | `DISTDIR=/var/cache/distfiles`, `PKGDIR=/var/cache/binpkgs` (from `make.conf`) | none in the docs; do not run while `emerge` runs (check the process) **(unverif.)** | low; `--deep` = medium (loses the downgrade option), opt-in only |
| pacman/pamac (Manjaro) | as on Arch: `paccache` (pacman-contrib); alternative `pamac clean -k N` / `-u` / `-b` | `pamac clean --dry-run` **(source: pamac.8)** | `/var/cache/pacman/pkg`; AUR builds: `BuildDirectory=/var/tmp` (default pamac.conf) **(source)** | `db.lck` as today | low |

Notes:
- **apt `clean` vs `autoclean`:** `clean` removes everything from `archives/` and `archives/partial/`
  except the lock, clears `lists/partial/` and deletes `pkgcache.bin` and `srcpkgcache.bin`,
  which apt rebuilds on the next run (verified in `apt-private/private-download.cc`:
  `CleanDownloadDirectories` → `pkgCacheFile::RemoveCaches()`). `autoclean` removes only
  .debs that can no longer be downloaded. **Do not use `apt-get distclean`**, as it also deletes
  the package lists. The `apt` binary deletes .debs after a successful install by default
  (`Binary::apt::APT::Keep-Downloaded-Packages "0"`, since apt 1.2), while `apt-get` does not.
  So the cache is often small, unless someone uses `apt-get`, Synaptic or unattended-upgrades.
- **apt permissions:** `archives/` is 0755, but `archives/partial/` belongs to `_apt` with 0700
  (apt sandbox) **(unverif.)**. The probe then counts `partial/` as unknown.
- **dnf `keepcache`** defaults to `False` (dnf4 and dnf5) **(source)**: packages are deleted
  after a successful transaction. The cache grows only after interrupted transactions, with `keepcache=1`
  or with `dnf download`/`system-upgrade`. In the GUI this is usually a few MB, but the task is still worth having.
  In addition: `dnf system-upgrade clean` (leftovers of a Fedora version upgrade,
  `/var/lib/dnf/system-upgrade`) **(unverif.)**.
- **zypper:** packages go to the cache only for repos with `keeppackages=1` (`zypper mr -k`) **(source)**.
  This is off by default **(unverif.)**.
- **Manjaro:** `pamac clean` works through pamac-daemon (polkit), i.e. against the rule
  "sudo in a terminal, no polkit". Recommendation: on Manjaro use the same path as on Arch
  (`paccache`). The `pacman-contrib` package may not be installed: then the task is
  hidden with a hint **(unverif. for the default ISO)**. A separate low-risk task:
  `/var/tmp/pamac-build-<user>` (leftovers of AUR builds), only when pamac is not running.
- **EndeavourOS and CachyOS** = Arch (no changes). EndeavourOS has its own `paccache` hook and
  "eos-…" tools, but XCleaner needs none of them.

## 2. Old kernels

**General recommendation:** XCleaner **does not delete kernel files or `/boot` by hand**.
On Debian/Ubuntu `/boot/vmlinuz-*` and `initrd.img-*` belong to packages or are
generated by hooks (`/etc/kernel/postrm.d`, initramfs-tools, update-grub). Deleting them by hand
leaves GRUB entries, gets dpkg out of sync and breaks initramfs generation at the next update
(unverif. in detail, but it follows from the dpkg and hook model).
Kernels are removed only with the package manager, with a dry-run shown in the GUI.

| Distro | Upstream mechanism | What XCleaner does | Probe | Risk |
|---|---|---|---|---|
| Debian, Ubuntu, Mint, Pop!_OS, Zorin, MX, elementary | `apt autoremove --purge`. Apt protects kernels: `APT::Protect-Kernels=true`, the running and the newest kernel are protected, `APT::NeverAutoRemove::KernelCount` = 2 **(source: apt.conf(5), apt test)** | **Not** the whole `autoremove`. Instead: (1) `apt-get -s autoremove --purge` (no root) → (2) filter to `linux-image-*`, `linux-headers-*`, `linux-modules-*`, `linux-modules-extra-*` with a version ≠ `uname -r` and no meta packages (`linux-image-amd64`, `linux-generic` etc.) → (3) root runs `apt-get -s purge <list>` and **stops** if the plan contains anything outside the list → (4) `apt-get -y purge <list>` | the `Remv` list from the simulation + `dpkg-query -W -f='${Installed-Size}'` (KiB) | medium (touches the bootloader through hooks), but protected by apt; never selected by default |
| Fedora (dnf4/dnf5), Nobara | `installonly_limit=3` (min. 2), `protect_running_kernel=True` **(source)**; `dnf remove --oldinstallonly` removes those above the limit, keeping the running kernel **(source)** | Root: `dnf remove --oldinstallonly --assumeno` (show the plan) → when the plan contains only `kernel*` packages and their `installonly` → `dnf -y remove --oldinstallonly`. Note: dnf5 has `clean_requirements_on_remove=True` **(source)**, so the plan may contain dependencies and must be filtered | without root: `rpm -q kernel-core --qf ...` + `uname -r` + `installonly_limit` from `/etc/dnf/dnf.conf` (heuristic) | low/medium; dnf watches the limit itself, so an excess appears rarely (usually after raising the limit) |
| openSUSE Tumbleweed/Leap | `zypper purge-kernels` by `multiversion.kernels` (vendor: `latest,latest-1,running`) **(source: zypp.conf(5))**; there is also `purge-kernels.service` at boot **(unverif.)** | Root: `zypper purge-kernels --dry-run` → `zypper -n purge-kernels` | the dry-run needs root (zypp lock) **(unverif.)**; without root: `rpm -q kernel-default` | low (in line with the distro policy) |

**Is `apt autoremove` safe to automate? No.** It removes *all*
packages flagged "auto" that nothing requires. After an `apt remove` of a desktop meta package
(e.g. `ubuntu-desktop`) or after a wrong apt-mark it can take down half the system
(a known pattern in bug reports, unverif. with a specific link). The same applies to `dnf autoremove`, `xbps-remove -o`
and `pamac remove -o`. XCleaner may at most **show** it (a list from the simulation, "Review first")
and should never run it in bulk.

**Orphaned module directories** (the counterpart of today's Arch task):
- Debian/Ubuntu: `/usr/lib/modules/<ver>` (with usrmerge `/lib` → `/usr/lib`). **Trap:** dpkg
  does not know the usrmerge aliases. Ask about both paths: `dpkg-query -S /lib/modules/<ver>` and
  `dpkg-query -S /usr/lib/modules/<ver>` (kernel packages still often declare `/lib/...`) **(unverif.)**.
  Typical source of orphans: DKMS and `linux-modules-extra` after a purge.
- Fedora/openSUSE: `rpm -qf /usr/lib/modules/<ver>` → "not owned". No root needed, as the rpmdb is readable.
- Conditions as today: no owner, ≠ `uname -r`, no `vmlinuz`/`modules.dep` owned by a package.
  Low risk; leave as "Review first" when the directory holds `updates/dkms` files.

## 3. Crash dumps

| Source | Path | Upstream cleanup | XCleaner | Risk |
|---|---|---|---|---|
| systemd-coredump (Arch, Fedora, openSUSE, Debian with the package) | `/var/lib/systemd/coredump/core.*` | tmpfiles `d /var/lib/systemd/coredump 0755 root root 2w` (upstream 2 weeks; CachyOS overrides to 3d) **(obs.)**; limits `MaxUse=`/`KeepFree=` in coredump.conf | the same as today; probe: stat (directory 0755) **(obs.)**. `coredumpctl` has no delete command **(source)** | low |
| apport (Ubuntu, Mint, Pop, Zorin, elementary) | `/var/crash/*.crash`, `*.upload`, `*.uploaded` | `/etc/cron.daily/apport`: removes empty files and files >7 days, as well as `/var/crash/YYYYMMDDHHMM` directories >7 days **(source: canonical/apport)** | regular `*.crash`/`*.upload*` files of any age; probe stat. Open files (whoopsie/apport while writing) are skipped | low |
| kdump-tools (Debian/Ubuntu) | `/var/crash/<YYYYMMDDHHMM>/dump.*`, `dmesg.*` | as above (apport cron) | "Review first", never by default: a vmcore is evidence when debugging a panic | medium |
| kdump (Fedora/RHEL) | `/var/crash/<ip>-<date>/vmcore` (default `path` in kdump.conf) **(unverif.)** | no automation | "Review first" | medium |
| ABRT (Fedora, older/optional) | `DumpLocation=/var/spool/abrt` (default), limit `MaxCrashReportsSize=5000` MiB **(source: abrt.conf.txt)**; `/var/tmp/abrt` (unverif.) | ABRT trims to the limit itself | prefer `abrt remove`/`abrt-cli rm <dir>`; fallback: problem directories in the spool when `abrtd` is not running | low |

## 4. journald and /var/log

- `journalctl --vacuum-time=` / `--vacuum-size=` / `--vacuum-files=` affect **only archived
  files**. Active files stay, unless `--rotate` was given first **(source)**.
  **There is no dry-run.** Probe: `journalctl --disk-usage` (archived + active).
- Who can read: a user sees their own journals, and the `systemd-journal`, `adm` and `wheel` groups
  see everything **(source)**. On the dev machine `--disk-usage` works without root (user in wheel) **(obs.)**.
  Fallback without the groups: stat of the `/var/log/journal/<machine-id>/*.journal` files. The directory is 2755
  with a `wheel`/`adm` r-x ACL **(obs.)**, so stat usually succeeds (unverif. on Debian, where
  the user is not in `adm`). To estimate the vacuum: the sum of `*@*.journal` files (archived)
  older than the threshold (mtime).
- Defaults: `Storage=` has a compile-time value (the systemd 262 page says `persistent`).
  In `auto` mode the journal is persistent only when `/var/log/journal` exists. `SystemMaxUse`
  defaults to 10% of the FS (max 4G), `MaxRetentionSec=0` (off) **(source)**. Distributions
  override it (CachyOS: `SystemMaxUse=50M` in `journald.conf.d` **(obs.)**). With volatile
  (`/run/log/journal`) the task is hidden.
- **Task:** `journalctl --vacuum-time=<N>` (4 weeks by default, selectable in the GUI), without
  `--rotate`. Never `--vacuum-size=1` or "delete everything" (as in the ROADMAP). Risk: low
  (we lose log history, which can hinder a diagnosis).

**Rotated logs in /var/log:**
- logrotate deletes old versions itself (`rotate N`, `maxage`). After old files are removed
  by hand, logging carries on correctly **(source: logrotate(8), the -f option)**. Removing
  *rotated* files is therefore safe; **active ones are never touched**.
- Matching (regular files only, `nlink==1`, no symlinks, not in `/var/log/journal`):
  `*.[0-9]`, `*.[0-9].gz|.xz|.zst|.bz2`, `*-YYYYMMDD` / `*-YYYYMMDD.gz` (`dateext`, the default
  on Fedora/openSUSE **(unverif.)**), `*.old`. A bare `*.gz` without a number/date is not matched,
  as it may be an app's active log.
- Exclusions / "Review first": `/var/log/audit/` (rotated by auditd, audit requirements),
  `wtmp.1`/`btmp.1` (login history; `btmp` is evidence of break-in attempts), `installer/` (Debian/Ubuntu,
  installer logs), `anaconda/` (Fedora).
- Probe: the `/var/log` directory is 0755, files are often 0600, but **stat works** (size without reading
  the content) **(obs.: `boot.log.N` 0600 root)**. Risk: low.

## 5. /tmp and /var/tmp

- Upstream `tmp.conf`: `q /tmp 1777 root root 10d`, `q /var/tmp 1777 root root 30d` **(obs.)**.
  Age is counted from atime, mtime, ctime and btime (default `abcmABM`), and a newer value of any of them
  blocks deletion. Entries with an existing flock are skipped together with their subtree. `x`/`X` exclude
  paths **(source: tmpfiles.d(5))**. `systemd-tmpfiles-clean.timer` runs this daily.
- `systemd-tmpfiles --clean --dry-run` exists since v256, and `-n` since v262 **(source)**. This gives an exact probe
  (but only for root, as it must walk whole trees of other users).
- **Recommendation:** a root task `tmpfiles-clean` = `systemd-tmpfiles --clean` (exactly what
  the timer does, with the same exclusions and locks). Probe: `--dry-run` if systemd ≥256,
  otherwise the size is unknown. **No own deletion of /var/tmp as root**: it holds
  `systemd-private-*` (PrivateTmp of running services), sockets, open files, build directories.
  The gain is small, since the timer runs daily anyway. The bigger gain is the **user's** files in `/var/tmp`
  (e.g. `pamac-build-<user>`, `flatpak-cache-*`) in the HOME phase (uid = owner, age > 30 d, no flock).
- `/tmp` is usually tmpfs (Fedora, Arch, Debian 13+, unverif. for all). Not touched.
- On systems without systemd (Void with runit, Alpine with OpenRC, Gentoo with OpenRC) cleaning `/tmp`
  depends on init/cron. Skipped.

## 6. Flatpak and Snap

**Flatpak**
- `flatpak uninstall --unused` removes unused refs (runtimes/extensions without apps).
  Pinned ones (`flatpak pin`) stay (unverif. in the docs; pin is described as "prevent automatic removal").
  The `-y`/`--noninteractive` options exist, and `--dry-run` **does not exist** (man 1.18) **(obs.)**.
- **Probe without root:** `flatpak uninstall --unused --system` prints the list and asks `[Y/n]`, so
  giving `n` on stdin cancels the operation **(obs.: 5 unused refs, e.g. org.gnome.Platform 51)**.
  Parsing that output is enough (`LC_ALL=C`, as the output is localised).
  A non-interactive alternative: the libflatpak API `flatpak_installation_list_unused_refs()`
  (unverif. version it was added in). Size: `flatpak info --show-size <ref>` or stat of the
  `/var/lib/flatpak/runtime/<id>/<arch>/<branch>` directory (this is a shared ostree repo, so the size
  is **approximate**, as objects may be shared).
- Execution: system = root `flatpak uninstall --unused --system -y --noninteractive`;
  user = no root `flatpak uninstall --unused --user -y` (HOME phase). Risk: low.
  Lock: flatpak locks the repo itself, so no manual check is needed.
- `flatpak repair` (`--dry-run` exists) prunes objects without refs and reinstalls
  damaged ones **(obs.: man)**. Do not offer it as "cleaning", as it may download from the network. At most
  as a repair action, outside v0.2.
- `/var/lib/flatpak/.removed/`: deploys uninstalled while the app was running go here.
  `flatpak_dir_cleanup_removed()` deletes them during later operations, but only when the deploy
  is not locked (`deploy_dir_is_locked`) **(source: flatpak-dir.c)**. **(obs.: 142 MB after the epicgames
  launcher.)** Recommendation: do not delete by hand. Calling a flatpak operation (e.g. the
  `--unused` task) with the app closed is enough. Probe: stat `.removed/*` (0755) **(obs.)**.

**Snap (Ubuntu)**
- `refresh.retain`: by default **2 on classic**, 3 on Ubuntu Core, range 2–20 **(source: snapd
  overlord/snapstate/snap.go, configcore/refresh.go)**.
- Disabled revisions: `snap list --all` (Notes column = `disabled`), removal through
  `snap remove <name> --revision=<rev>`. Size: stat `/var/lib/snapd/snaps/<name>_<rev>.snap`
  (no root, unverif. permissions). Low risk: we lose the `snap revert` option. Lock: snapd itself
  serialises changes, and on a conflict the command returns an error. Snaps with an active change
  (`snap changes`) must be skipped.
- `/var/lib/snapd/cache/`: the files are **hardlinks** to the snaps in `snaps/` (content cache). A file
  with `nlink>1` frees no space. snapd itself removes "unreferenced" ones (nlink==1), at most 5,
  older than 30 days **(source: snapd store/cache.go)**. **Do not touch.** At most show the sum of files
  with nlink==1 as information.

## 7. Immutable systems

| System | Detection | What to do | What not to do |
|---|---|---|---|
| Fedora Silverblue/Kinoite, Bazzite, Aurora, Bluefin (rpm-ostree/bootc) | `/run/ostree-booted` exists | `rpm-ostree cleanup -b` (interrupted operations, "use this first") and `-m` (repomd cache) **(source: rpm-ostree(1))**; flatpak `--unused` | `-r` (rollback) and `-p` (pending) only as an opt-in with a warning (you lose the way back); do not touch `/usr`, `/sysroot` or the package cache |
| SteamOS 3 | `ID=steamos` in os-release; read-only root (A/B) **(unverif.)** | flatpak `--unused`, journald vacuum, coredumps (`/var` is writable) **(unverif.)** | no pacman tasks (paccache, modules, sync DB): the rootfs is read-only and overwritten on update |
| NixOS | `/etc/NIXOS` or `ID=nixos` | at most information: `nix-collect-garbage --delete-older-than 30d` (opt-in). Docs: "Deleting previous configurations makes rollbacks to them impossible" **(source)**. Probe: `nix-store --gc --print-dead` (unverif. cost) | `-d` by default; manual deletion in `/nix/store`; bootloader entries disappear only after `nixos-rebuild boot/switch` **(unverif.)** |
| openSUSE MicroOS/Aeon/Kalpa (transactional-update) | `/usr/sbin/transactional-update` + ro btrfs root | flatpak, journald | `zypper` on the live system; deleting snapper snapshots (that is the rollback mechanism; snapper has its own `cleanup` by `NUMBER_LIMIT`) |
| openSUSE TW/Leap with snapper | `/.snapshots` + `snapper` | show the information "snapper snapshots take X" (`snapper list` needs root or ALLOW_USERS) | delete snapshots |

## 8. Privilege escalation without polkit

| Mechanism | Where by default | Notes |
|---|---|---|
| `sudo` | Arch derivatives (wheel), Ubuntu and derivatives (`sudo` group), Fedora (wheel), openSUSE (sudo asks for the **root** password, `Defaults targetpw`, unverif.), Void (installed, but the `%wheel` rule commented out **(source)**) | used today; sets `SUDO_UID` |
| `su -c` | Debian, when a root password was set during installation: then "sudo does not get installed" and the user is not added to the `sudo` group **(source: wiki.debian.org/sudo)** | `su` passes the command to root's shell as **one string**, so safe quoting is needed (the AppImage path may contain spaces). Does not set `SUDO_UID` |
| `doas` | Alpine (sudo replacement since 3.15/3.16; `setup-user -a` adds to wheel and installs doas; rules in `/etc/doas.d/*.conf`) (unverif.: the Alpine wiki returns 403, knowledge from search results); optionally Void/Gentoo/Arch | OpenDoas sets `DOAS_USER` (a name, not a UID) **(unverif.)** |
| `run0` | systemd ≥256 **(source)** | **Needs polkit** (authentication through polkit; without polkitd PID1 rejects an unprivileged user, unverif.), so it breaks the "no polkit" rule. At most a last fallback when polkitd is running. Its own pty. Does not set `SUDO_UID` (not in man 262) **(obs.)** |
| `pkexec` | almost everywhere with polkit | polkit, so out (it has a text agent as a fallback **(source)**, but that is still polkit) |

**Detection (proposal):**
1. `geteuid()==0` → the GUI refuses as today.
2. `sudo` in `PATH` (trusted: `/usr/bin`, `/bin`) **and** the user in the `wheel`/`sudo`/`admin` group
   (getgroups). `sudo -n true` without a password says nothing, and `sudo -l` asks for the password, so the groups decide.
   On openSUSE and Void sudo may still ask for the root password or refuse; the error message must be shown.
3. `doas` in PATH and `/etc/doas.conf` or `/etc/doas.d/` exists (files often 0600, so do not parse them).
4. Otherwise `su -c` (root password).
5. `run0` only when explicitly chosen in the settings.
6. Always: a switch in the settings (sudo/doas/su/run0) as a manual override.

**Required code change:** `run_root()` needs `SUDO_UID` today. For doas/su/run0 pass the
UID explicitly (e.g. `--uid <n>` added by the GUI). This is safe, as the process is already root after
authentication, and the uid is used only to check the owner of the report file. Order:
`SUDO_UID` → `PKEXEC_UID` → `--uid` → `DOAS_USER` (getpwnam).

## 9. Locks: summary

| Tool | File | Type | Detection before start (root) |
|---|---|---|---|
| pacman | `/var/lib/pacman/db.lck` | file presence | as today |
| apt/dpkg | `/var/lib/dpkg/lock-frontend`, `/var/lib/dpkg/lock`, `/var/cache/apt/archives/lock`, `/var/lib/apt/lists/lock` | fcntl (whole file) **(source: dpkg frontend.txt, apt)** | `F_OFD_GETLK`/`F_GETLK` on `lock-frontend`; apt-get fails with an error anyway (exit 100) |
| dnf4 | `*_lock.pid` in dnf's lock directory | PID + flock **(source: dnf/lock.py)** | leave it to dnf (exit 200 = lock problem) **(source)** |
| dnf5, rpm | (unverif.) / `.rpm.lock` in the rpmdb (unverif.) | — | leave it to the tool |
| zypper | `/run/zypp.pid` | PID in the file **(source)** | read the PID, check `/proc/<pid>`; exit 7 = locked |
| xbps | `/var/db/xbps/lock` | flock, **waits** **(source)** | `flock(LOCK_EX|LOCK_NB)` before starting, otherwise the terminal hangs |
| apk | `/lib/apk/db/lock` | flock, no waiting **(source)** | leave it to apk |
| PackageKit | holds the locks of the above | — | additionally a message "Close the Software manager / Discover" |

## 10. Sources

- apt-get(8): https://manpages.debian.org/trixie/apt/apt-get.8.en.html
- apt.conf(5): https://manpages.debian.org/trixie/apt/apt.conf.5.en.html
- apt code `CleanDownloadDirectories`: https://salsa.debian.org/apt-team/apt (mirror https://github.com/Debian/apt, `apt-private/private-download.cc`)
- apt kernel protection test: https://github.com/Debian/apt/blob/main/test/integration/test-kernel-helper-autoremove
- Keep-Downloaded-Packages: https://bugs.launchpad.net/bugs/1970110 , https://lists.debian.org/deity/2020/01/msg00046.html
- dpkg frontend locking: https://sources.debian.org/src/dpkg/1.17.12/doc/frontend.txt , https://bugs.launchpad.net/bugs/1781169
- dnf5 clean: https://dnf5.readthedocs.io/en/latest/commands/clean.8.html ; dnf5.conf: https://dnf5.readthedocs.io/en/latest/dnf5.conf.5.html ; dnf5 remove: https://dnf5.readthedocs.io/en/latest/commands/remove.8.html
- dnf4 conf/command ref: https://dnf.readthedocs.io/en/latest/conf_ref.html , https://dnf.readthedocs.io/en/latest/command_ref.html ; lock: https://github.com/rpm-software-management/dnf/blob/master/dnf/lock.py
- zypper(8): https://man.archlinux.org/man/zypper.8.en ; zypp.conf(5): https://github.com/openSUSE/libzypp/blob/master/zypp/doc/zypp.conf.5.txt ; lock: https://github.com/openSUSE/libzypp/tree/master/maintainer-docs
- xbps-remove(1): https://man.voidlinux.org/xbps-remove.1 ; lock: https://github.com/void-linux/xbps/blob/master/lib/pkgdb.c
- apk-cache(8): https://man.archlinux.org/man/apk-cache.8.en ; lock: https://gitlab.alpinelinux.org/alpine/apk-tools/-/blob/master/src/database.c
- eclean: https://wiki.gentoo.org/wiki/Eclean
- pamac(8), pamac.conf(5): https://github.com/manjaro/pamac-cli/tree/master/data/doc ; default pamac.conf: https://github.com/manjaro/libpamac/blob/master/data/config/pamac.conf
- journalctl(1): https://www.freedesktop.org/software/systemd/man/latest/journalctl.html ; journald.conf(5): https://www.freedesktop.org/software/systemd/man/latest/journald.conf.html
- tmpfiles.d(5): https://www.freedesktop.org/software/systemd/man/latest/tmpfiles.d.html ; systemd-tmpfiles(8): https://www.freedesktop.org/software/systemd/man/latest/systemd-tmpfiles.html
- coredumpctl(1): https://www.freedesktop.org/software/systemd/man/latest/coredumpctl.html
- run0(1): https://www.freedesktop.org/software/systemd/man/latest/run0.html ; pkexec(1): https://man.archlinux.org/man/pkexec.1.en
- apport cron: https://github.com/canonical/apport/blob/main/etc/cron.daily/apport
- ABRT abrt.conf: https://github.com/abrt/abrt/blob/master/doc/abrt.conf.txt
- logrotate(8): https://manpages.debian.org/trixie/logrotate/logrotate.8.en.html
- flatpak: https://docs.flatpak.org/en/latest/flatpak-command-reference.html ; `.removed`: https://github.com/flatpak/flatpak/blob/main/common/flatpak-dir.c
- snapd retain/cache: https://github.com/canonical/snapd/blob/master/overlord/snapstate/snap.go , https://github.com/canonical/snapd/blob/master/overlord/configstate/configcore/refresh.go , https://github.com/canonical/snapd/blob/master/store/cache.go
- rpm-ostree(1): https://github.com/coreos/rpm-ostree/blob/main/man/rpm-ostree.xml
- nix-collect-garbage: https://nix.dev/manual/nix/2.28/command-ref/nix-collect-garbage
- Debian sudo: https://wiki.debian.org/sudo ; Void: https://docs.voidlinux.org/config/users-and-groups.html ; Alpine doas (403 when fetching, quoted from search results): https://wiki.alpinelinux.org/wiki/Setting_up_a_new_user
- Unavailable during the research (403/404/login): the Alpine wiki, en.opensuse.org SDB, Fedora docs (Anubis), the snap documentation (readthedocs login), the Manjaro wiki. Conclusions from them are marked (unverif.) or based on source code.
