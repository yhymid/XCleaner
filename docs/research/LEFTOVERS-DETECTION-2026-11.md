# Leftovers of uninstalled applications in HOME — detection design (2026-11-22)

**English** | [Polski](LEFTOVERS-DETECTION-2026-11.pl.md)

Owner question (PL, paraphrased): "Uninstallers never remove everything. Leftovers of removed apps
always stay in `~/.config` and `~/.local`. How can we check, or prove, which dirs are leftovers?"

Scope: `~/.config/*`, `~/.local/share/*`, `~/.local/state/*`, `~/.var/app/*`, top-level `~/.*`.
Arch/CachyOS, Phase 1 (HOME only, never root). UI label: **"Pozostałości po odinstalowanych aplikacjach"**.
Nothing here deletes anything. Preselection policy is the owner's decision (see §8).

Method: web research (URLs below, pages fetched 2026-11-22), a study of XUninstaller's identity engine
(MIT, same owner), and a read-only measurement on this machine (names, sizes, mtime/atime/btime;
no personal file contents were read). The measurement script is
`/tmp/claude-1000/xc-leftover/measure.py` (temporary, not in the repo).

---

## 0. TL;DR

- **No upstream tool solves this.** Every tool found is either a curated "program → paths"
  list (xdg-ninja, antidot, xdg-unused-data, BleachBit) or a system-only unowned-file lister
  (lostfiles, pacreport). Arch Wiki: "*Pacman will not remove configurations that the
  application itself creates (for example "dotfiles" in the home directory).*" A forum moderator
  adds: "*Generally having a generic command for this is borderline impossible*".
- **Flatpak is the one exception.** `~/.var/app/<id>` is keyed by an exact app ID, so
  "no installed ref with this ID in any installation" counts as proof. Flatpak defines it the
  same way: `flatpak uninstall --delete-data` without a REF deletes all "unowned" data
  (source checked below).
- **What gives near-proof on Arch:** three independent facts that agree:
  1. **Name:** the entry name, normalized, equals an identity of a package whose *last*
     `pacman.log` event is `removed`. The identity can be the package name, a binary, a
     desktop ID or a reverse-DNS leaf. Binaries and desktop IDs come from the sync files DB
     (`pacman -Fl`) or from XUninstaller records.
  2. **No owner:** no installed thing claims the name. That means a package, a binary in PATH
     (owned or not), a desktop ID, WMClass, a Flatpak, an AppImage or a running process.
  3. **Time:** the dir's **birth time (statx btime)** falls seconds after that package's
     `installed` line, and nothing was written to it after the removal.
     On this machine 13 of the 23 removed-package matches were born **3 s to 28 min** after
     the install (discord +5 s, chromium +4 s, go +3 s, BSManager +30 s).
- **Measured here:** 218 entries.
  - 23 match removed packages. Their names are all correctly attributed, but one turned out
    to be in use (see the next bullet).
  - Leftovers total about **9.5 GB**: BSManager 5.4 GB, pnpm 3.0 GB, discord 708 MB,
    chromium profile + psd backup 342 MB, shadPS4 66 MB, and others.
  - About 3 GB more looks like leftovers but is only REVIEW grade: osu-winello 3.1 GB and
    Epic `com.epicgames.eglv2` 2.5 GB.
  - 45 entries stay **unknown**. They come from AppImages, manual installs, games and vendor
    dirs. They must never be offered as leftovers.
- **Traps we hit:**
  - `Discord Key Storage` matches removed `discord`, but it was **born after** the removal
    and belongs to the installed `discord-canary`.
  - Four browser dirs (`BraveSoftware`, `microsoft-edge`, `opera`, `vivaldi`) exist for
    browsers that were never installed. Claude Code's native-messaging installer creates
    them.
  - `atime` is useless. The mount is `relatime`, and every scan updates the dir atime: all
    218 entries show 2026-11-21/22.
- **Reuse from XU:** `identity/normalize.rs`, `lists.rs`, `removed.rs`, `score.rs`, the
  identifier builder in `identifiers.rs` and the xdg-ninja data (`curated.rs`). That is about
  2.3k code lines, std + libc only. Details in §3.

---

## 1. Prior art (web research)

| Tool | Approach | Finds removed-app leftovers? | License | Reuse |
|---|---|---|---|---|
| xdg-ninja | curated JSON | no (XDG hints) | MIT | data, done in XU |
| antidot | curated YAML | no | MIT | skip (subset) |
| xdg-unused-data | curated + `which` | yes, curated | MIT | data, maybe |
| lostfiles | `/` vs pacman | no HOME | MIT | idea only |
| pacreport | unowned files | no HOME | MIT | idea only |
| BleachBit | CleanerML | no (junk only) | GPL-3 | no |
| flatpak `--delete-data` | exact app ID | yes, Flatpak | LGPL-2.1 | semantics |
| Flatsweep | `~/.var/app` vs app dirs | yes, Flatpak | MPL-2.0 | idea only |
| mundus | curated DB | yes, curated | GPL-3 | no (dead 2016) |
| Revo / BCU | name + score | Windows | closed / Apache | method (XU did) |

Details:
- **xdg-ninja** (https://github.com/b3nj5m1n/xdg-ninja, MIT, 3.4k stars, HEAD `ebf481a`
  2026-10-04).
  - `programs/*.json` holds `{name, files:[{path, movable, help}]}` for 655 programs.
  - It checks which known paths exist and says whether they can be moved to XDG.
  - It has no notion of installed vs removed. XU already imports it at the same pinned
    SHA, with the `help` prose stripped (`data/xdg-ninja-rules.json`, 52 KB, credited in
    `THIRD_PARTY.md`).
- **xdg-unused-data** (https://github.com/pawel-0/xdg-unused-data, MIT, 37 stars, last push
  2024-09).
  - The only found tool that targets exactly our question.
  - 336 `applications/*.json` files: `{name, executables[], locations[{file:"$XDG_CONFIG_HOME/0ad"}]}`.
  - README: "*If the executable is missing but at least one of the associated files or
    folders exists, it is flagged*".
  - It once had a delete feature and removed it "*due to concerns about potential risks*".
  - Lesson: a curated list plus an "executable missing" test. Coverage is limited to known
    apps, and it lacks package history, so a renamed binary gives false positives.
  - Its data is MIT and could be imported like xdg-ninja (attribution required). It is
    optional, because the measurement shows the pacman-based evidence covers more.
- **antidot** (https://github.com/doron-cohen/antidot, MIT): about 111 move rules, a subset of
  xdg-ninja. Skip.
- **lostfiles** (https://github.com/graysky2/lostfiles, MIT file in repo).
  - Arch Wiki *Pacman/Tips and tricks → Identify files not owned by any package*:
    `comm -13 <(pacman -Qlq | sed 's,/$,,' | sort) <(find /etc /usr /opt … )`. "*The lostfiles
    script performs similar steps, but also includes an extensive blacklist to remove common
    false positives*".
  - System only, no attribution. Useful for Phase 2 only.
- **pacreport --unowned-files** (pacutils, https://man.archlinux.org/man/pacreport.1.en).
  - Unowned-file check with `IgnoreUnowned` and `PkgIgnoreUnowned` globs in
    `/etc/pacreport.conf`.
  - The man page's example sets `IgnoreUnowned = home/*`, so HOME is out of scope by design.
- **BleachBit deep scan** (https://sources.debian.org/src/bleachbit/5.0.2-1/cleaners/deepscan.xml).
  - Backup files (`*~`, `*.bak`), `.DS_Store`, `Thumbs.db`, Office temp files, vim swap files;
    newer versions add `node_modules`/`venv`.
  - No removed-app logic. GPL, so we copy nothing (AGENTS.md).
- **Flatpak** (https://man.archlinux.org/man/flatpak-uninstall.1.en): `--delete-data`
  "*Remove app data in ~/.var/app and in the permission store*". "*When --delete-data is used
  without a REF, all 'unowned' app data is removed.*"
  - Source `app/flatpak-builtins-uninstall.c`: it iterates `~/.var/app`. It skips non-dirs.
    It skips a name when `flatpak_find_current_ref(name)` finds a ref in any installation.
    It also skips a name that is not a valid Flatpak name. It prompts for each remaining dir.
  - This is our EXACT rule for `~/.var/app`. XU deliberately never calls `--delete-data`: it
    also wipes portal permissions, and the descriptor engine must do the deleting.
- **Flatsweep** (https://github.com/giantpinkrobots/flatsweep, MPL-2.0, GTK4/libadwaita, Python).
  - It flags a `~/.var/app/<dir>` that has `cache`+`config`+`data`, no space in its name, and
    no entry in `/var/lib/flatpak/app` or `~/.local/share/flatpak/app`.
  - Flathub warns it may misfire with a custom install path. It ignores
    `/etc/flatpak/installations.d`; we must not.
- **Flatpak app ID rules** (https://docs.flatpak.org/en/latest/conventions.html): reverse-DNS,
  D-Bus bus-name rules, a lowercase domain. Data lives in `~/.var/app/<app-id>/{data,config,cache}`.
- **Desktop file IDs** (https://specifications.freedesktop.org/desktop-entry/latest/file-naming.html):
  - The basename before `.desktop` "*should be a valid D-Bus well-known name*", in reverse-DNS
    form (`org.example.FooViewer`).
  - Names in `~/.local/share/<rdns-id>` (here `com.zarz.mornye`,
    `dev.stormix.deadlock-mod-manager`) therefore map to desktop IDs and their last component.
- **Electron** (https://www.electronjs.org/docs/latest/api/app): `userData` "*by default is the
  appData directory appended with your app's name*". The name is `productName`, which "*will be
  preferred over name*". That explains `~/.config/Mullvad VPN`, `discord`, `VSCodium`.
- **Arch Wiki *System maintenance → Old configuration files*:** "*Remove unneeded configurations
  periodically, particularly in your home directory and ~/.config*". The wiki offers no method
  beyond manual review and a personal list (`rmshit.py`).
- **Forums (the same question asked many times):**
  - https://bbs.archlinux.org/viewtopic.php?id=299467: suggestions are `find ~/.config -atime +720`
    (fails on noatime), sorting by mtime, leaving `dconf`/`gtk-*`/session dirs alone, deleting
    in batches, re-login.
  - https://discuss.kde.org/t/app-for-deleting-obsolete-files-under-home-username/50476: a fish
    function matches the package name across XDG dirs and trashes the matches.
    Problems reported: `ephoto-bl` vs `ephoto`, `keepass` vs `keepassxc`, and "*Icons are
    usually part of icon themes*".
  - https://forum.manjaro.org/t/how-to-clean-up-config-files-from-unninstalled-apps/169422:
    "*the user files of each app have its own static directory*" (Flatpak), with no reliable
    check for native apps.
  - https://discuss.cachyos.org/t/home-directory-orphan-file-management/35445: "*It really
    does require user input*". pacman "*only knows about files in the package's inventory*".
- **Conclusion:** nobody combines **package history + birth time + installed-identity veto**.
  That combination is XCleaner's (and XU's) edge. The measurement in §4 shows it works.

---

## 2. Evidence sources on Arch/CachyOS (all readable without root)

| Source | Gives | Cost here |
|---|---|---|
| `/var/lib/pacman/local/*/{desc,files}` | installed names, base, provides, files | 1537 pkgs |
| `/var/log/pacman.log` | install/remove times | 343 removes |
| sync files DB (`pacman -Fl`) | files of removed repo pkgs | 3.4 s, 221/253 |
| `/var/cache/pacman/pkg` | archive of removed pkg | 0 hits here |
| XU records `~/.local/share/xuninstaller/applications/*.record` | aliases, binaries, launchers saved before removal | 10 records |
| `.desktop` (sys, user, Flatpak exports) | ID, Exec argv0, WMClass, Icon, Name | — |
| PATH binaries incl. unowned | `cava` is unowned in `/usr/bin` | — |
| `/proc/*/comm` | running apps | — |
| Flatpak installations | app IDs (user + system + `installations.d`) | 1 app |
| AppImages (`~/Applications`, `~/.local/bin`) | names | 3 |
| Steam manifests | installed games | — |
| statx btime | dir birth | btrfs has it |

Notes:
- `pacman.log` here starts **2026-06-24**, and HOME was restored in mass on
  2025-10-13 / 2026-09-04 / 2026-09-07. On those days 8–12 entries per root were born.
  Anything removed before the log started is invisible. Examples: `gnome-software`, `yelp`'s
  first life, `cava`'s earlier history.
- The pacman cache had **no** archive of any removed package (it is cleaned with
  `paccache -ruk0` or similar). The sync files DB is what resolves `psd` →
  `profile-sync-daemon` and `plasmalogin` → `plasma-login-manager`.
  - It is 82 MB of zstd/gzip tar. XCleaner's goal is "no shelling out", so reading it in
    process needs `zstd` + `flate2` + a tar reader.
  - Recommendation: v1 runs `pacman -Fl <removed…>` once, with an argv vector and no shell,
    and caches the result. That is a deliberate exception for a read-only query. If the owner
    forbids it, use only the pkgname + XU-record evidence.
  - AUR packages (`dorion-bin`, `bs-manager-git`, `millennium`, `shadps4-*`) are not in any
    files DB. For them the name, the btime and XU records do the work.
- **Atime is unusable.** The mount is `relatime`, and reading a directory updates its atime,
  so XCleaner's own scan or XU's sets every dir to "today". Measured: every entry has an atime
  of 2026-11-21 or 2026-11-22.
- **Mtime of the dir itself is polluted by foreign writers.** `~/.config/chromium` got
  `NativeMessagingHosts/com.anthropic.claude_code_browser_extension.json` on 2026-11-20, four
  days after chromium was removed. "Last activity" must ignore known foreign subtrees
  (`NativeMessagingHosts`, `.directory`, `.DS_Store`).

---

## 3. XUninstaller identity engine (MIT, reuse allowed)

Path: XUninstaller repository, `crates/xuninstaller-core/src/identity/`.
Design docs in the same repo under `docs/research/`: `FILE-IDENTIFICATION-2026-11.md` (the 10-agent
synthesis), `FILE-LEFTOVERS-2026-11.md` (evidence model, Mullvad case), and
`RESEARCH-WAVE2-2026-11.md` (sections 06 Electron, 17 AUR caches, 20/21 KDE/GNOME).

**Evidence it uses:**
- **Package ownership.** It reads pacman local DB `desc`/`files` directly, with root-owned
  files only and `O_NOFOLLOW`. It builds a `PackageIdentity` per installed package from
  several identifiers:
  - pkgname and pkgbase;
  - `/usr/bin` basenames, and wrapper targets into `/opt`;
  - `.desktop` ID, `Exec` argv0, `StartupWMClass`, `Icon`, `Name`;
  - AppStream ID and binaries;
  - gschema IDs and paths, polkit IDs, D-Bus names.
- **Removed packages.** `removed.rs::parse_pacman_log` returns the packages whose **last**
  event is `removed`, with the timestamp. Saved XU records (`application_history`) add aliases,
  display names, binaries and launcher IDs captured *before* removal.
- **Static extractors** (`extract.rs`):
  - Electron `app.asar` → `package.json` `productName`/`name` (asar header parser, no
    dependency);
  - VS Code `product.json` `dataFolderName` (→ `~/.vscode-oss`);
  - Unity `app.info` (Company/Product);
  - Steam `appmanifest_*.acf`.
- **Curated data** (`curated.rs`): xdg-ninja JSON at a pinned SHA, plus local rules (Discord,
  Mullvad, vendor dirs).
- **Normalization** (`normalize.rs`):
  - fold (NFKC-ish, lowercase) and squash (alphanumeric only);
  - strip extension, version and packaging/channel suffixes;
  - reverse-DNS split into vendor + leaf;
  - tokens and a bounded edit distance.
- **Scoring** (`score.rs`): BCU-style integers, **best alias only** (scores never add up).
  - Weights: pkgname/desktop ID/app ID +6, binary/WMClass/Icon +5, display name +4.
  - Name + known suffix (`rc`, `-backups`, …) +3, first token +3, edit distance ≤1 +2.
  - Vetoes: owned by an installed package, or another installed package matches equally
    (−4) or better (veto).
  - "Written after uninstall" −3.
  - Tiers: ≥6 PreChecked, 2–5 Unchecked, 0–1 Hidden, <0 Dropped.
  - Polish reason strings come with each score.
- **Hard lists** (`lists.rs`): `SHARED_NEVER` (about 100 names), `VENDOR_PARENTS` (leaf only),
  `GENERIC_STOPLIST`. Aliases of ≤4 chars match exactly only; fuzzy needs ≥6; a binary shared by
  more than 20 packages is ignored.
- **Integration files** (`integration.rs`): `.desktop`/autostart files whose Exec targets the
  app, user systemd units and their `.wants` links, icons by `Icon=`, native-messaging JSON, and
  token-level `mimeapps.list` edits.

**Portability into `xcleaner-core`** (code lines exclude inline tests):

| File | Code lines | Deps | Port? |
|---|---|---|---|
| identity/normalize.rs | 288 | std | yes, as is |
| identity/lists.rs | 285 | normalize | yes + XC DANGER list |
| identity/removed.rs | 72 | `valid_package_name` | yes + keep install times |
| identity/score.rs | 549 | identifiers | yes, retune tiers |
| identity/identifiers.rs | 749 | libc, `local_db::Fields` | yes, small parser |
| identity/curated.rs | 200 | JSON in extract | yes + `xdg-ninja-rules.json` |
| identity/extract.rs | 510 | std | later (asar, product.json) |
| identity/integration.rs | 873 | — | partly (dead `.desktop`, v0.0.2) |
| identity/mod.rs | 592 | XU internals | no, write own adapter |
| application_history.rs | 739 | XU paths | reader of `*.record` only |
| removed_history.rs | 139 | — | no (dup of removed.rs) |

- **Coupling to cut:**
  - `crate::local_db::Fields`: a strict `%KEY%` tokenizer. Copy about 80 lines or write a
    small one.
  - `crate::removal::valid_package_name`: about 10 lines.
  - `crate::runner::run_command("pacman-conf", ["LogFile"])`: replace with the default
    `/var/log/pacman.log` plus a `LogFile =` lookup in `/etc/pacman.conf`.
  - `crate::application_history`, `Category`, `InventoryError`, `xdg_scan`.
  - Nothing GTK, no new crates. `xcleaner-core` already depends on `libc` only.
- **Changes needed for XCleaner:**
  1. `removed.rs` must also return the **install timestamps** of each removed package
     (needed for btime correlation). Today it keeps only the last removal.
  2. `IdentifyMode`/`Tier` semantics: XU preselects at ≥6 because the user just chose "remove
     this app". XCleaner scans with no target app, so the default must be stricter (§6).
  3. XU identifies leftovers *for one target package*. XCleaner needs the inverse: for each
     HOME entry, find the *best owner* among installed and removed identities.
     `IdentityIndex { installed, removed }` and `score_candidate` already model this, as in
     `identify_foreign_in`. The loop just has to run per entry instead of per target.
- **Share rather than fork?** A tiny shared crate (`x-identity`) used by both repos would avoid
  two diverging copies. That is an owner decision; copying is acceptable under MIT with the
  copyright line kept.

---

## 4. Measurement on this machine (read-only, 2026-11-22)

Inputs: 1537 installed packages. 253 packages whose last log event is `removed` (log from
2026-06-24); 221 of them resolved by the files DB. Flatpak: `com.hypixel.HytaleLauncher`
(user). `~/.var/app` does not exist. 3 AppImages, 2 scripts in `~/.local/bin`.

Counts (218 entries: `~/.config` 79, `~/.local/share` 107, `~/.local/state` 11, `~/.*` 21):

| Verdict | Entries | Size |
|---|---|---|
| installed owner | 67 | 39 GB |
| shared/system (deny-list) | 66 | 351 GB (Steam 346) |
| removed-package match | 23 | 9.5 GB |
| prefix of installed pkg | 13 | 6.3 GB |
| integration stub | 4 | 16 KB |
| unknown | 45 | 13 GB |

### 4.1 Removed-package matches (candidate leftovers)

Columns: entry, size, removed package and date, newest mtime, btime Δ after that package's
install, hand check. `cfg` = `~/.config`, `share` = `~/.local/share`.

| Entry | Size | Removed pkg | mtime | btime Δ | Check |
|---|---|---|---|---|---|
| share/BSManager | 5.4 GB | bs-manager-git 10-18 | 10-06 | +30 s | true, game data |
| share/pnpm | 3.0 GB | pnpm 11-18 | 11-12 | pre-log | true, hard links |
| cfg/discord | 708 MB | discord 10-21 | 10-20 | +5 s | true |
| cfg/chromium | 168 MB | chromium 11-16 | 11-20* | +4 s | true, profile |
| cfg/chromium-back-ovfs.base | 174 MB | psd 11-18 | 11-18 | −6 s before psd rm | true, psd backup |
| share/shadPS4QtLauncher | 65 MB | shadps4-qtlauncher-bin 07-29 | 07-25 | +11 s | true |
| share/hhd | 12 MB | hhd 11-18 | 2025-10-14 | pre-log | true |
| share/shadPS4 | 1 MB | shadps4-bin 07-29 | 07-25 | +90 s | true, saves? |
| share/zed | 1 MB | zed 09-05 | 09-05 | +10 min | true |
| share/millennium | 904 KB | millennium 11-18 | 10-29 | +16 s | true |
| share/kactivitymanagerd | 420 KB | kactivitymanagerd 09-05 | 06-20 | pre-log | true |
| cfg/millennium | 332 KB | millennium 11-18 | 11-09 | +16 s | true |
| share/evolution | 100 KB | evolution-data-server 09-11 | 09-11 | pre-log | true (first token) |
| cfg/go | 52 KB | go 11-18 | 11-12 | +3 s | true |
| cfg/evolution | 12 KB | evolution-data-server 09-11 | 09-04 | pre-log | true (first token) |
| cfg/dorion | 12 KB | dorion-bin 11-04 | 11-04 | +5 s | true |
| share/yelp | 8 KB | yelp 09-05 | 2025-12-08 | pre-log | true |
| cfg/com.spikehd.dorion | 4 KB | dorion-bin 11-04 | 11-04 | +138 s | true |
| cfg/psd | 4 KB | profile-sync-daemon 11-18 | 11-18 | +2 d | true (files DB: `psd`) |
| share/glances | 4 KB | glances 09-05 | 09-05 | +28 min | true |
| cfg/Discord Key Storage | 4 KB | discord 10-21 | 10-22 | born after rm | **false**: discord-canary |
| share/dorion | 0 B | dorion-bin 11-04 | 11-04 | +5 s | true |
| share/plasmalogin | 0 B | plasma-login-manager 09-05 | 06-20 | pre-log | true (files DB) |

\* `cfg/chromium` mtime 11-20 comes only from a foreign `NativeMessagingHosts/` write (Claude
Code). Excluding that subtree, the newest write is 11-18 (psd unsync at removal).

- Attribution precision: **22/23**. The miss is caught by the "born after removal" veto (§5.3).
- "true" does not mean "safe to preselect":
  - BSManager holds Beat Saber installs and songs.
  - shadPS4 may hold saves.
  - chromium and the `*-back-ovfs.base` dirs are full browser profiles (cookies, logins).
  - pnpm `store` is hard-linked into projects, so freed space may be smaller (see the
    HOME-JUNK-CATALOG `st_nlink` note).
- `chromium-back-ovfs.base` and `google-chrome-back-ovfs.base` were both born 2026-11-18 08:20:05.
  `profile-sync-daemon` was removed at 08:20:11. This proves they are psd's "unsync" backups.
  A curated rule `*-back-ovfs.base` + psd removed belongs in STRONG.

### 4.2 Prefix of an installed package (13): not leftovers, except 3

| Entry | Size | Matched | Check |
|---|---|---|---|
| share/umu | 3.7 GB | umu-launcher | in use |
| share/osu | 2.4 GB | dead `osuwinello-*.desktop` | **leftover** (osu-winello, manual) |
| cfg/google-chrome-back-ovfs.base | 193 MB | google-chrome | **leftover** of psd |
| share/audacious-discord-rpc | 15 MB | audacious | unclear (plugin) |
| cfg/google-chrome-for-testing | 332 KB | google-chrome | in use (tooling) |
| cfg/hypr | 28 KB | hyprland (share dir) | **in use, critical** |
| ~/.vscode-oss(-shared) | 32 KB | vscodium-bin | in use (`dataFolderName`) |
| ~/.android | 12 KB | android-tools | in use |
| cfg/cachyos | 8 KB | cachyos-* | in use |
| share/node-steamuser | 4 KB | nodejs | unclear |
| cfg/google-chrome-headless | 0 B | google-chrome | in use |
| share/gnome-software | 0 B | gnome-desktop-* | leftover, pre-log |

- Prefix and first-token matches are **ambiguous in both directions**. They must never pass
  REVIEW, and when they point at an *installed* package they veto by default.
- `hypr` ↔ `hyprland` shows why the vetoes must be generous. A missed owner would offer
  the compositor config.

### 4.3 Integration stubs (4)

- `~/.config/{BraveSoftware,microsoft-edge,opera,vivaldi}` (4 KB each, 2026-11-20) contain only
  `…/NativeMessagingHosts/com.anthropic.claude_code_browser_extension.json`. None of these
  browsers was ever installed. Claude Code writes its host manifest into every Chromium-family
  path.
- Rule: a dir whose only content is `NativeMessagingHosts/` belongs to the app named by the
  manifest, not to the browser. Do not show it as a browser leftover. The owning app recreates it.

### 4.4 Unknown (45, 13 GB): never offered

- Manual/AppImage/game data still in use:
  - `wineprefixes` 4.9 GB and `osuconfig` 731 MB (osu-winello, foobar2000, l4d);
  - `yawl` 2.7 GB (Wine wrapper runtime);
  - `com.megalithofficial.basalt-launcher` 1.6 GB, `CapoRhythia` 100 MB, `Antra` 81 MB,
    `spotube` 30 MB, `gg.essential.mod` 29 MB, `Vencord` 4 MB (mtime 11-21);
  - `xdns` (owner's project), `ukishima*` and `pill` state files (mtime 11-18…22).
- Vendor dirs (Unity/Qt "Company"):
  - `Sandbox Interactive GmbH` (Albion Online), `Ndemic Creations`, `Meltytech` (Shotcut,
    not installed), `MelonLoader Installer`.
  - Vendor names never match package names. They need curated rules or the Unity
    `app.info` extractor, and leaf-only deletion.
- `com.epicgames.eglv2` 2.5 GB:
  - `epic-games-launcher-bin` was installed 11-12 and removed 11-18 (same day as the last
    write). But the dir was born **10-15**, before that install, so an earlier install
    (Flatpak/Heroic?) made it.
  - The vendor token `epicgames` matches the removed package. Verdict: REVIEW, not STRONG.
- Library/tool state with no owning app name:
  - `matplotlib`, `jgit`, `nextjs-nodejs` (env-paths `<name>-nodejs`), `.semgrep`, `rclone`
    (0 B), `.gsd-keyboard.settings-ported` (gnome-settings-daemon marker);
  - `resourcefullib`, `rpc-bridge-latest`, `steamforge*`, `simple-update-notifier`,
    `portmaster` + `io.safing.portmaster`, `warp-terminal`, `duckstation`, `PandoraLauncher`,
    `VRSideForge`, `sbox`, `SpotiFLAC-Next`, `stalefiles`, `omniroute-desktop`.
  - Many of these are probably leftovers (portmaster, warp-terminal, duckstation are not
    installed). But there is **no evidence** beyond "no owner": they predate the log or came
    from AppImages. They stay unknown.

### 4.5 Deny-list false-positive classes observed

- Toolkit and desktop state named after a library, not an app: `gtk-3.0`, `gtk-4.0`,
  `glib-2.0`, `dconf`, `ibus`, `goa-1.0`, `procps`, `pulse`, `pipewire`, `wireplumber`,
  `systemd`, `environment.d`, `menus`, `kdeglobals`, `QtProject.conf`, `mimeapps.list`,
  `user-dirs.*`, `gvfs-metadata`, `webkitgtk-4.1`, `gegl-0.4`, `gstreamer-1.0`, `folks`, `icc`,
  `vulkan`.
- Spec-defined shared dirs: `applications`, `icons`, `fonts`, `mime`, `sounds`,
  `desktop-directories`, `Trash`, `keyrings`, `recently-used.xbel`, `user-places.xbel*`.
- Runtime roots: `Steam`, `flatpak`, `containers`, `pki`, `man`, `dbus-1`, `IsolatedStorage`
  (.NET), `Sentry`, `openvr`, `openxr`, `unity3d`, `Valve`, `networkmanagement`, `orca`.
- Dotfiles: `.bashrc`, `.bash_history`, `.ssh`, `.gnupg`, `.pki`, `.java`, `.npm`, `.npmrc`,
  `.cargo`, `.steam`, and the XDG roots themselves.
- Most of these *also* match an installed package (`dconf`, `ibus`, `pipewire`), so a pure
  "has owner" check would be right today. But after removing e.g. `ibus` the dir would wrongly
  become "leftover of ibus". It is still shared GTK/IM state, so the deny-list must win over
  removed-package evidence.

---

## 5. Algorithm

### 5.1 Pipeline (per scan, about 1 s plus `pacman -Fl` about 3.4 s cached)

1. **Collect installed identities:**
   - local DB (name, base, provides, `usr/bin/*`, `usr/share/applications/*.desktop`,
     metainfo, `opt/<x>/`, `etc/xdg/<x>`, user units, D-Bus services, gschemas);
   - every PATH binary, unowned included;
   - all `.desktop` files (system, user, Flatpak exports): ID, rDNS leaf, Exec argv0, WMClass,
     Icon, Name;
   - Flatpak app IDs from all installations;
   - AppImages, `/proc/*/comm`, Steam manifests.
   - Scope it: XU's `identifiers.rs` covers the package part. Add PATH, procs, Flatpak and
     AppImages.
2. **Collect removed identities:** packages whose last `pacman.log` event is `removed` and
   that are not installed now. Keep every install/remove timestamp. Enrich them from:
   - `pacman -Fl` (binaries, desktop IDs, user units);
   - XU `*.record` files;
   - cached archives, when present.
3. **Enumerate** depth-1 entries of the roots, using `lstat` with no follow. For each entry
   compute: size, newest mtime excluding foreign subtrees, btime, "contains executables",
   "contains `drive_c`/saves-like dirs", and file count.
4. **Normalize** names: fold, squash, core (strip extension, version, packaging suffix,
   `rc`/`-backups`/`-icons`/`-cache`/`-location`), reverse-DNS → (vendor, leaf).
   Aliases ≤4 chars match exactly only.
5. **Deny-list first** (§5.4): the entry is never a leftover.
6. **Installed veto:** if any installed identity equals the entry (squash or core, or rDNS leaf
   = installed desktop/Flatpak leaf), the verdict is `installed`. Stop.
7. **Score against removed identities** (§5.2), best alias only.
8. **Tier** (§5.3) and attach the reasons.

### 5.2 Evidence and weights

Positive (take the best name signal, then add context):

| Id | Evidence | Weight |
|---|---|---|
| N1 | name = removed pkgname/base | +6 |
| N2 | name = removed binary/desktop ID/rDNS leaf (files DB) | +6 |
| N3 | name = XU record alias/binary/desktop ID | +7 |
| N4 | first token = removed pkg (≥4 chars) | +3 |
| N5 | rDNS vendor = removed pkg first token | +2 |
| N6 | curated rule (xdg-ninja, local) for removed pkg | +3 |
| T1 | btime 0–30 min after that pkg's install | +4 |
| T2 | btime 30 min–14 d after install | +2 |
| T3 | newest write ≤ removal + 1 d | +1 |
| D1 | user `.desktop` with dead Exec names it | +3 |

Negative and vetoes:

| Id | Evidence | Effect |
|---|---|---|
| V1 | deny-list name | veto |
| V2 | installed identity equal | veto (installed) |
| V3 | **btime after removal** | veto (someone else made it) |
| V4 | process running / open fd inside | blocked |
| V5 | installed pkg shares first token | −3 |
| V6 | write > removal + 1 d (excl. foreign) | −4 |
| V7 | btime in mass-birth cohort | T1/T2 = 0 |
| V8 | only `NativeMessagingHosts/` | stub, not leftover |
| V9 | symlink, mount point, other owner | blocked |

- A mass-birth cohort is a day on which more than 30% of a root's entries were born. Here those
  days are 2025-10-13, 2026-09-04 and 2026-09-07 (from XU R7).
- "Pre-log" (btime earlier than the first log line) is neutral, not negative.

### 5.3 Tiers

- **EXACT ("pewne"):** score ≥ 9 with **N1/N2/N3 + T1**, no veto, no V5/V6. Also any
  `~/.var/app/<valid-id>` with no current ref in any Flatpak installation (Flatpak's own rule).
  - Here: discord, chromium, go, dorion×3, millennium×2, shadPS4×2, BSManager. Add zed and
    glances via T2, which makes them STRONG, not EXACT.
- **STRONG ("prawie pewne"):** score 6–8, name evidence N1–N3, no veto.
  - Here: pnpm, hhd, kactivitymanagerd, yelp, plasmalogin, psd, zed, glances, and the
    `*-back-ovfs.base` pair (curated psd rule).
- **REVIEW ("do przejrzenia"):** score 3–5. Examples: first-token only (evolution×2), vendor
  only (com.epicgames.eglv2), dead launcher only (osu, osuconfig).
- **UNKNOWN:** score < 3 or no evidence. Not listed in the leftovers group. At most an optional
  read-only diagnostics list ("Nieprzypisane katalogi") with no checkbox.
- **Preselection (owner to decide; recommendation):**
  - Never preselect REVIEW or UNKNOWN.
  - EXACT or STRONG may be preselected only when the entry has **no personal-data markers**:
    browser profile files (`Cookies`, `Login Data`, `places.sqlite`), `drive_c`, saves,
    songs/instances, `.git`, more than N user files, or keys/tokens (`*Key Storage*`,
    `*.key`). Today's AGENTS.md says "leftover profiles are never preselected", which already
    rules out chromium, discord, BSManager and the ovfs pair.
  - Owner decision pending: EXACT + no-personal-data → preselect? Recommendation: **no in
    v0.0.3**. Show it checked-able with a bold "pewne" chip, and revisit after real-world use.
- **Leaf only:** vendor parents (`unity3d/<Company>`, `~/.config/<Org>`) are offered only as
  `<Org>/<App>`. The parent goes only when it becomes empty.

### 5.4 Deny-list (shared or system; never a leftover even on exact match)

- Start from XU `lists.rs::SHARED_NEVER` plus the HOME-JUNK-CATALOG DANGER list. Add the names
  measured in §4.5:
  - toolkit: `gtk-2.0/3.0/4.0`, `gtkrc*`, `glib-2.0`, `dconf`, `ibus`, `fontconfig`,
    `gstreamer-1.0`, `webkitgtk-*`, `gegl-*`, `QtProject.conf`, `kdeglobals`, `kde.org`,
    `Trolltech.conf`;
  - session: `systemd`, `environment.d`, `autostart`, `menus`, `mimeapps.list`,
    `user-dirs.dirs`, `user-dirs.locale`, `pulse`, `pipewire`, `wireplumber`, `goa-1.0`,
    `procps`, `gvfs-metadata`, `folks`, `dbus-1`, `.gsd-*`;
  - spec dirs: `applications`, `icons`, `fonts`, `mime`, `sounds`, `desktop-directories`,
    `Trash`, `keyrings`, `recently-used.xbel`, `user-places.xbel*`, `icc`, `vulkan`, `man`;
  - runtime roots: `flatpak`, `containers`, `Steam`, `.steam`, `pki`, `.pki`,
    `IsolatedStorage`, `Sentry`, `openvr`, `openxr`, `unity3d`, `Valve`, `Epic`, `.mozilla`,
    `networkmanagement`, `orca`;
  - dev roots: `.cargo`, `.rustup`, `.npm`, `go` (as `~/go`), `.java`, `.mono`, `.dotnet`,
    `.nuget`, `.wine`;
  - secrets/config: `.ssh`, `.gnupg`, `.bashrc`, `.bash_history`, `.profile`, `.npmrc`,
    `.claude*`, `.codex`, `hypr`, `fish`, `kitty`, `quickshell`.
- Match folded and with extension stripped, the same as `is_shared_never`.
- The list is enforced by a unit test, like the v0.0.2 "never" list.

### 5.5 Renamed apps, case and spacing

- Case and spacing: compare **squashed** forms (`Mullvad VPN` ≡ `mullvadvpn` ≡ `mullvad-vpn`,
  `discord-canary` ≡ `discordcanary`, `shadPS4` ≡ `shadps4`).
- Packaging suffixes: strip `-bin`, `-git`, `-appimage`, `-nightly`, `-beta` and versions
  (`dorion-bin` → `dorion`, `bs-manager-git` → `bsmanager` ≡ `BSManager`). **Keep channel
  words** (`canary`, `ptb`, `beta` as part of the app name) when the installed side uses them,
  so that `discord` ≠ `discordcanary`.
- Renames and forks: the name the package uses differs from the dir name in several common
  cases:
  - binary in the files DB: `profile-sync-daemon` → `psd`;
  - desktop ID / rDNS leaf: `com.spikehd.dorion` → `dorion`;
  - Electron `productName`: needs the extractor when installed, or an XU record when removed;
  - provides/replaces: `procps-ng` provides `procps`.
  - A package rename (old name removed, new name installed the same minute, e.g. `-git` ↔
    `-bin`) shows up as `removed X` + `installed Y` in one transaction. Treat Y's identities as
    installed. If X and Y squash to the same core, it is the **installed** owner (V2), never a
    leftover.
- Families: the shortest installed name that prefixes others is the base
  (`audacious` / `audacious-plugins`). A dir named after the base is owned while any member is
  installed.

### 5.6 `~/.var/app` (Flatpak)

- EXACT when all of these hold:
  - the name is a valid Flatpak ID (D-Bus rules, ≥3 components);
  - no ref with that ID in the user installation, the system installation, or any
    `/etc/flatpak/installations.d/*.conf` installation (list via `flatpak list --app
    --columns=application,installation` or the `app/` dirs of each installation);
  - the app is not running (`flatpak ps`, or `/proc/*/root` under the app).
- Present `cache/` separately (safe-ish) from `config/` + `data/` (user data). Never call
  `flatpak uninstall --delete-data`: it also resets portal permissions and deletes without our
  descriptor engine. Permission-store cleanup is a separate, report-only item.
- This machine has no `~/.var/app` yet, so the rule is untested here. Fixture-test it.

### 5.7 UI presentation (Polish)

- Group: **"Pozostałości po odinstalowanych aplikacjach"**, with sub-groups "pewne",
  "prawie pewne" and "do przejrzenia". Unknown entries are not in this group.
- Row: path, size, last write (excluding foreign subtrees), tier chip, and a one-line reason
  built from the strongest evidence, e.g.:
  - "Usunięto pakiet discord 2026-10-21; katalog powstał 5 s po jego instalacji (2026-10-20);
    brak programu w systemie."
  - "Usunięto pakiet profile-sync-daemon 2026-11-18; nazwa `psd` to jego program (baza plików
    pacmana)."
  - "Flatpak com.example.App nie jest zainstalowany (ani dla użytkownika, ani systemowo)."
  - "Do przejrzenia: nazwa zaczyna się od `evolution` (usunięty evolution-data-server
    2026-09-11); brak dokładnego dopasowania."
- A "Dlaczego?" expander lists all contributions with their signs, as XU's
  `reasons: Vec<String>` already does, including the vetoes checked ("żaden zainstalowany
  pakiet nie używa tej nazwy", "żaden proces nie działa").
- Warning chips:
  - "profil przeglądarki: hasła/ciasteczka";
  - "dane gry / zapisy";
  - "twarde dowiązania: zwolni mniej niż X";
  - "zmieniany po odinstalowaniu".

---

## 6. Tests to write (fixture-based, like XU `identity/tests.rs::owner_machine`)

- discord removed + discord-canary installed: `discord` is EXACT, `discordcanary` is installed,
  and `Discord Key Storage` is vetoed by V3.
- `psd` resolves through files-DB binary evidence. `*-back-ovfs.base` gets the curated psd rule.
- `hypr` is never a leftover while `hyprland` is installed. `ibus`/`dconf` are never leftovers
  after their package is removed (deny-list beats removed evidence).
- An NMH-only browser dir is a stub, not a leftover.
- A mass-birth cohort zeroes T1/T2. A pre-log btime is neutral.
- An unowned PATH binary (`/usr/bin/cava`) vetoes `~/.config/cava`.
- Flatpak: an orphan ID in `~/.var/app` with a system-installation ref is not an orphan, and an
  invalid ID is ignored.
- Rename transaction: `foo-git` removed and `foo-bin` installed in the same minute, so `foo`
  is installed.

---

## 7. Limits and unverified assumptions

- Precision numbers come from **one machine**. They are hand-checked by name, timeline and
  directory layout, not by app source code.
- AUR packages lack files-DB data. Their leftovers rely on name and btime only. XU records
  help only for apps removed through XU.
- Packages removed before `pacman.log` begins (here 2026-06-24) give no removed evidence.
  Their dirs stay unknown, by design.
- btime requires a filesystem that supports `statx` `STATX_BTIME` (btrfs, ext4 and xfs do). A
  copy/restore resets it, which is what the cohort detection handles.
- `pacman -Fl` depends on the files DB being synced (`pacman -Fy`, root). Here
  `cachyos-core-v3.files` was from 2026-10-07. A stale DB only reduces recall.
- The xdg-unused-data rule set was not evaluated against this HOME. It would add coverage for
  known apps without pacman history (and needs an MIT attribution entry).
- The claim that `com.epicgames.eglv2` came from an earlier Epic install is inferred from btime
  10-15 < pacman install 11-12. Not proven.
- Not covered here (separate roadmap items): `~/.cache/*` leftovers (lower value, regenerable),
  dead `.desktop` files and symlinks (v0.0.2), Steam `compatdata` orphans (catalog §1.3), and
  `mimeapps.list` token cleanup (XU `integration.rs`).
