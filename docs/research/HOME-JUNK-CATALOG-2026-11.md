# $HOME junk catalog (research, 2026-11-22)

Scope: Phase 1, `$HOME` only. Read-only measurements on the owner's machine (Steam Deck OLED, CachyOS deckify kernel 7.3.4, Hyprland). `/home` is btrfs `@home`, 475 GB, **460 GB used, 15 GB free (98%)**.
Method: `du -xsh`, `find`, `stat` (incl. birth time `%w`), `pgrep -x`, `coredumpctl list`, `flatpak list`. No file contents were read except Steam manifest `name`/`LastPlayed` keys and Proton `config_info` (first lines: Proton version, paths).
Reused, not redone: `~/Pobrane/test/xu-integration/docs/research/RESEARCH-WAVE2-2026-11.md` (sections 03 Steam, 06 Electron, 14 coredumps).

Running at measurement time: `steam` (2), `steamwebhelper` (10), `DiscordCanary` (9), `codium` (9), `Hyprland`, `kitty`. Not running: zen, chrome, prismlauncher, java, wineserver, cs2.

Caveat for sizes: `du` counts a hard-linked file once per invocation. Flatpak (`repo/objects` vs `runtime/`) and pnpm (`store` vs project `node_modules`) use hard links, so deleting one side frees little or nothing. XCleaner must compute "reclaimable" by inode link count (`st_nlink == 1`, or all links inside the selection).

---

## 0. Concept: "junk that regenerates with the same config"

- **XDG Base Directory spec:** "`$XDG_CACHE_HOME` defines the base directory relative to which user-specific **non-essential** data files should be stored", default `$HOME/.cache`. State (history, last-opened, window layout) belongs in `$XDG_STATE_HOME` (`~/.local/state`), config in `$XDG_CONFIG_HOME`. Source: https://specifications.freedesktop.org/basedir/latest/
- **CACHEDIR.TAG spec:** a file named `CACHEDIR.TAG` whose first 43 bytes are `Signature: 8a477f597d28d172789f06886806bc55` marks the whole directory tree as "cached information that can be re-generated if necessary". The spec itself says tags are **hints**, not proof. It must be a regular file, not a symlink. Source: https://bford.info/cachedir/
- **Definition used in this catalog.** An item "regenerates with the same config" when all three hold:
  1. It is derived data (compiled shaders, HTTP/JS bytecode cache, font index, thumbnails, build output, downloaded packages that can be re-fetched).
  2. Its loss changes no setting, login, history or user-created content.
  3. The app rebuilds it automatically, without user action, on next use.
- **Two speeds of regeneration:**
  - *Fast, invisible*: fontconfig, GTK, gstreamer registry, Electron `Cache`/`Code Cache`/`GPUCache`. Rebuilt in seconds.
  - *Slow, visible*: shader caches (stutter until each shader compiles once), Rust `target/` (minutes of rebuild), package caches (re-download, needs network). Still regenerable, but the UI must say what it costs.
- **Evidence on this machine that `~/.cache` really regenerates.** `stat %w` shows `~/.cache` itself was born `2026-11-17 15:50:26`; `mesa_shader_cache` was born the same second, `fontconfig` 6 s later, `radv_builtin_shaders` 6 s later, `gtk-4.0` 17 s later, `zen` 3.5 min later. The cache was wiped/recreated and the apps rebuilt their dirs on first login with no config loss. Oldest file in `fontconfig`, `mesa_shader_cache`, `radv_builtin_shaders`, `zen` is 2026-11-17: everything there was rebuilt since then.
- **But "in ~/.cache" is not proof (exceptions found).** See rule C2 below: on this machine `~/.cache/music-opus` (8.5 GB, 2105 `.opus` tracks) is user data, and `~/.cache/yay/vcs.json` is state.

---

## 1. Measurements on this machine

### 1.1 Top level

| Location | Size |
|---|---|
| ~/.local/share | 400 GB |
| ~/Pobrane | 24 GB |
| ~/.cache | 11 GB |
| ~/.config | 4.7 GB |
| ~/.claude | 1.7 GB |
| ~/.cargo | 282 MB |
| ~/.npm | 30 MB |
| ~/.local/state | 96 KB |

Not present: `~/.rustup`, `~/.gradle`, `~/.m2`, `~/.wine`, `~/go`, `~/.bun`, `~/.deno`, `~/.yarn`, `~/.var/app` (no Flatpak app data yet), `~/.cache/thumbnails`, `~/.xsession-errors`. Rust is the system package (`/usr/bin/cargo`), no rustup.

### 1.2 ~/.cache (11 GB)

- `music-opus` **8.5 GB**: 2105 `.opus` music files, born 2026-11-19 10:15. **User data, not cache.**
- `bladephoria-web-target` 1021 MB: a Cargo `CARGO_TARGET_DIR` (has `CACHEDIR.TAG`, `debug/`, `tmp/`), born 2026-11-21.
- `yay` 724 MB: `google-chrome` 329 MB (PKGBUILD, `.deb` + built `.pkg.tar.zst` 183 MB), `horizon-launcher-bin` 299 MB (two old builds 0.2.87, 0.2.98), `git-credential-manager-bin` 58 MB, `discord-canary` 36 MB, `darkly-bin` 1 MB, plus `vcs.json` and `completion.cache`.
- `mesa_shader_cache` 515 MB (70 712 files), `radv_builtin_shaders` 2.5 MB.
- `google-chrome` 153 MB (`Default/Code Cache` 102 MB, `Default/Cache` 51 MB).
- `zen` 34 MB (`startupCache` 21 MB, `safebrowsing` 12 MB, `cache2` 1.7 MB).
- Small: `dev.stormix.deadlock-mod-manager` 18 MB, `com.epicgames.eglv2` 17 MB, `javasharedresources` 3.4 MB, `gstreamer-1.0`, `google-chrome-headless`, `fontconfig` (has `CACHEDIR.TAG`), `claude-cli-nodejs`, `gtk-4.0`, `pip`, `protonfixes`, `quickshell`, `gh`, `TelegramDesktop`, `kitty`, `hyprland`, `fish`, `event-sound-cache.tdb.*`.

### 1.3 ~/.local/share (400 GB)

| Dir | Size |
|---|---|
| Steam | 343 GB |
| PrismLauncher | 13 GB |
| horizon-launcher | 8.3 GB |
| bedrock-on-linux | 8.2 GB |
| BSManager | 5.5 GB |
| wineprefixes | 4.9 GB |
| pnpm | 2.9 GB |
| osu | 2.4 GB |
| com.epicgames.eglv2 | 2.3 GB |
| flatpak | 2.1 GB |
| basalt-launcher | 1.6 GB |
| umu | 1.5 GB |
| horizon | 1.2 GB |
| yawl | 986 MB |
| osuconfig | 731 MB |
| pipx | 490 MB |
| lutris | 472 MB |
| goverlay | 333 MB |
| uv | 215 MB |
| Trash | 147 MB |

**Steam (343 GB)**
- `steamapps/common` 306 GB: GTA V Enhanced 97 GB, CS2 70 GB, Witcher 3 69 GB, Deadlock 37 GB, Mirror's Edge Catalyst 24 GB, SteamVR 5.2 GB, Proton Experimental 1.5 GB, Proton 11.0 1.4 GB, Steam Linux Runtimes ~2.6 GB. All have manifests and were played in November. **Not junk.**
- `steamapps/shadercache` **19 GB**: 730 (CS2) 11 GB (`fozpipelinesv6` 6.8 GB, `mesa_shader_cache_sf` 4.2 GB), 1422450 (Deadlock) 6.6 GB, 3240220 (GTA V) 722 MB, 250820 empty. All four belong to installed games. The older orphans (2250500, 3527290, 2287966934, -767600689) from wave 2 are **gone now**.
- `steamapps/compatdata` **12 GB**: 20 entries.
  - Installed games: 292030 Witcher 3 2.0 GB, 1233570 Mirror's Edge 1.9 GB, 3240220 GTA V 1.3 GB, 1422450 Deadlock 443 MB, 730/250820 empty.
  - Live shortcut: 4208508988 Beat Saber 732 MB.
  - **Orphans (no manifest)**: 1304930 557 MB (GE-Proton11-7, last touched 2026-10-16; wave 2 saw it as The Outlast Trials with a paused update, the manifest is gone now), 1943950 335 MB, seven old shortcut ids 2287966934 / 2533579821 / 2582002084 / 2846320704 / 3426298747 / 3439826857 / 3527366607 at 335–349 MB each (~2.4 GB), `0` 823 MB (Proton-EM, report-only).
  - Prefixes for Proton tool appids: 4628710 (Proton 11.0) 306 MB, 1493710 (Proton Experimental) 371 MB. Unusual; report-only.
  - Manual backup `1422450.backup-before-ge-reset-20261027` 839 MB: report-only.
  - Orphan total (excluding `0`, backup, tool ids): **≈3.3 GB**.
- `compatibilitytools.d` 3.0 GB: Proton-GE Latest 1.6 GB, Proton-EM Latest 1.5 GB. Both referenced (1304930 uses GE, `0` uses EM). Not junk.
- `logs` **411 MB**, 230 files; `cef_log.previous.txt` alone is 232 MB.
- `package` 504 MB (39 client update bundles), `bootstrap.tar.xz` 408 MB, `steamrt64` 1.5 GB, `appcache` 261 MB (`librarycache` 188 MB, `httpcache` 27 MB, `stats` 18 MB, `appinfo.vdf` 28 MB), `config/htmlcache` 118 MB, `depotcache` 49 MB, `workshop` 556 MB, `userdata` 74 MB. `downloading`, `temp`, `corrupt` are empty. `config/config.vdf.async10954.tmp` 252 KB is an async-write temp.

**Other big launchers (2 levels)**
- PrismLauncher 13 GB: `java` 3.1 GB, `assets` 3.0 GB, `cache` 2.8 GB (`edge.forgecdn.net` 1.3 GB, `java` 794 MB, `ModrinthPacks` 250 MB, `bladephoriafonk.xyz` 185 MB, `FlamePacks` 132 MB), `instances` 1.8 GB, `libraries` 1.4 GB, `backups` 720 MB.
- horizon-launcher 8.3 GB: `native` 3.8 GB (of which `native/target` 3.2 GB with `CACHEDIR.TAG`, plus `source`, `build.log`), `versions` 1.8 GB, `server-runtime` 917 MB, `minecraft` 881 MB, `rust` 592 MB (bundled toolchain), `cargo` 345 MB (registry/git, `CACHEDIR.TAG`).
- bedrock-on-linux 8.2 GB: `games` 2.5 GB, `proton` 2.5 GB, `compatdata` 1.7 GB, `cache` 843 MB, `umu` 800 MB.
- BSManager 5.5 GB: `BSInstances` 5.1 GB (Beat Saber versions + songs), `SharedContent` 371 MB.
- wineprefixes 4.9 GB: `osu-wineprefix` 1.7 GB, `osu-stable` 1.6 GB, `l4d-physx-install` 637 MB, `l4d-physx-install-0408` 577 MB, `foobar2000` 549 MB, `l4d-physx-test` 8 KB.
- pnpm 2.9 GB: `store` 2.6 GB, `package-manager-store` 316 MB.
- osu (lazer) 2.4 GB: `files` 2.4 GB (beatmaps/skins, content-addressed), `cache` 52 MB.
- com.epicgames.eglv2 2.3 GB: `compat` 2.1 GB (Wine prefix), `WebKitCache` 215 MB, `CacheStorage` 25 MB.
- flatpak 2.1 GB: `repo/objects` 2.1 GB (hard links of deployed runtimes), `appstream` 111 MB, `repo/tmp` 7.7 MB. User installs: `com.hypixel.HytaleLauncher`, `org.gnome.Platform` (+Locale), `org.freedesktop.Platform.GL.default` (2 branches), `codecs-extra`.
- umu 1.5 GB (`steamrt3` 792 MB, `steamrt4` 662 MB), yawl 986 MB (`SteamLinuxRuntime_sniper` 800 MB **plus its own `.tar.xz` 186 MB**), lutris `runtime` 472 MB, goverlay `fgmod` 333 MB, pipx 490 MB (`py` has `CACHEDIR.TAG`), uv `python` 114 MB + `tools` 102 MB.
- Trash: `files` 147 MB, 1 item.

### 1.4 ~/.config (4.7 GB)

| Dir | Size |
|---|---|
| discordcanary | 1.7 GB |
| zen | 1.2 GB |
| discord | 709 MB |
| VSCodium | 386 MB |
| google-chrome | 195 MB |
| google-chrome-back-ovfs.base | 193 MB |
| chromium-back-ovfs.base | 174 MB |
| chromium | 169 MB |
| CapoRhythia | 100 MB |

- **discordcanary** (installed, running): `Cache` 855 MB (75 590 files, oldest 2026-09-04), `app-1.0.2506` 564 MB (live updater payload; coredump shows an older `app-1.0.2500` crashed), `module_data` 145 MB, `logs` 39 MB, `Dictionaries` 23 MB, `Code Cache` 20 MB, `Shared Dictionary` 13 MB, `Local Storage` 4.5 MB.
- **discord** (package `discord` NOT installed; leftover of a removed app): `app-1.0.163` 582 MB, `Cache` 51 MB, `download` 38 MB, `Code Cache` 28 MB, `Local Storage` 2.4 MB, newest Cache file 2026-10-20.
- **zen** (installed): profile `gm8yiq13.Default (release)` 1.2 GB, of which `storage/default` 902 MB (site data: IndexedDB, Cache API, localStorage), `storage/permanent` 147 MB, `extensions` 29 MB, `security_state` 25 MB, `gmp-widevinecdm` 21 MB. The real HTTP cache is in `~/.cache/zen`.
- **VSCodium** (installed, running): `WebStorage` 303 MB (`1/CacheStorage` 216 MB, `2/CacheStorage` 86 MB: webview Cache API for extensions), `CachedData` 41 MB (V8 code cache), `Cache` 10 MB, `GPUCache` 8 MB, `User` 3.4 MB (settings!).
- **google-chrome** (installed): `Default` 61 MB, `optimization_guide_model_store` 43 MB, `WasmTtsEngine` 24 MB, `Safe Browsing` 18 MB, `component_crx_cache` 13 MB. HTTP cache lives in `~/.cache/google-chrome`.
- **chromium** (package NOT installed): 169 MB leftover profile.
- **`*-back-ovfs.base`** (2 dirs, 367 MB): copies of the chromium and google-chrome profiles, both mtime 2026-11-18 08:20. The name is the overlay backup convention of profile-sync-daemon; `profile-sync-daemon` is not installed, no mount is active (`findmnt`), no `/run/user/1000/psd`. Leftover full profile copies (cookies, logins inside): **review, personal data**.
- Crash/log/cache subdirs found inside ~/.config: no `Crashpad` dirs over 1 MB; `CapoRhythia/cache` 92 MB + `logs` 2.6 MB.

### 1.5 Other

- `~/.local/state` 96 KB: wireplumber, VSCodium, ukishima wallpaper state, showtime, pill. All state, nothing to clean.
- Coredumps (`/var/lib/systemd/coredump`, outside $HOME, user-readable via ACL, **not user-deletable**): 1.2 GB, 3 files. CS2 1 GB (2026-11-18), DiscordCanary 132 MB, chrome 19 MB. Phase 1 can only report; tmpfiles ages them out after 2 weeks.
- `~/.claude` 1.7 GB (`jobs` 1.2 GB, `projects` 307 MB): the running agent's own data. Never offer.
- Dev artifacts with `CACHEDIR.TAG`:
  - `~/Pobrane/test/xu-integration/target` **11 GB**
  - `~/.local/share/horizon-launcher/native/target` 3.2 GB (launcher-managed build; review)
  - `~/.cache/bladephoria-web-target` 1021 MB
  - `~/Pobrane/test/XUninstaller/.claude/worktrees/agent-a8ac65cc5ac8d22d8/target` 333 MB (inside a stale agent worktree)
  - `~/Projects/XNetworkFilter/target` 46 MB, `~/Projects/XDNS/target` 12 MB
  - No `target/` without `CACHEDIR.TAG` found. No `node_modules` in `~/Projects` or `~/Pobrane`. Global ones: `~/.local/lib/node_modules` (npm -g installs, e.g. claude-rpc: **not junk**), `~/.npm/_npx/*/node_modules` (npx cache).
- `~/.cargo/registry` 335 MB (`src` 273 MB, `cache` 39 MB, `index` 24 MB), has `CACHEDIR.TAG`. `~/.npm/_cacache` 29 MB.
- Orphan `.desktop` files in `~/.local/share/applications` (27 total), Exec target missing:
  - `io.github.yhymid.xuninstaller.desktop` → `~/.local/bin/XUninstaller-0.1.0-x86_64.AppImage` (missing)
  - `osu-wine.desktop`, `osuwinello-file-extensions-handler.desktop`, `osuwinello-url-handler.desktop` → `~/.local/bin/osu-wine` (missing)
  - 3× `BeatSaberModManager-url-*.desktop` → `BeatSaberModManager` not in PATH
- Broken symlinks in `~/.local/bin`, `~/.local/share/applications`, `~/.config/autostart`: none. `~/.local/bin/steam` is a custom wrapper script: never touch.
- `~/.local/share/recently-used.xbel` 16 KB.

---

## 2. Rule catalog

Fields per rule: **path** · **kind** · **regenerates?** · **safe while running?** (processes to check) · **risk** · **source / reasoning**.
Risk levels: *safe* = pre-check allowed; *low* = offer, unchecked by default or with a cost note; *review* = show only, the user decides per item, never pre-checked.

### C. XDG cache

- **C1. General rule `~/.cache/<x>`**
  - kind: cache · regenerates: yes (usually same content) · running: no for the owning app · risk: low by default, safe only when a specific rule below matches.
  - Source: XDG spec ("non-essential"). Evidence: whole `~/.cache` recreated on 2026-11-17 with no config loss.
  - Unknown subdirs: never pre-check, show "unknown cache".
- **C2. Exceptions: things in ~/.cache that are NOT safe**
  - `~/.cache/music-opus` (this machine, 8.5 GB of `.opus`): user media. Heuristic: a dir in `~/.cache` whose files are mostly media/documents (`.opus .mp3 .flac .mkv .mp4 .pdf .png .jpg .zip`) with human-readable names and no `CACHEDIR.TAG` → **review, "looks like your files"**.
  - `~/.cache/yay/vcs.json`: devel-package VCS state. Losing it makes yay forget `-git` package commits; rebuilt only with `yay -Y --gendb`. Keep it; clean only package subdirs. Source: https://github.com/Jguer/yay (gendb FAQ).
  - KeePassXC (not installed here): versions before late 2023 kept last-opened databases and window state in `~/.cache/keepassxc/keepassxc.ini`; PR #9755 moved it to `~/.local/state`. Source: https://github.com/keepassxreboot/keepassxc/issues/9920 and PR #9755. Rule: review.
  - Spotify (not installed): `~/.cache/spotify/Storage` holds offline downloads (re-download needs Premium + network). Rule: low, cost note. Reasoning, not verified here.
  - `~/.cache/<x>-target` / `CARGO_TARGET_DIR` (here `bladephoria-web-target`): build output, regenerable but slow; handled by rule D1.
  - `~/.cache/claude-cli-nodejs`: logs of a running tool. Skip while `claude` runs.
  - `~/.cache/gh`: gh CLI HTTP cache; auth lives in `~/.config/gh/hosts.yml` (not in cache). Safe.
- **C3. Thumbnails `~/.cache/thumbnails/{normal,large,x-large,xx-large,fail}`**
  - kind: cache · regenerates: yes, same content · running: yes · risk: safe.
  - Source: freedesktop Thumbnail Managing Standard, https://specifications.freedesktop.org/thumbnail-spec/latest/ . Not present on this machine.
- **C4. fontconfig `~/.cache/fontconfig`**
  - cache · yes, same (rebuilt on next font lookup or `fc-cache`) · yes · safe. Has `CACHEDIR.TAG`. Rebuilt 6 s after login here.
- **C5. GTK/GStreamer/glycin/kitty/hyprland/quickshell/fish/event-sound small caches**
  - cache · yes, same · gstreamer registry yes, others no for owner app (`Hyprland`, `kitty`, `quickshell`, `fish`) · safe but tiny. Low value: group them, don't list each.
- **C6. `~/.cache/javasharedresources`** (OpenJ9/Java shared class cache): cache · yes · no (`java`) · safe.
- **C7. `~/.cache/pip`, `~/.cache/uv`, `~/.cache/pypoetry`, `~/.cache/go-build`**
  - package-cache / build cache · yes (re-download/rebuild) · no (`pip`, `uv`, `go`) · safe.
  - Sources: https://pip.pypa.io/en/stable/topics/caching/ , https://docs.astral.sh/uv/concepts/cache/ (`uv cache clean`), `go help cache` (`go clean -cache`). Prefer the tool's own clean command when present.

### G. GPU shader caches

- **G1. Mesa `~/.cache/mesa_shader_cache`, `mesa_shader_cache_sf`, `mesa_shader_cache_db`**
  - cache · yes, slow rebuild (stutter on first use of each shader) · no: any GPU app holds it open (check: process has the dir open via `/proc/*/fd`; at least `Hyprland`, `steam`, games) · risk: low.
  - Mesa caps multi-file cache at 1 GB by default (`MESA_SHADER_CACHE_MAX_SIZE`); the single-file variant has no size limit. So 515 MB here is self-managed; offering it gains little. Source: https://docs.mesa3d.org/envvars.html
- **G2. `~/.cache/radv_builtin_shaders`**: cache · yes, fast · no (GPU apps) · safe, tiny.
- **G3. NVIDIA `~/.cache/nvidia/GLCache`, `~/.nv/ComputeCache`**: cache · yes, slow · no · low. Not present (AMD APU).
- **G4. DXVK `*.dxvk-cache`, vkd3d `vkd3d-proton.cache*`**: cache · yes, slow · no (game) · low. Usually inside game dir or prefix; Steam keeps them in `shadercache/<id>/DXVK_state_cache` (all 0 bytes here). Stray `~/.local/share/Steam/d3ddriverquery64.dxvk-cache` is 12 bytes.
- **G5. Steam `steamapps/shadercache/<appid>`**
  - cache · yes, slow: Steam re-downloads/replays pre-cached Fossilize data, games stutter until rebuilt · running: no (`steam`, `steamwebhelper`, `fossilize_replay`, `reaper`) · risk: installed game = **low with stutter warning**, orphan appid (no manifest, no shortcut) = safe.
  - Sources: forum reports that clearing shader caches brings stutter back until rebuilt (https://steamcommunity.com/app/1675200/discussions/0/4038102303460802607 , https://github.com/ValveSoftware/steam-for-linux/issues/11392). Valve does not document it officially.
  - Normalise signed/unsigned appid (wave 2 §03).

### S. Steam client (root `~/.local/share/Steam`, reached via `~/.steam/root`)

All S-rules: **only when Steam is not running** (`pgrep -x steam`, `steamwebhelper`). Steam was running during this measurement.

- **S1. `logs/*`** (411 MB here): log · regenerates (new logs) · no · safe. Keep the current file set if Steam is running anyway.
- **S2. `appcache/httpcache`**: cache · yes, same · no · safe. `appcache/librarycache` (artwork): cache · yes, re-downloaded · no · safe. **Never** `appcache/appinfo.vdf`, `packageinfo.vdf`, `stats` (Steam rebuilds them, but slow and causes a long "updating" start; low value).
- **S3. `config/htmlcache`** (118 MB, CEF browser cache): cache · yes · no · low. Contains web session cookies for the store pages; deleting may log out of web views, not of Steam itself (reasoning, not verified).
- **S4. `depotcache/*.manifest`**: cache · yes, re-downloaded on update · no · low.
- **S5. `steamapps/downloading/*`, `steamapps/temp/*`, `steamapps/corrupt/*`**: leftover · no (partial downloads restart) · no · safe if the app has no active download (StateFlags bit 1<<20 / 1024 unset). Empty here.
- **S6. `package/*`** (504 MB client update bundles), `bootstrap.tar.xz`: package-cache · Steam re-downloads on next start · no · review. Breaking them can force a full client re-download; not worth the risk for <1 GB.
- **S7. Dumps: `/tmp/dumps` (outside HOME), `Steam/dumps`, `*.dmp`**: dump · no · yes · safe.
- **S8. `config/*.async*.tmp`**: leftover atomic-write temp · no · no · safe when Steam is closed.
- **S9. `steamapps/compatdata/<appid>` orphan** (no manifest, no shortcut): leftover · **no** (prefix recreated empty; saves are lost) · no (`steam`, `wineserver`, `reaper`) · **review**, with "saves may be inside `pfx/drive_c/users/steamuser`". Never for `0`, `*.backup-*`, Proton tool appids. Rules in wave 2 §03.

### W. Proton / Wine / launchers

- **W1. Wine prefix temp `pfx/drive_c/users/steamuser/AppData/Local/Temp/*`, `drive_c/windows/temp/*`** (in any prefix): cache/temp · no need · no (`wineserver`) · safe.
- **W2. Whole Wine prefixes** (`~/.local/share/wineprefixes/*`, launcher `compat`): **review** always. `l4d-physx-install*` (1.2 GB) and `l4d-physx-test` look like test leftovers by name only.
- **W3. Launcher runtimes** (`umu/steamrt*`, `yawl/SteamLinuxRuntime_sniper`, `lutris/runtime`): package-cache · yes, re-downloaded by the launcher · no (`umu-run`, `yawl`, `lutris`) · low. Exception: `yawl/SteamLinuxRuntime_sniper.tar.xz` (186 MB) beside the extracted dir is a download archive; low.
- **W4. Launcher download caches**: `PrismLauncher/cache` (2.8 GB: mod/modpack/Java downloads), `bedrock-on-linux/cache` (843 MB), `osu/cache` (52 MB), `com.epicgames.eglv2/WebKitCache` + `CacheStorage` (240 MB), `CapoRhythia/cache` (92 MB): cache · yes, re-downloaded · no (`prismlauncher`, `java`, `osu!`, launcher name) · low. Sources: Prism Launcher keeps HTTP cache in `cache/` with `metacache` index (reasoning from layout); verify per launcher before marking safe.
- **W5. Launcher bundled build trees**: `horizon-launcher/native/target` (3.2 GB, `CACHEDIR.TAG`), `horizon-launcher/cargo/{registry,git}`: build · yes, slow · no · review (the launcher may expect them to exist; unknown).

### E. Electron / Chromium apps (`~/.config/<app>`)

Running check: `SingletonLock` → `<host>-<pid>` with live pid, or `pgrep -x` of the app (here `DiscordCanary`, `codium` were running). Rules from wave 2 §06.

- **Safe subdirs (cache, regenerates same):** `Cache`, `Code Cache`, `GPUCache`, `DawnCache`, `DawnGraphiteCache`, `DawnWebGPUCache`, `GrShaderCache`, `ShaderCache`, `GPUPersistentCache`, `Service Worker/CacheStorage`, `Service Worker/ScriptCache`, `component_crx_cache`, VSCode-family `CachedData`, `CachedExtensionVSIXs`, `logs`, `Crashpad/completed`, `Crashpad/pending`, `Crash Reports`. Source: Chromium disk-cache/code-cache design, Electron `app.getPath('sessionData')` docs https://www.electronjs.org/docs/latest/api/app .
- **Low (regenerates but costs re-download):** `Dictionaries` (hunspell), `Shared Dictionary`, `optimization_guide_model_store`, `WasmTtsEngine`, `OnDeviceHeadSuggestModel`, VSCodium `WebStorage/*/CacheStorage` (webview Cache API of extensions; extension webviews re-fetch).
- **NEVER:** `Local Storage`, `IndexedDB`, `Session Storage`, `Cookies*`, `Login Data*`, `Web Data`, `Preferences`, `Local State`, `Network Persistent State`, `TransportSecurity`, `Trust Tokens`, `blob_storage`, `databases`, `WebStorage/QuotaManager`, `User/` (VSCode settings, keybindings, snippets), `Crashpad/settings.dat`, `Discord Key Storage`, Discord `app-*`/`module_data`/`installer.db` of an installed Discord.
- **Discord updater payload:** old `app-1.0.NNNN` dirs other than the newest one: leftover · no · no (`DiscordCanary`) · low. Here only one exists.
- **Leftover whole profile of an uninstalled app** (`~/.config/discord` 709 MB, `~/.config/chromium` 169 MB, `*-back-ovfs.base` 367 MB): leftover · no · n/a · **review**: they hold logins and history. Offer only after the user confirms the app is gone (pacman says not installed).

### B. Browsers

- **Firefox/Zen/LibreWolf:** HTTP cache is in `~/.cache/<browser>/<profile>/cache2`, `startupCache`, `safebrowsing`, `jumpListCache`, `thumbnails`: cache · yes, same · no (`firefox`, `zen`, `zen-bin`) · safe. Inside `~/.config/zen/<profile>` (or `~/.mozilla/firefox/<profile>`) **only** `storage/default/*/cache` (Cache API) is cache-like, and even that is low; everything else (`places.sqlite`, `logins.json`, `key4.db`, `cookies.sqlite`, `storage/default/*/idb`, `extensions`, `sessionstore*`) is NEVER. Source: https://support.mozilla.org/kb/profiles-where-firefox-stores-user-data .
- **Chrome/Chromium/Brave:** `~/.cache/google-chrome/<profile>/{Cache,Code Cache}` (153 MB here): cache · yes · no (`chrome`) · safe. In `~/.config/google-chrome` apply the Electron lists above per profile (`Default`, `Profile N`).

### P. Package managers and dev tools

- **P1. yay/paru/pikaur `~/.cache/{yay,paru/clone,pikaur}/<pkg>`**: package-cache/build · yes (re-clone, rebuild) · no (`yay`, `paru`, `pikaur`, `makepkg`) · safe for dirs whose package is not installed; low for installed ones (old `*.pkg.tar.zst` are the only local downgrade copy). Keep `yay/vcs.json` and `completion.cache` unless asked.
- **P2. Cargo `~/.cargo/registry/{cache,src}`, `~/.cargo/git/checkouts`**: package-cache · yes, re-download · no (`cargo`, `rustc`, `rust-analyzer`) · safe; `registry/index` low (slow first resolve). Never `~/.cargo/bin`, `~/.cargo/config.toml`, `credentials.toml`. Source: https://doc.rust-lang.org/cargo/guide/cargo-home.html (cache "can be cleared").
- **P3. rustup `~/.rustup/{downloads,tmp}`**: package-cache · yes · no · safe. `toolchains` = review. Not present here.
- **P4. npm `~/.npm/_cacache`, `~/.npm/_npx`, `~/.npm/_logs`**: package-cache/log · yes · no (`npm`, `node`, `npx`) · safe. Source: https://docs.npmjs.com/cli/commands/npm-cache . Never `~/.local/lib/node_modules` (global installs).
- **P5. pnpm store `~/.local/share/pnpm/store`** (2.6 GB): package-cache, **hard-linked** into projects · yes · no (`pnpm`, `node`) · low. Use `pnpm store prune` semantics (remove only unreferenced packages); deleting the store outright breaks nothing on disk but frees only files with `nlink==1`. Source: https://pnpm.io/cli/store .
- **P6. pip `~/.cache/pip`, uv `~/.cache/uv`, pipx `~/.local/share/pipx/{py,.cache}`**: package-cache · yes · no · safe. Never `pipx/venvs`, `uv/tools`, `uv/python` (installed tools; review).
- **P7. Go `~/.cache/go-build`** safe; `~/go/pkg/mod` low (read-only files, needs `go clean -modcache`). Not present.
- **P8. Gradle `~/.gradle/caches`, `~/.gradle/wrapper/dists`** low; Maven `~/.m2/repository` low (offline builds break). Not present.
- **P9. Flatpak user**: `~/.local/share/flatpak/repo/tmp` (7.7 MB) leftover · safe when no `flatpak` process runs; unused runtimes → recommend `flatpak uninstall --user --unused` (do not delete `repo/objects` by hand: hard links, corrupts the repo). Source: https://docs.flatpak.org/en/latest/flatpak-command-reference.html . `~/.var/app/<id>/cache`: cache · yes · no (`bwrap` with that app id, `flatpak ps`) · safe. None exist here.
- **P10. Build dirs `target/` with valid `CACHEDIR.TAG`, `node_modules/` next to `package.json`, `__pycache__`, `.pytest_cache`, `.mypy_cache`, `.ruff_cache`**: build · yes, slow (rebuild/reinstall) · no (`cargo`, `rustc`, `node`, `pnpm`, `npm`, `rust-analyzer`) · safe for `CACHEDIR.TAG` dirs not modified in the last N days, else low. Never descend into `.git`. Show the project name and last build time.

### L. Logs, history, trash, integration debris

- **L1. `~/.local/state/*/log*`, `~/.local/share/*/logs`, `~/.xsession-errors*`**: log · regenerates · yes for rotated files (`*.old`, `*.1`, `*.previous*`), no for the current file · safe for rotated, low for current. Here: only `goverlay/logs`, `osu/logs`, `yawl/yawl.log`, Steam logs.
- **L2. `~/.local/share/recently-used.xbel`**: history (privacy) · regenerates empty · yes (GTK rewrites it) · low, label "privacy: clears recent files list", not "junk".
- **L3. Trash `~/.local/share/Trash/{files,info,expunged}`**: trash · no · yes · low: list item names and dates; the user may have trashed something by mistake. Remove `files/X` and `info/X.trashinfo` together. Source: https://specifications.freedesktop.org/trash-spec/latest/ .
- **L4. Broken symlinks** in `~/.local/bin`, `~/.local/share/applications`, `~/.config/autostart`, `~/.local/share/icons`: leftover · no · yes · safe (unlink the link, never follow). None here.
- **L5. Orphan `.desktop` files** (Exec binary missing, TryExec missing): leftover · no · yes · low (a removable drive or AppImage folder may be just unmounted). Here 7 files (XUninstaller AppImage, osu-wine ×3, BeatSaberModManager ×3). Also check `mimeapps.list` lines pointing at them (edit lines, never delete the file).
- **L6. Coredumps** (`/var/lib/systemd/coredump`): dump · no · n/a · report-only in Phase 1 (root-owned).

---

## 3. DANGER list (never offer, derived from this machine)

Hard-block paths and patterns (not shown as candidates, and blocked as parents of a selection):
- Whole `~/.config`, `~/.local`, `~/.local/share`, `~/.cache`, `~/.steam`, `~/.local/share/Steam` (the owner already lost everything once by deleting `~/.config`). XCleaner deletes only leaf rules, never a top-level XDG dir.
- Secrets: `~/.ssh`, `~/.gnupg`, `~/.pki` (NSS db), `~/.local/share/keyrings`, `~/.config/Discord Key Storage`, `~/.config/gh`, `~/.npmrc`, `~/.cargo/credentials*`, `~/.claude.json`, `~/.codex`, any `*.key`, `*token*`, `*wallet*`, password-manager dirs (keepassxc, Bitwarden, 1Password if present).
- Shell/desktop config: `~/.bashrc`, `~/.bash_history`, `~/.config/hypr`, `~/.config/fish`, `~/.config/kitty`, `~/.config/quickshell`, `~/.config/dconf`, `~/.config/systemd`, `~/.config/autostart`, `~/.config/mimeapps.list`, `~/.local/state/*`, `~/.local/bin/*` (incl. custom `steam` wrapper), `~/.local/lib/node_modules`.
- Steam: `config/` (config.vdf, loginusers.vdf), `userdata/` (cloud saves, localconfig, shortcuts.vdf), `steamapps/*.acf`, `libraryfolders.vdf`, `steamapps/common` (games), `compatibilitytools.d` (in use), `steamrt64`, `ubuntu12_*`, `appcache/*.vdf`, `compatdata/<installed id>` (e.g. 292030 Witcher 3 saves live inside the prefix), `compatdata/0`, `compatdata/*.backup-*`, `workshop/content/241100`.
- Game data/saves: `PrismLauncher/{instances,backups,accounts.json}`, `horizon/{saves,instances}`, `com.megalithofficial.basalt-launcher/{instances,basalt.db}`, `bedrock-on-linux/games`, `BSManager/BSInstances`, `osu/files` + `client.realm` (lazer library), `wineprefixes/osu-*` (osu!stable songs/skins), `osuconfig`, `CapoRhythia/*.json` + `rhythia.db`, `lutris/pga.db`, `heroic`, launcher `compat` prefixes.
- Browser/app profiles except cache subdirs: `~/.config/zen/*` (except rules in B), `~/.config/google-chrome/*`, `~/.config/VSCodium/User`, `~/.vscode-oss` (extensions), `~/.config/Vencord`, `~/.config/discordcanary` non-cache parts.
- Package/runtime data: `~/.local/share/flatpak` (except `repo/tmp`), `pipx/venvs`, `uv/tools`, `uv/python`, `~/.cargo/bin`.
- Agent/tooling: `~/.claude`, `~/.claude-rpc`, `~/.codex`, `~/.semgrep`.
- User files: `~/Pictures`, `~/Videos`, `~/Games`, `~/Applications` (AppImages), `~/backups`, `~/benchmarks`, `~/Pobrane` except build-artifact rules, and `~/.cache/music-opus`.

Guard rules:
- Never follow symlinks; refuse world-writable parents (HOME is 0777 here, wave 2).
- Refuse when the owning process runs.
- Refuse a path whose canonical form leaves `$HOME`.

---

## 4. "Regenerates with the same config": concrete findings here

| Dir | Size | Evidence |
|---|---|---|
| ~/.cache/fontconfig | 692 KB | TAG, reborn 11-17 |
| ~/.cache/mesa_shader_cache | 515 MB | reborn 11-17 |
| ~/.cache/radv_builtin_shaders | 2.5 MB | reborn 11-17 |
| ~/.cache/gtk-4.0 | 280 KB | reborn 11-17 |
| ~/.cache/gstreamer-1.0 | 1.6 MB | registry, 1 file |
| ~/.cache/google-chrome | 153 MB | Chromium cache |
| ~/.cache/zen | 34 MB | reborn 11-17 |
| ~/.cache/yay/* pkgs | 723 MB | AUR rebuildable |
| ~/.cargo/registry | 335 MB | CACHEDIR.TAG |
| ~/.npm/_cacache | 29 MB | npm docs |
| pipx/py | small | CACHEDIR.TAG |
| discordcanary/Cache | 855 MB | Chromium cache |
| discordcanary/Code Cache | 20 MB | V8 cache |
| VSCodium/CachedData | 41 MB | V8 cache |
| Steam logs | 411 MB | logs |
| Steam appcache/httpcache | 27 MB | HTTP cache |
| Steam config/htmlcache | 118 MB | CEF cache |
| xu-integration/target | 11 GB | CACHEDIR.TAG |
| bladephoria-web-target | 1 GB | CACHEDIR.TAG |
| PrismLauncher/cache | 2.8 GB | download cache |

Notes:
- "Reborn 11-17" = birth time within seconds to minutes of `~/.cache` itself (2026-11-17 15:50:26), with file contents only from 11-17 onward. That shows the app rebuilt it without user action.
- `discordcanary/Cache` oldest file 2026-09-04 and 75k files: Chromium's simple cache is size-capped but large here. It regenerates on use (re-downloads avatars/emoji/media).
- Steam `shadercache/*` (19 GB) and `mesa_shader_cache` also regenerate, but **not invisibly**: stutter in CS2/Deadlock until rebuilt. Report as "slow rebuild".

---

## 5. Top 15 reclaimable items on this machine

| # | Item | Size | Risk |
|---|---|---|---|
| 1 | shadercache/730 CS2 | 11 GB | low, stutter |
| 2 | xu-integration/target | 11 GB | safe |
| 3 | ~/.cache/music-opus | 8.5 GB | review, user |
| 4 | shadercache/1422450 Deadlock | 6.6 GB | low, stutter |
| 5 | Orphan compatdata (9) | 3.3 GB | review, saves |
| 6 | horizon native/target | 3.2 GB | review |
| 7 | PrismLauncher/cache | 2.8 GB | low |
| 8 | ~/.cache/bladephoria-web-target | 1 GB | safe |
| 9 | discordcanary/Cache | 855 MB | safe (closed) |
| 10 | bedrock-on-linux/cache | 843 MB | low |
| 11 | Leftover profiles (discord, chromium, ovfs) | 1.2 GB | review |
| 12 | ~/.cache/yay packages | 723 MB | safe |
| 13 | shadercache/3240220 GTA V | 722 MB | low, stutter |
| 14 | Steam logs | 411 MB | safe (closed) |
| 15 | VSCodium WebStorage CacheStorage | 302 MB | low |

Honourable mentions: `~/.cache/mesa_shader_cache` 515 MB (low), `~/.cargo/registry` 312 MB (`src`+`cache`, safe), XUninstaller worktree target 333 MB (safe), `l4d-physx-*` prefixes 1.2 GB (review), Steam `config/htmlcache` 118 MB, yawl `.tar.xz` 186 MB, Trash 147 MB (low), `~/.cache/google-chrome` 153 MB (safe).

- **Totals:**
  - Safe or low with no data loss (rows 1, 2, 4, 7, 8, 9, 10, 12, 13, 14, 15 plus mentions): **≈ 38 GB**. Of that, ~19 GB is shader caches that cost stutter.
  - Without shader caches: **≈ 19 GB**.
  - Review-only rows (3, 5, 6, 11): ≈ 16 GB more, user decides.
- **Not reclaimable as junk:** the 306 GB of installed games. The real space lever is uninstalling games (XUninstaller), not cleaning.

---

## Unverified assumptions / limits

- Steam behaviour when `htmlcache`, `depotcache`, `package` are deleted is from community reports and reasoning, not tested.
- The `-back-ovfs.base` origin (profile-sync-daemon overlay mode) is inferred from the name; psd is not installed now.
- Prism/bedrock/horizon launcher cache semantics are inferred from directory layout, not from their source code.
- Sizes are `du` apparent-on-disk with btrfs; compression/reflinks may make real freed space smaller.
- Coredumps were only listed (`coredumpctl`), not stat'ed per file.
