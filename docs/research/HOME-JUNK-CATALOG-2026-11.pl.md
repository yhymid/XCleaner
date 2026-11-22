# Katalog śmieci w $HOME (research, 2026-11-22)

[English](HOME-JUNK-CATALOG-2026-11.md) | **Polski**

Zakres: Faza 1, tylko `$HOME`. Pomiary tylko do odczytu na maszynie właściciela (Steam Deck OLED, CachyOS, kernel deckify 7.3.4, Hyprland). `/home` to btrfs `@home`, 475 GB, **460 GB zajęte, 15 GB wolne (98%)**.
Metoda: `du -xsh`, `find`, `stat` (z czasem utworzenia `%w`), `pgrep -x`, `coredumpctl list`, `flatpak list`. Nie czytano treści plików poza kluczami `name`/`LastPlayed` z manifestów Steam i `config_info` Protona (pierwsze linie: wersja Protona, ścieżki).
Wykorzystane, nie powtarzane: `~/Pobrane/test/xu-integration/docs/research/RESEARCH-WAVE2-2026-11.md` (sekcje 03 Steam, 06 Electron, 14 coredumpy).

Działające w czasie pomiaru: `steam` (2), `steamwebhelper` (10), `DiscordCanary` (9), `codium` (9), `Hyprland`, `kitty`. Niedziałające: zen, chrome, prismlauncher, java, wineserver, cs2.

Zastrzeżenie co do rozmiarów: `du` liczy plik z twardymi linkami raz na wywołanie. Flatpak (`repo/objects` kontra `runtime/`) i pnpm (`store` kontra `node_modules` projektów) używają twardych linków, więc usunięcie jednej strony zwalnia mało albo nic. XCleaner musi liczyć „do odzyskania” po liczbie linków i-węzła (`st_nlink == 1` albo wszystkie linki wewnątrz zaznaczenia).

---

## 0. Pojęcie: „śmieci, które odtwarzają się przy tej samej konfiguracji”

- **Specyfikacja XDG Base Directory:** „`$XDG_CACHE_HOME` defines the base directory relative to which user-specific **non-essential** data files should be stored”, domyślnie `$HOME/.cache`. Stan (historia, ostatnio otwierane, układ okien) należy do `$XDG_STATE_HOME` (`~/.local/state`), konfiguracja do `$XDG_CONFIG_HOME`. Źródło: https://specifications.freedesktop.org/basedir/latest/
- **Specyfikacja CACHEDIR.TAG:** plik o nazwie `CACHEDIR.TAG`, którego pierwsze 43 bajty to `Signature: 8a477f597d28d172789f06886806bc55`, oznacza całe drzewo katalogów jako „cached information that can be re-generated if necessary”. Sama specyfikacja mówi, że znaczniki są **wskazówkami**, a nie dowodem. Musi to być zwykły plik, nie symlink. Źródło: https://bford.info/cachedir/
- **Definicja używana w tym katalogu.** Pozycja „odtwarza się przy tej samej konfiguracji”, gdy spełnione są wszystkie trzy warunki:
  1. To dane pochodne (skompilowane shadery, cache HTTP/bajtkodu JS, indeks czcionek, miniatury, wyniki budowania, pobrane paczki, które da się pobrać ponownie).
  2. Ich utrata nie zmienia żadnego ustawienia, logowania, historii ani treści stworzonej przez użytkownika.
  3. Aplikacja odbudowuje je sama, bez działania użytkownika, przy następnym użyciu.
- **Dwie szybkości odtwarzania:**
  - *Szybko, niezauważalnie*: fontconfig, GTK, rejestr gstreamera, Electron `Cache`/`Code Cache`/`GPUCache`. Odbudowa w kilka sekund.
  - *Wolno, zauważalnie*: cache shaderów (przycinanie, dopóki każdy shader nie skompiluje się raz), `target/` Rusta (minuty przebudowy), cache paczek (ponowne pobieranie, wymaga sieci). Nadal da się je odtworzyć, ale UI musi powiedzieć, ile to kosztuje.
- **Dowód na tej maszynie, że `~/.cache` naprawdę się odtwarza.** `stat %w` pokazuje, że sam `~/.cache` powstał `2026-11-17 15:50:26`; `mesa_shader_cache` powstał w tej samej sekundzie, `fontconfig` 6 s później, `radv_builtin_shaders` 6 s później, `gtk-4.0` 17 s później, `zen` 3,5 min później. Cache został wyczyszczony i utworzony od nowa, a aplikacje odbudowały swoje katalogi przy pierwszym logowaniu bez utraty konfiguracji. Najstarszy plik w `fontconfig`, `mesa_shader_cache`, `radv_builtin_shaders` i `zen` pochodzi z 2026-11-17: wszystko tam zostało od tego czasu odbudowane.
- **Ale „w ~/.cache” to nie dowód (znalezione wyjątki).** Patrz reguła C2 niżej: na tej maszynie `~/.cache/music-opus` (8,5 GB, 2105 utworów `.opus`) to dane użytkownika, a `~/.cache/yay/vcs.json` to stan.

---

## 1. Pomiary na tej maszynie

### 1.1 Najwyższy poziom

| Lokalizacja | Rozmiar |
|---|---|
| ~/.local/share | 400 GB |
| ~/Pobrane | 24 GB |
| ~/.cache | 11 GB |
| ~/.config | 4,7 GB |
| ~/.claude | 1,7 GB |
| ~/.cargo | 282 MB |
| ~/.npm | 30 MB |
| ~/.local/state | 96 KB |

Nie ma: `~/.rustup`, `~/.gradle`, `~/.m2`, `~/.wine`, `~/go`, `~/.bun`, `~/.deno`, `~/.yarn`, `~/.var/app` (jeszcze brak danych aplikacji Flatpak), `~/.cache/thumbnails`, `~/.xsession-errors`. Rust to pakiet systemowy (`/usr/bin/cargo`), bez rustup.

### 1.2 ~/.cache (11 GB)

- `music-opus` **8,5 GB**: 2105 plików muzycznych `.opus`, utworzony 2026-11-19 10:15. **Dane użytkownika, nie cache.**
- `bladephoria-web-target` 1021 MB: `CARGO_TARGET_DIR` Cargo (ma `CACHEDIR.TAG`, `debug/`, `tmp/`), utworzony 2026-11-21.
- `yay` 724 MB: `google-chrome` 329 MB (PKGBUILD, `.deb` + zbudowany `.pkg.tar.zst` 183 MB), `horizon-launcher-bin` 299 MB (dwa stare buildy 0.2.87, 0.2.98), `git-credential-manager-bin` 58 MB, `discord-canary` 36 MB, `darkly-bin` 1 MB, do tego `vcs.json` i `completion.cache`.
- `mesa_shader_cache` 515 MB (70 712 plików), `radv_builtin_shaders` 2,5 MB.
- `google-chrome` 153 MB (`Default/Code Cache` 102 MB, `Default/Cache` 51 MB).
- `zen` 34 MB (`startupCache` 21 MB, `safebrowsing` 12 MB, `cache2` 1,7 MB).
- Małe: `dev.stormix.deadlock-mod-manager` 18 MB, `com.epicgames.eglv2` 17 MB, `javasharedresources` 3,4 MB, `gstreamer-1.0`, `google-chrome-headless`, `fontconfig` (ma `CACHEDIR.TAG`), `claude-cli-nodejs`, `gtk-4.0`, `pip`, `protonfixes`, `quickshell`, `gh`, `TelegramDesktop`, `kitty`, `hyprland`, `fish`, `event-sound-cache.tdb.*`.

### 1.3 ~/.local/share (400 GB)

| Katalog | Rozmiar |
|---|---|
| Steam | 343 GB |
| PrismLauncher | 13 GB |
| horizon-launcher | 8,3 GB |
| bedrock-on-linux | 8,2 GB |
| BSManager | 5,5 GB |
| wineprefixes | 4,9 GB |
| pnpm | 2,9 GB |
| osu | 2,4 GB |
| com.epicgames.eglv2 | 2,3 GB |
| flatpak | 2,1 GB |
| basalt-launcher | 1,6 GB |
| umu | 1,5 GB |
| horizon | 1,2 GB |
| yawl | 986 MB |
| osuconfig | 731 MB |
| pipx | 490 MB |
| lutris | 472 MB |
| goverlay | 333 MB |
| uv | 215 MB |
| Trash | 147 MB |

**Steam (343 GB)**
- `steamapps/common` 306 GB: GTA V Enhanced 97 GB, CS2 70 GB, Witcher 3 69 GB, Deadlock 37 GB, Mirror's Edge Catalyst 24 GB, SteamVR 5,2 GB, Proton Experimental 1,5 GB, Proton 11.0 1,4 GB, Steam Linux Runtimes ~2,6 GB. Wszystkie mają manifesty i były uruchamiane w listopadzie. **To nie śmieci.**
- `steamapps/shadercache` **19 GB**: 730 (CS2) 11 GB (`fozpipelinesv6` 6,8 GB, `mesa_shader_cache_sf` 4,2 GB), 1422450 (Deadlock) 6,6 GB, 3240220 (GTA V) 722 MB, 250820 pusty. Wszystkie cztery należą do zainstalowanych gier. Starsze sieroty (2250500, 3527290, 2287966934, -767600689) z fali 2 **już zniknęły**.
- `steamapps/compatdata` **12 GB**: 20 wpisów.
  - Zainstalowane gry: 292030 Witcher 3 2,0 GB, 1233570 Mirror's Edge 1,9 GB, 3240220 GTA V 1,3 GB, 1422450 Deadlock 443 MB, 730/250820 puste.
  - Aktywny skrót: 4208508988 Beat Saber 732 MB.
  - **Sieroty (bez manifestu)**: 1304930 557 MB (GE-Proton11-7, ostatnio ruszany 2026-10-16; fala 2 widziała go jako The Outlast Trials z wstrzymaną aktualizacją, manifestu już nie ma), 1943950 335 MB, siedem starych identyfikatorów skrótów 2287966934 / 2533579821 / 2582002084 / 2846320704 / 3426298747 / 3439826857 / 3527366607 po 335–349 MB każdy (~2,4 GB), `0` 823 MB (Proton-EM, tylko raport).
  - Prefiksy dla appid narzędzi Protona: 4628710 (Proton 11.0) 306 MB, 1493710 (Proton Experimental) 371 MB. Nietypowe; tylko raport.
  - Ręczna kopia `1422450.backup-before-ge-reset-20261027` 839 MB: tylko raport.
  - Suma sierot (bez `0`, kopii i identyfikatorów narzędzi): **≈3,3 GB**.
- `compatibilitytools.d` 3,0 GB: Proton-GE Latest 1,6 GB, Proton-EM Latest 1,5 GB. Oba używane (1304930 używa GE, `0` używa EM). To nie śmieci.
- `logs` **411 MB**, 230 plików; sam `cef_log.previous.txt` ma 232 MB.
- `package` 504 MB (39 paczek aktualizacji klienta), `bootstrap.tar.xz` 408 MB, `steamrt64` 1,5 GB, `appcache` 261 MB (`librarycache` 188 MB, `httpcache` 27 MB, `stats` 18 MB, `appinfo.vdf` 28 MB), `config/htmlcache` 118 MB, `depotcache` 49 MB, `workshop` 556 MB, `userdata` 74 MB. `downloading`, `temp` i `corrupt` są puste. `config/config.vdf.async10954.tmp` 252 KB to plik tymczasowy zapisu asynchronicznego.

**Inne duże launchery (2 poziomy)**
- PrismLauncher 13 GB: `java` 3,1 GB, `assets` 3,0 GB, `cache` 2,8 GB (`edge.forgecdn.net` 1,3 GB, `java` 794 MB, `ModrinthPacks` 250 MB, `bladephoriafonk.xyz` 185 MB, `FlamePacks` 132 MB), `instances` 1,8 GB, `libraries` 1,4 GB, `backups` 720 MB.
- horizon-launcher 8,3 GB: `native` 3,8 GB (z czego `native/target` 3,2 GB z `CACHEDIR.TAG`, do tego `source`, `build.log`), `versions` 1,8 GB, `server-runtime` 917 MB, `minecraft` 881 MB, `rust` 592 MB (dołączony toolchain), `cargo` 345 MB (registry/git, `CACHEDIR.TAG`).
- bedrock-on-linux 8,2 GB: `games` 2,5 GB, `proton` 2,5 GB, `compatdata` 1,7 GB, `cache` 843 MB, `umu` 800 MB.
- BSManager 5,5 GB: `BSInstances` 5,1 GB (wersje Beat Saber + piosenki), `SharedContent` 371 MB.
- wineprefixes 4,9 GB: `osu-wineprefix` 1,7 GB, `osu-stable` 1,6 GB, `l4d-physx-install` 637 MB, `l4d-physx-install-0408` 577 MB, `foobar2000` 549 MB, `l4d-physx-test` 8 KB.
- pnpm 2,9 GB: `store` 2,6 GB, `package-manager-store` 316 MB.
- osu (lazer) 2,4 GB: `files` 2,4 GB (beatmapy/skórki, adresowane treścią), `cache` 52 MB.
- com.epicgames.eglv2 2,3 GB: `compat` 2,1 GB (prefiks Wine), `WebKitCache` 215 MB, `CacheStorage` 25 MB.
- flatpak 2,1 GB: `repo/objects` 2,1 GB (twarde linki wdrożonych runtime’ów), `appstream` 111 MB, `repo/tmp` 7,7 MB. Instalacje użytkownika: `com.hypixel.HytaleLauncher`, `org.gnome.Platform` (+Locale), `org.freedesktop.Platform.GL.default` (2 gałęzie), `codecs-extra`.
- umu 1,5 GB (`steamrt3` 792 MB, `steamrt4` 662 MB), yawl 986 MB (`SteamLinuxRuntime_sniper` 800 MB **plus własny `.tar.xz` 186 MB**), lutris `runtime` 472 MB, goverlay `fgmod` 333 MB, pipx 490 MB (`py` ma `CACHEDIR.TAG`), uv `python` 114 MB + `tools` 102 MB.
- Trash: `files` 147 MB, 1 pozycja.

### 1.4 ~/.config (4,7 GB)

| Katalog | Rozmiar |
|---|---|
| discordcanary | 1,7 GB |
| zen | 1,2 GB |
| discord | 709 MB |
| VSCodium | 386 MB |
| google-chrome | 195 MB |
| google-chrome-back-ovfs.base | 193 MB |
| chromium-back-ovfs.base | 174 MB |
| chromium | 169 MB |
| CapoRhythia | 100 MB |

- **discordcanary** (zainstalowany, działa): `Cache` 855 MB (75 590 plików, najstarszy z 2026-09-04), `app-1.0.2506` 564 MB (aktywny ładunek aktualizatora; coredump pokazuje, że wywaliła się starsza `app-1.0.2500`), `module_data` 145 MB, `logs` 39 MB, `Dictionaries` 23 MB, `Code Cache` 20 MB, `Shared Dictionary` 13 MB, `Local Storage` 4,5 MB.
- **discord** (pakiet `discord` NIE jest zainstalowany; pozostałość po usuniętej aplikacji): `app-1.0.163` 582 MB, `Cache` 51 MB, `download` 38 MB, `Code Cache` 28 MB, `Local Storage` 2,4 MB, najnowszy plik w Cache z 2026-10-20.
- **zen** (zainstalowany): profil `gm8yiq13.Default (release)` 1,2 GB, z czego `storage/default` 902 MB (dane stron: IndexedDB, Cache API, localStorage), `storage/permanent` 147 MB, `extensions` 29 MB, `security_state` 25 MB, `gmp-widevinecdm` 21 MB. Właściwy cache HTTP jest w `~/.cache/zen`.
- **VSCodium** (zainstalowany, działa): `WebStorage` 303 MB (`1/CacheStorage` 216 MB, `2/CacheStorage` 86 MB: Cache API webview rozszerzeń), `CachedData` 41 MB (cache kodu V8), `Cache` 10 MB, `GPUCache` 8 MB, `User` 3,4 MB (ustawienia!).
- **google-chrome** (zainstalowany): `Default` 61 MB, `optimization_guide_model_store` 43 MB, `WasmTtsEngine` 24 MB, `Safe Browsing` 18 MB, `component_crx_cache` 13 MB. Cache HTTP jest w `~/.cache/google-chrome`.
- **chromium** (pakiet NIE jest zainstalowany): pozostały profil 169 MB.
- **`*-back-ovfs.base`** (2 katalogi, 367 MB): kopie profili chromium i google-chrome, oba z mtime 2026-11-18 08:20. Nazwa to konwencja kopii nakładki profile-sync-daemon; `profile-sync-daemon` nie jest zainstalowany, żadne montowanie nie jest aktywne (`findmnt`), brak `/run/user/1000/psd`. Pozostałe pełne kopie profili (w środku ciasteczka, loginy): **do przejrzenia, dane osobiste**.
- Podkatalogi awarii/logów/cache w ~/.config: brak katalogów `Crashpad` powyżej 1 MB; `CapoRhythia/cache` 92 MB + `logs` 2,6 MB.

### 1.5 Inne

- `~/.local/state` 96 KB: wireplumber, VSCodium, stan tapety ukishima, showtime, pill. Sam stan, nic do czyszczenia.
- Coredumpy (`/var/lib/systemd/coredump`, poza $HOME, czytelne dla użytkownika przez ACL, **nie do usunięcia przez użytkownika**): 1,2 GB, 3 pliki. CS2 1 GB (2026-11-18), DiscordCanary 132 MB, chrome 19 MB. Faza 1 może tylko raportować; tmpfiles usuwa je po 2 tygodniach.
- `~/.claude` 1,7 GB (`jobs` 1,2 GB, `projects` 307 MB): dane działającego agenta. Nigdy nie proponować.
- Artefakty deweloperskie z `CACHEDIR.TAG`:
  - `~/Pobrane/test/xu-integration/target` **11 GB**
  - `~/.local/share/horizon-launcher/native/target` 3,2 GB (build zarządzany przez launcher; do przejrzenia)
  - `~/.cache/bladephoria-web-target` 1021 MB
  - `~/Pobrane/test/XUninstaller/.claude/worktrees/agent-a8ac65cc5ac8d22d8/target` 333 MB (w nieaktualnym worktree agenta)
  - `~/Projects/XNetworkFilter/target` 46 MB, `~/Projects/XDNS/target` 12 MB
  - Nie znaleziono `target/` bez `CACHEDIR.TAG`. Brak `node_modules` w `~/Projects` i `~/Pobrane`. Globalne: `~/.local/lib/node_modules` (instalacje npm -g, np. claude-rpc: **to nie śmieci**), `~/.npm/_npx/*/node_modules` (cache npx).
- `~/.cargo/registry` 335 MB (`src` 273 MB, `cache` 39 MB, `index` 24 MB), ma `CACHEDIR.TAG`. `~/.npm/_cacache` 29 MB.
- Osierocone pliki `.desktop` w `~/.local/share/applications` (27 łącznie), brak celu Exec:
  - `io.github.yhymid.xuninstaller.desktop` → `~/.local/bin/XUninstaller-0.1.0-x86_64.AppImage` (brak)
  - `osu-wine.desktop`, `osuwinello-file-extensions-handler.desktop`, `osuwinello-url-handler.desktop` → `~/.local/bin/osu-wine` (brak)
  - 3× `BeatSaberModManager-url-*.desktop` → `BeatSaberModManager` nie ma w PATH
- Zepsute symlinki w `~/.local/bin`, `~/.local/share/applications`, `~/.config/autostart`: brak. `~/.local/bin/steam` to własny skrypt opakowujący: nigdy nie ruszać.
- `~/.local/share/recently-used.xbel` 16 KB.

---

## 2. Katalog reguł

Pola reguły: **ścieżka** · **rodzaj** · **odtwarza się?** · **bezpieczne, gdy działa?** (procesy do sprawdzenia) · **ryzyko** · **źródło / uzasadnienie**.
Poziomy ryzyka: *bezpieczne* = wolno zaznaczyć z góry; *niskie* = proponować, domyślnie niezaznaczone albo z notą o koszcie; *do przejrzenia* = tylko pokazać, użytkownik decyduje o każdej pozycji, nigdy nie zaznaczać z góry.

### C. Cache XDG

- **C1. Reguła ogólna `~/.cache/<x>`**
  - rodzaj: cache · odtwarza się: tak (zwykle z tą samą treścią) · gdy działa: nie dla aplikacji-właściciela · ryzyko: domyślnie niskie, bezpieczne tylko gdy pasuje konkretna reguła niżej.
  - Źródło: specyfikacja XDG („non-essential”). Dowód: cały `~/.cache` odtworzony 2026-11-17 bez utraty konfiguracji.
  - Nieznane podkatalogi: nigdy nie zaznaczać z góry, pokazywać „nieznany cache”.
- **C2. Wyjątki: rzeczy w ~/.cache, które NIE są bezpieczne**
  - `~/.cache/music-opus` (ta maszyna, 8,5 GB `.opus`): multimedia użytkownika. Heurystyka: katalog w `~/.cache`, którego pliki to głównie multimedia/dokumenty (`.opus .mp3 .flac .mkv .mp4 .pdf .png .jpg .zip`) o czytelnych nazwach i bez `CACHEDIR.TAG` → **do przejrzenia, „wygląda na Twoje pliki”**.
  - `~/.cache/yay/vcs.json`: stan VCS pakietów deweloperskich. Jego utrata sprawia, że yay zapomina commity pakietów `-git`; odtwarza się tylko przez `yay -Y --gendb`. Zostawić; czyścić tylko podkatalogi paczek. Źródło: https://github.com/Jguer/yay (FAQ gendb).
  - KeePassXC (tu niezainstalowany): wersje sprzed końca 2023 trzymały ostatnio otwierane bazy i stan okna w `~/.cache/keepassxc/keepassxc.ini`; PR #9755 przeniósł to do `~/.local/state`. Źródło: https://github.com/keepassxreboot/keepassxc/issues/9920 i PR #9755. Reguła: do przejrzenia.
  - Spotify (niezainstalowany): `~/.cache/spotify/Storage` trzyma pobrane offline utwory (ponowne pobranie wymaga Premium i sieci). Reguła: niskie, z notą o koszcie. Uzasadnienie, nie sprawdzone tutaj.
  - `~/.cache/<x>-target` / `CARGO_TARGET_DIR` (tu `bladephoria-web-target`): wyniki budowania, do odtworzenia, ale wolno; obsługuje to reguła D1.
  - `~/.cache/claude-cli-nodejs`: logi działającego narzędzia. Pomijać, gdy działa `claude`.
  - `~/.cache/gh`: cache HTTP CLI gh; uwierzytelnienie jest w `~/.config/gh/hosts.yml` (nie w cache). Bezpieczne.
- **C3. Miniatury `~/.cache/thumbnails/{normal,large,x-large,xx-large,fail}`**
  - rodzaj: cache · odtwarza się: tak, tak samo · gdy działa: tak · ryzyko: bezpieczne.
  - Źródło: freedesktop Thumbnail Managing Standard, https://specifications.freedesktop.org/thumbnail-spec/latest/ . Na tej maszynie nie ma.
- **C4. fontconfig `~/.cache/fontconfig`**
  - cache · tak, tak samo (odbudowa przy następnym wyszukaniu czcionki albo `fc-cache`) · tak · bezpieczne. Ma `CACHEDIR.TAG`. Tutaj odbudowany 6 s po zalogowaniu.
- **C5. Małe cache GTK/GStreamer/glycin/kitty/hyprland/quickshell/fish/event-sound**
  - cache · tak, tak samo · rejestr gstreamera tak, pozostałe nie dla aplikacji-właściciela (`Hyprland`, `kitty`, `quickshell`, `fish`) · bezpieczne, ale maleńkie. Niewielka wartość: grupować, nie wymieniać każdego.
- **C6. `~/.cache/javasharedresources`** (współdzielony cache klas OpenJ9/Java): cache · tak · nie (`java`) · bezpieczne.
- **C7. `~/.cache/pip`, `~/.cache/uv`, `~/.cache/pypoetry`, `~/.cache/go-build`**
  - cache paczek / cache budowania · tak (ponowne pobranie/budowanie) · nie (`pip`, `uv`, `go`) · bezpieczne.
  - Źródła: https://pip.pypa.io/en/stable/topics/caching/ , https://docs.astral.sh/uv/concepts/cache/ (`uv cache clean`), `go help cache` (`go clean -cache`). Gdy narzędzie ma własne polecenie czyszczenia, preferować je.

### G. Cache shaderów GPU

- **G1. Mesa `~/.cache/mesa_shader_cache`, `mesa_shader_cache_sf`, `mesa_shader_cache_db`**
  - cache · tak, wolna odbudowa (przycinanie przy pierwszym użyciu każdego shadera) · nie: każda aplikacja GPU trzyma go otwartego (sprawdzić: proces ma katalog otwarty przez `/proc/*/fd`; co najmniej `Hyprland`, `steam`, gry) · ryzyko: niskie.
  - Mesa domyślnie ogranicza cache wieloplikowy do 1 GB (`MESA_SHADER_CACHE_MAX_SIZE`); wariant jednoplikowy nie ma limitu. Te 515 MB zarządza się więc samo; proponowanie go daje niewiele. Źródło: https://docs.mesa3d.org/envvars.html
- **G2. `~/.cache/radv_builtin_shaders`**: cache · tak, szybko · nie (aplikacje GPU) · bezpieczne, maleńkie.
- **G3. NVIDIA `~/.cache/nvidia/GLCache`, `~/.nv/ComputeCache`**: cache · tak, wolno · nie · niskie. Nie ma (APU AMD).
- **G4. DXVK `*.dxvk-cache`, vkd3d `vkd3d-proton.cache*`**: cache · tak, wolno · nie (gra) · niskie. Zwykle w katalogu gry albo prefiksie; Steam trzyma je w `shadercache/<id>/DXVK_state_cache` (tu wszystkie po 0 bajtów). Zabłąkany `~/.local/share/Steam/d3ddriverquery64.dxvk-cache` ma 12 bajtów.
- **G5. Steam `steamapps/shadercache/<appid>`**
  - cache · tak, wolno: Steam ponownie pobiera/odtwarza wstępnie przygotowane dane Fossilize, a gry przycinają do odbudowy · gdy działa: nie (`steam`, `steamwebhelper`, `fossilize_replay`, `reaper`) · ryzyko: zainstalowana gra = **niskie z ostrzeżeniem o przycinaniu**, osierocony appid (bez manifestu i skrótu) = bezpieczne.
  - Źródła: zgłoszenia na forach, że wyczyszczenie cache shaderów przywraca przycinanie do czasu odbudowy (https://steamcommunity.com/app/1675200/discussions/0/4038102303460802607 , https://github.com/ValveSoftware/steam-for-linux/issues/11392). Valve nie dokumentuje tego oficjalnie.
  - Normalizować appid ze znakiem i bez znaku (fala 2 §03).

### S. Klient Steam (root `~/.local/share/Steam`, osiągany przez `~/.steam/root`)

Wszystkie reguły S: **tylko gdy Steam nie działa** (`pgrep -x steam`, `steamwebhelper`). Podczas tego pomiaru Steam działał.

- **S1. `logs/*`** (tu 411 MB): log · odtwarza się (nowe logi) · nie · bezpieczne. Jeśli Steam mimo wszystko działa, zostawić bieżący zestaw plików.
- **S2. `appcache/httpcache`**: cache · tak, tak samo · nie · bezpieczne. `appcache/librarycache` (grafiki): cache · tak, pobierane ponownie · nie · bezpieczne. **Nigdy** `appcache/appinfo.vdf`, `packageinfo.vdf`, `stats` (Steam je odbudowuje, ale wolno i z długim „aktualizowaniem” przy starcie; mała wartość).
- **S3. `config/htmlcache`** (118 MB, cache przeglądarki CEF): cache · tak · nie · niskie. Zawiera ciasteczka sesji stron sklepu; usunięcie może wylogować z widoków stron, ale nie z samego Steama (uzasadnienie, nie sprawdzone).
- **S4. `depotcache/*.manifest`**: cache · tak, pobierane ponownie przy aktualizacji · nie · niskie.
- **S5. `steamapps/downloading/*`, `steamapps/temp/*`, `steamapps/corrupt/*`**: pozostałość · nie (częściowe pobrania zaczynają się od nowa) · nie · bezpieczne, jeśli aplikacja nie ma aktywnego pobierania (bit StateFlags 1<<20 / 1024 wyłączony). Tu puste.
- **S6. `package/*`** (504 MB paczek aktualizacji klienta), `bootstrap.tar.xz`: cache paczek · Steam pobierze je ponownie przy następnym starcie · nie · do przejrzenia. Uszkodzenie może wymusić pełne ponowne pobranie klienta; przy <1 GB nie warto ryzykować.
- **S7. Zrzuty: `/tmp/dumps` (poza HOME), `Steam/dumps`, `*.dmp`**: zrzut · nie · tak · bezpieczne.
- **S8. `config/*.async*.tmp`**: pozostały plik tymczasowy zapisu atomowego · nie · nie · bezpieczne, gdy Steam jest zamknięty.
- **S9. Osierocone `steamapps/compatdata/<appid>`** (bez manifestu i skrótu): pozostałość · **nie** (prefiks tworzy się od nowa pusty; zapisy przepadają) · nie (`steam`, `wineserver`, `reaper`) · **do przejrzenia**, z dopiskiem „w `pfx/drive_c/users/steamuser` mogą być zapisy gier”. Nigdy dla `0`, `*.backup-*` ani appid narzędzi Protona. Reguły w fali 2 §03.

### W. Proton / Wine / launchery

- **W1. Pliki tymczasowe prefiksu Wine `pfx/drive_c/users/steamuser/AppData/Local/Temp/*`, `drive_c/windows/temp/*`** (w dowolnym prefiksie): cache/tymczasowe · nie trzeba odtwarzać · nie (`wineserver`) · bezpieczne.
- **W2. Całe prefiksy Wine** (`~/.local/share/wineprefixes/*`, `compat` launcherów): zawsze **do przejrzenia**. `l4d-physx-install*` (1,2 GB) i `l4d-physx-test` wyglądają na pozostałości testów, ale tylko po nazwie.
- **W3. Runtime’y launcherów** (`umu/steamrt*`, `yawl/SteamLinuxRuntime_sniper`, `lutris/runtime`): cache paczek · tak, launcher pobierze je ponownie · nie (`umu-run`, `yawl`, `lutris`) · niskie. Wyjątek: `yawl/SteamLinuxRuntime_sniper.tar.xz` (186 MB) obok rozpakowanego katalogu to archiwum pobrania; niskie.
- **W4. Cache pobrań launcherów**: `PrismLauncher/cache` (2,8 GB: pobrane mody, modpacki, Java), `bedrock-on-linux/cache` (843 MB), `osu/cache` (52 MB), `com.epicgames.eglv2/WebKitCache` + `CacheStorage` (240 MB), `CapoRhythia/cache` (92 MB): cache · tak, pobierane ponownie · nie (`prismlauncher`, `java`, `osu!`, nazwa launchera) · niskie. Źródła: Prism Launcher trzyma cache HTTP w `cache/` z indeksem `metacache` (uzasadnienie z układu katalogów); przed oznaczeniem jako bezpieczne sprawdzić każdy launcher.
- **W5. Dołączone drzewa budowania launcherów**: `horizon-launcher/native/target` (3,2 GB, `CACHEDIR.TAG`), `horizon-launcher/cargo/{registry,git}`: build · tak, wolno · nie · do przejrzenia (launcher może oczekiwać, że istnieją; nie wiadomo).

### E. Aplikacje Electron / Chromium (`~/.config/<app>`)

Sprawdzenie, czy działa: `SingletonLock` → `<host>-<pid>` z żyjącym pid albo `pgrep -x` aplikacji (tu działały `DiscordCanary` i `codium`). Reguły z fali 2 §06.

- **Bezpieczne podkatalogi (cache, odtwarza się tak samo):** `Cache`, `Code Cache`, `GPUCache`, `DawnCache`, `DawnGraphiteCache`, `DawnWebGPUCache`, `GrShaderCache`, `ShaderCache`, `GPUPersistentCache`, `Service Worker/CacheStorage`, `Service Worker/ScriptCache`, `component_crx_cache`, `CachedData` i `CachedExtensionVSIXs` z rodziny VSCode, `logs`, `Crashpad/completed`, `Crashpad/pending`, `Crash Reports`. Źródło: projekt disk-cache/code-cache Chromium, dokumentacja Electron `app.getPath('sessionData')` https://www.electronjs.org/docs/latest/api/app .
- **Niskie (odtwarza się, ale kosztem ponownego pobrania):** `Dictionaries` (hunspell), `Shared Dictionary`, `optimization_guide_model_store`, `WasmTtsEngine`, `OnDeviceHeadSuggestModel`, VSCodium `WebStorage/*/CacheStorage` (Cache API webview rozszerzeń; webview pobierają ponownie).
- **NIGDY:** `Local Storage`, `IndexedDB`, `Session Storage`, `Cookies*`, `Login Data*`, `Web Data`, `Preferences`, `Local State`, `Network Persistent State`, `TransportSecurity`, `Trust Tokens`, `blob_storage`, `databases`, `WebStorage/QuotaManager`, `User/` (ustawienia, skróty i snippety VSCode), `Crashpad/settings.dat`, `Discord Key Storage`, Discordowe `app-*`/`module_data`/`installer.db` zainstalowanego Discorda.
- **Ładunek aktualizatora Discorda:** stare katalogi `app-1.0.NNNN` poza najnowszym: pozostałość · nie · nie (`DiscordCanary`) · niskie. Tu istnieje tylko jeden.
- **Pozostały cały profil odinstalowanej aplikacji** (`~/.config/discord` 709 MB, `~/.config/chromium` 169 MB, `*-back-ovfs.base` 367 MB): pozostałość · nie · nd. · **do przejrzenia**: są tam loginy i historia. Proponować dopiero po potwierdzeniu, że aplikacji nie ma (pacman mówi, że nie jest zainstalowana).

### B. Przeglądarki

- **Firefox/Zen/LibreWolf:** cache HTTP jest w `~/.cache/<przeglądarka>/<profil>/cache2`, `startupCache`, `safebrowsing`, `jumpListCache`, `thumbnails`: cache · tak, tak samo · nie (`firefox`, `zen`, `zen-bin`) · bezpieczne. W `~/.config/zen/<profil>` (albo `~/.mozilla/firefox/<profil>`) **tylko** `storage/default/*/cache` (Cache API) przypomina cache, i to też niskie; wszystko inne (`places.sqlite`, `logins.json`, `key4.db`, `cookies.sqlite`, `storage/default/*/idb`, `extensions`, `sessionstore*`) to NIGDY. Źródło: https://support.mozilla.org/kb/profiles-where-firefox-stores-user-data .
- **Chrome/Chromium/Brave:** `~/.cache/google-chrome/<profil>/{Cache,Code Cache}` (tu 153 MB): cache · tak · nie (`chrome`) · bezpieczne. W `~/.config/google-chrome` stosować listy Electron z góry dla każdego profilu (`Default`, `Profile N`).

### P. Menedżery pakietów i narzędzia deweloperskie

- **P1. yay/paru/pikaur `~/.cache/{yay,paru/clone,pikaur}/<pkg>`**: cache paczek/build · tak (ponowne klonowanie, budowanie) · nie (`yay`, `paru`, `pikaur`, `makepkg`) · bezpieczne dla katalogów, których pakiet nie jest zainstalowany; niskie dla zainstalowanych (stare `*.pkg.tar.zst` to jedyna lokalna kopia do downgrade). Zostawić `yay/vcs.json` i `completion.cache`, chyba że użytkownik poprosi.
- **P2. Cargo `~/.cargo/registry/{cache,src}`, `~/.cargo/git/checkouts`**: cache paczek · tak, ponowne pobranie · nie (`cargo`, `rustc`, `rust-analyzer`) · bezpieczne; `registry/index` niskie (wolne pierwsze rozwiązywanie zależności). Nigdy `~/.cargo/bin`, `~/.cargo/config.toml`, `credentials.toml`. Źródło: https://doc.rust-lang.org/cargo/guide/cargo-home.html (cache „can be cleared”).
- **P3. rustup `~/.rustup/{downloads,tmp}`**: cache paczek · tak · nie · bezpieczne. `toolchains` = do przejrzenia. Tu nie ma.
- **P4. npm `~/.npm/_cacache`, `~/.npm/_npx`, `~/.npm/_logs`**: cache paczek/log · tak · nie (`npm`, `node`, `npx`) · bezpieczne. Źródło: https://docs.npmjs.com/cli/commands/npm-cache . Nigdy `~/.local/lib/node_modules` (instalacje globalne).
- **P5. Magazyn pnpm `~/.local/share/pnpm/store`** (2,6 GB): cache paczek, **twardo linkowany** do projektów · tak · nie (`pnpm`, `node`) · niskie. Stosować semantykę `pnpm store prune` (usuwać tylko paczki bez odwołań); usunięcie całego magazynu niczego na dysku nie psuje, ale zwalnia tylko pliki z `nlink==1`. Źródło: https://pnpm.io/cli/store .
- **P6. pip `~/.cache/pip`, uv `~/.cache/uv`, pipx `~/.local/share/pipx/{py,.cache}`**: cache paczek · tak · nie · bezpieczne. Nigdy `pipx/venvs`, `uv/tools`, `uv/python` (zainstalowane narzędzia; do przejrzenia).
- **P7. Go `~/.cache/go-build`** bezpieczne; `~/go/pkg/mod` niskie (pliki tylko do odczytu, wymaga `go clean -modcache`). Nie ma.
- **P8. Gradle `~/.gradle/caches`, `~/.gradle/wrapper/dists`** niskie; Maven `~/.m2/repository` niskie (buildy offline przestają działać). Nie ma.
- **P9. Flatpak użytkownika**: `~/.local/share/flatpak/repo/tmp` (7,7 MB) pozostałość · bezpieczne, gdy nie działa żaden proces `flatpak`; nieużywane runtime’y → zalecić `flatpak uninstall --user --unused` (nie usuwać `repo/objects` ręcznie: twarde linki, uszkadza repo). Źródło: https://docs.flatpak.org/en/latest/flatpak-command-reference.html . `~/.var/app/<id>/cache`: cache · tak · nie (`bwrap` z tym id aplikacji, `flatpak ps`) · bezpieczne. Tu żadnych nie ma.
- **P10. Katalogi budowania `target/` z poprawnym `CACHEDIR.TAG`, `node_modules/` obok `package.json`, `__pycache__`, `.pytest_cache`, `.mypy_cache`, `.ruff_cache`**: build · tak, wolno (przebudowa/reinstalacja) · nie (`cargo`, `rustc`, `node`, `pnpm`, `npm`, `rust-analyzer`) · bezpieczne dla katalogów z `CACHEDIR.TAG` niezmienianych od N dni, inaczej niskie. Nigdy nie wchodzić do `.git`. Pokazywać nazwę projektu i czas ostatniego budowania.

### L. Logi, historia, kosz, resztki integracji

- **L1. `~/.local/state/*/log*`, `~/.local/share/*/logs`, `~/.xsession-errors*`**: log · odtwarza się · tak dla zrotowanych plików (`*.old`, `*.1`, `*.previous*`), nie dla bieżącego pliku · bezpieczne dla zrotowanych, niskie dla bieżącego. Tu: tylko `goverlay/logs`, `osu/logs`, `yawl/yawl.log`, logi Steama.
- **L2. `~/.local/share/recently-used.xbel`**: historia (prywatność) · odtwarza się pusta · tak (GTK go przepisuje) · niskie, z etykietą „prywatność: czyści listę ostatnich plików”, a nie „śmieci”.
- **L3. Kosz `~/.local/share/Trash/{files,info,expunged}`**: kosz · nie · tak · niskie: wypisać nazwy i daty pozycji; użytkownik mógł coś wyrzucić przez pomyłkę. Usuwać `files/X` i `info/X.trashinfo` razem. Źródło: https://specifications.freedesktop.org/trash-spec/latest/ .
- **L4. Zepsute symlinki** w `~/.local/bin`, `~/.local/share/applications`, `~/.config/autostart`, `~/.local/share/icons`: pozostałość · nie · tak · bezpieczne (usuwać sam link, nigdy za nim nie podążać). Tu brak.
- **L5. Osierocone pliki `.desktop`** (brak binarki z Exec albo TryExec): pozostałość · nie · tak · niskie (dysk wymienny albo folder z AppImage może być akurat odmontowany). Tu 7 plików (AppImage XUninstallera, osu-wine ×3, BeatSaberModManager ×3). Sprawdzać też linie `mimeapps.list`, które na nie wskazują (edytować linie, nigdy nie usuwać pliku).
- **L6. Coredumpy** (`/var/lib/systemd/coredump`): zrzut · nie · nd. · w Fazie 1 tylko raport (należą do roota).

---

## 3. Lista NIEBEZPIECZNYCH (nigdy nie proponować, wyprowadzona z tej maszyny)

Twarde blokady ścieżek i wzorców (nie pokazywane jako kandydaci i blokowane jako rodzice zaznaczenia):
- Całe `~/.config`, `~/.local`, `~/.local/share`, `~/.cache`, `~/.steam`, `~/.local/share/Steam` (właściciel raz już stracił wszystko, usuwając `~/.config`). XCleaner usuwa tylko reguły-liście, nigdy katalog XDG najwyższego poziomu.
- Sekrety: `~/.ssh`, `~/.gnupg`, `~/.pki` (baza NSS), `~/.local/share/keyrings`, `~/.config/Discord Key Storage`, `~/.config/gh`, `~/.npmrc`, `~/.cargo/credentials*`, `~/.claude.json`, `~/.codex`, każde `*.key`, `*token*`, `*wallet*`, katalogi menedżerów haseł (keepassxc, Bitwarden, 1Password, jeśli są).
- Konfiguracja powłoki i pulpitu: `~/.bashrc`, `~/.bash_history`, `~/.config/hypr`, `~/.config/fish`, `~/.config/kitty`, `~/.config/quickshell`, `~/.config/dconf`, `~/.config/systemd`, `~/.config/autostart`, `~/.config/mimeapps.list`, `~/.local/state/*`, `~/.local/bin/*` (łącznie z własnym skryptem `steam`), `~/.local/lib/node_modules`.
- Steam: `config/` (config.vdf, loginusers.vdf), `userdata/` (zapisy w chmurze, localconfig, shortcuts.vdf), `steamapps/*.acf`, `libraryfolders.vdf`, `steamapps/common` (gry), `compatibilitytools.d` (w użyciu), `steamrt64`, `ubuntu12_*`, `appcache/*.vdf`, `compatdata/<zainstalowany id>` (np. zapisy 292030 Witcher 3 są w prefiksie), `compatdata/0`, `compatdata/*.backup-*`, `workshop/content/241100`.
- Dane i zapisy gier: `PrismLauncher/{instances,backups,accounts.json}`, `horizon/{saves,instances}`, `com.megalithofficial.basalt-launcher/{instances,basalt.db}`, `bedrock-on-linux/games`, `BSManager/BSInstances`, `osu/files` + `client.realm` (biblioteka lazer), `wineprefixes/osu-*` (piosenki/skórki osu!stable), `osuconfig`, `CapoRhythia/*.json` + `rhythia.db`, `lutris/pga.db`, `heroic`, prefiksy `compat` launcherów.
- Profile przeglądarek i aplikacji poza podkatalogami cache: `~/.config/zen/*` (poza regułami z B), `~/.config/google-chrome/*`, `~/.config/VSCodium/User`, `~/.vscode-oss` (rozszerzenia), `~/.config/Vencord`, części `~/.config/discordcanary`, które nie są cache.
- Dane paczek i runtime’ów: `~/.local/share/flatpak` (poza `repo/tmp`), `pipx/venvs`, `uv/tools`, `uv/python`, `~/.cargo/bin`.
- Agenci i narzędzia: `~/.claude`, `~/.claude-rpc`, `~/.codex`, `~/.semgrep`.
- Pliki użytkownika: `~/Pictures`, `~/Videos`, `~/Games`, `~/Applications` (AppImage), `~/backups`, `~/benchmarks`, `~/Pobrane` poza regułami artefaktów budowania, oraz `~/.cache/music-opus`.

Reguły ochronne:
- Nigdy nie podążać za symlinkami; odmawiać przy rodzicach zapisywalnych dla wszystkich (tu HOME ma 0777, fala 2).
- Odmawiać, gdy działa proces-właściciel.
- Odmawiać ścieżki, której postać kanoniczna wychodzi poza `$HOME`.

---

## 4. „Odtwarza się przy tej samej konfiguracji”: konkretne wyniki tutaj

| Katalog | Rozmiar | Dowód |
|---|---|---|
| ~/.cache/fontconfig | 692 KB | TAG, odtworzony 11-17 |
| ~/.cache/mesa_shader_cache | 515 MB | odtworzony 11-17 |
| ~/.cache/radv_builtin_shaders | 2,5 MB | odtworzony 11-17 |
| ~/.cache/gtk-4.0 | 280 KB | odtworzony 11-17 |
| ~/.cache/gstreamer-1.0 | 1,6 MB | rejestr, 1 plik |
| ~/.cache/google-chrome | 153 MB | cache Chromium |
| ~/.cache/zen | 34 MB | odtworzony 11-17 |
| ~/.cache/yay/* paczki | 723 MB | AUR do przebudowania |
| ~/.cargo/registry | 335 MB | CACHEDIR.TAG |
| ~/.npm/_cacache | 29 MB | dokumentacja npm |
| pipx/py | mały | CACHEDIR.TAG |
| discordcanary/Cache | 855 MB | cache Chromium |
| discordcanary/Code Cache | 20 MB | cache V8 |
| VSCodium/CachedData | 41 MB | cache V8 |
| Logi Steama | 411 MB | logi |
| Steam appcache/httpcache | 27 MB | cache HTTP |
| Steam config/htmlcache | 118 MB | cache CEF |
| xu-integration/target | 11 GB | CACHEDIR.TAG |
| bladephoria-web-target | 1 GB | CACHEDIR.TAG |
| PrismLauncher/cache | 2,8 GB | cache pobrań |

Uwagi:
- „Odtworzony 11-17” = czas utworzenia w ciągu sekund lub minut od samego `~/.cache` (2026-11-17 15:50:26), z zawartością plików dopiero od 11-17. To pokazuje, że aplikacja odbudowała go bez działania użytkownika.
- `discordcanary/Cache`: najstarszy plik z 2026-09-04 i 75 tys. plików. Simple cache Chromium ma limit rozmiaru, ale tu jest duży. Odtwarza się przy użyciu (ponowne pobieranie awatarów, emoji, multimediów).
- Steam `shadercache/*` (19 GB) i `mesa_shader_cache` też się odtwarzają, ale **nie niezauważalnie**: przycinanie w CS2/Deadlock do czasu odbudowy. Raportować jako „wolna odbudowa”.

---

## 5. 15 największych pozycji do odzyskania na tej maszynie

| # | Pozycja | Rozmiar | Ryzyko |
|---|---|---|---|
| 1 | shadercache/730 CS2 | 11 GB | niskie, przycinanie |
| 2 | xu-integration/target | 11 GB | bezpieczne |
| 3 | ~/.cache/music-opus | 8,5 GB | do przejrzenia, dane użytkownika |
| 4 | shadercache/1422450 Deadlock | 6,6 GB | niskie, przycinanie |
| 5 | Osierocone compatdata (9) | 3,3 GB | do przejrzenia, zapisy |
| 6 | horizon native/target | 3,2 GB | do przejrzenia |
| 7 | PrismLauncher/cache | 2,8 GB | niskie |
| 8 | ~/.cache/bladephoria-web-target | 1 GB | bezpieczne |
| 9 | discordcanary/Cache | 855 MB | bezpieczne (zamknięty) |
| 10 | bedrock-on-linux/cache | 843 MB | niskie |
| 11 | Pozostałe profile (discord, chromium, ovfs) | 1,2 GB | do przejrzenia |
| 12 | ~/.cache/yay paczki | 723 MB | bezpieczne |
| 13 | shadercache/3240220 GTA V | 722 MB | niskie, przycinanie |
| 14 | Logi Steama | 411 MB | bezpieczne (zamknięty) |
| 15 | VSCodium WebStorage CacheStorage | 302 MB | niskie |

Wyróżnienia: `~/.cache/mesa_shader_cache` 515 MB (niskie), `~/.cargo/registry` 312 MB (`src`+`cache`, bezpieczne), target worktree XUninstallera 333 MB (bezpieczne), prefiksy `l4d-physx-*` 1,2 GB (do przejrzenia), Steam `config/htmlcache` 118 MB, `.tar.xz` yawl 186 MB, kosz 147 MB (niskie), `~/.cache/google-chrome` 153 MB (bezpieczne).

- **Sumy:**
  - Bezpieczne albo niskie bez utraty danych (wiersze 1, 2, 4, 7, 8, 9, 10, 12, 13, 14, 15 plus wyróżnienia): **≈ 38 GB**. Z tego ~19 GB to cache shaderów, których kosztem jest przycinanie.
  - Bez cache shaderów: **≈ 19 GB**.
  - Wiersze tylko do przejrzenia (3, 5, 6, 11): ≈ 16 GB więcej, decyduje użytkownik.
- **Nie do odzyskania jako śmieci:** 306 GB zainstalowanych gier. Prawdziwą dźwignią miejsca jest odinstalowywanie gier (XUninstaller), a nie czyszczenie.

---

## Niezweryfikowane założenia / ograniczenia

- Zachowanie Steama po usunięciu `htmlcache`, `depotcache` i `package` pochodzi z raportów społeczności i rozumowania, nie z testów.
- Pochodzenie `-back-ovfs.base` (tryb nakładki profile-sync-daemon) wywnioskowano z nazwy; psd nie jest teraz zainstalowany.
- Semantykę cache launcherów Prism, bedrock i horizon wywnioskowano z układu katalogów, a nie z ich kodu źródłowego.
- Rozmiary to `du` na btrfs (zajętość na dysku); kompresja i reflinki mogą sprawić, że realnie zwolni się mniej.
- Coredumpy były tylko wylistowane (`coredumpctl`), bez `stat` każdego pliku.
