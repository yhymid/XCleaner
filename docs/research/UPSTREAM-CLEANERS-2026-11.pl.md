# Analiza istniejących programów do czyszczenia (2026-11)

[English](UPSTREAM-CLEANERS-2026-11.md) | **Polski**

Research dla XCleanera (Rust + GTK4/libadwaita, MIT). Faza 1 = tylko `$HOME`, faza 2 = `/` przez brokera roota z sudo.
Wszystkie fakty pochodzą z płytkich klonów repozytoriów upstream (HEAD z 2026-11-22), API crates.io i oficjalnej dokumentacji.
Stwierdzenia oznaczone **(zaobserwowane)** zmierzono na maszynie deweloperskiej (CachyOS, btrfs, 8 wątków, `$HOME` = 441 GiB, 1,14 mln wpisów).
Stwierdzenia oznaczone **(niezweryfikowane)** to założenia.

Główne źródła:

- BleachBit: https://github.com/bleachbit/bleachbit (GPL-3.0-or-later, ostatni push 2026-11-19)
- czkawka/krokiet: https://github.com/qarmin/czkawka (v12.0.2, 2026-09-09)
- kondo: https://github.com/tbillington/kondo (MIT)
- cargo-sweep: https://github.com/holmgr/cargo-sweep (MIT, README mówi, że nieutrzymywany)
- npkill: https://github.com/voidcosmos/npkill (MIT, v1.0.0)
- Stacer: https://github.com/oguzhaninan/Stacer (GPL-3.0, ostatni push 2024-02-10, ostatnie wydanie v1.1.0 z 2019)
- KDE Sweeper: https://github.com/KDE/sweeper (mirror invent.kde.org, LGPL-2.1-or-later według `LICENSES/`)
- dust: https://github.com/bootandy/dust (Apache-2.0), gdu: https://github.com/dundee/gdu (MIT)
- podręcznik ncdu: https://dev.yorhel.nl/ncdu/man (2.9.x)
- dua-cli / dua-core: https://github.com/Byron/dua-cli
- jwalk: https://github.com/Byron/jwalk (zarchiwizowany)
- walkdir: https://github.com/BurntSushi/walkdir, ignore: https://github.com/BurntSushi/ripgrep/tree/master/crates/ignore
- Rust std (`DirEntry::metadata`): https://github.com/rust-lang/rust/blob/main/library/std/src/sys/fs/unix.rs
- CCleaner: https://www.ccleaner.com/legal/products-policy , https://support.ccleaner.com/articles/en_US/Master_Article/what-is-health-check , https://www.ccleaner.com/ccleaner/version-history
- Podwoluminy btrfs: https://btrfs.readthedocs.io/en/latest/Subvolumes.html

---

## 1. BleachBit

### 1.1 Format CleanerML

Schemat: `doc/cleaner_markup_language.xsd`
(https://github.com/bleachbit/bleachbit/blob/master/doc/cleaner_markup_language.xsd). Struktura:

- `<cleaner id=.. os=..>`
  - `<label translate=..>` (nazwa aplikacji), opcjonalnie `<description>`
  - `<running type="exe|pathname" same_user="true|false" os=..>` (0..n): nazwa procesu albo glob pliku blokady
  - `<var name=..><value search="glob|winreg" os=..>` (0..n): rozwija `$$name$$` w ścieżkach (np. `$$profile$$` = każdy profil Firefoksa)
  - `<option id=.. os=..>` (0..n)
    - `<label>`, `<description>`
    - `<warning>` (0..1) — **element, nie atrybut**; pokazywany, gdy użytkownik zaznaczy opcję
    - `<action command=.. search=.. path=.. ...>` (1..n)
  - `<localizations>` z zagnieżdżonymi `<path location= filter= directoryregex=>` i `<regexfilter prefix= postfix=>`
- Gramatyka `os`: lista po przecinkach z opcjonalną negacją `!`, wartości `bsd|darwin|linux|macos|netbsd|openbsd|freebsd|unix|windows`.
- Wartości logiczne muszą być dosłownie `true`/`false` (wielkość liter bez znaczenia), bez `1/0`.

Wyliczenie `action@command` (pełna lista z XSD):

| Grupa | Polecenia |
|---|---|
| Operacje na plikach | `delete`, `truncate`, `shred` |
| Edycja struktur | `json`, `ini`, `sqlite.vacuum` |
| Bazy przeglądarek | `chrome.*`, `mozilla.*`, `cookie` |
| Menedżery pakietów | `apt.*`, `dnf.*`, `yum.clean_all`, `pacman.cache`, `snap.disabled` |
| System | `journald.clean`, `process`, `process.terminate` |
| Windows | `winreg`, `win.shell.change.notify`, `office_registrymodifications` |
| macOS | `macos.orphaned_framework_versions` |

Wartości `action@search`:

| Wartość | Znaczenie |
|---|---|
| `file` | jedna ścieżka |
| `glob` | rozwinięcie globa |
| `walk.files` | rekurencyjnie, tylko pliki |
| `walk.all` | rekurencyjnie, pliki + katalogi |
| `walk.top` | rekurencyjnie + katalog główny |
| `deep` | korzenie wybrane przez użytkownika, regex |

Inne atrybuty akcji: `path`, `regex`/`nregex` (nazwa bazowa), `wholeregex`/`nwholeregex` (pełna ścieżka), `type="f|d"`,
`cmd`, `wait`, `address`, `name`, `parameter`, `section` (dla ini/json), `same_user`, `skip_open` (tylko deep).

### 1.2 Moduły czyszczące istotne na Linuksie w `cleaners/*.xml`

Wyciągnięte przez sparsowanie każdego XML i odrzucenie części `os="windows"`/`macos`. `!` = opcja ma `<warning>`. Polecenia w
nawiasach, gdy nie jest to zwykłe `delete`. `run=` to blokada działającego procesu.

| Moduł | Opcje (Linux) | Blokada |
|---|---|---|
| Firefox / LibreWolf / Waterfox / Zen / Pale Moon | backup, cache, cookies, crash_reports, forms, passwords!, session, site_data, site_preferences, url_history, vacuum | exe + `lock` profilu |
| Chromium / Chrome / Brave / Edge / Vivaldi / Opera / Helium | cache, cookies, crash_reports, form_history, history, search_engines, session, site_data, statistics, sync, passwords!, vacuum, site_preferences (Chrome także `ai`) | exe + `SingletonLock` |
| Epiphany | cache, cookies, passwords!, places!, site_data | exe + lock |
| Thunderbird | cache, cookies, crash_reports, index!, sessionjson, passwords!, vacuum | exe |
| Evolution / Geary | cache; Geary także attachments, sync_conflict, vacuum! | exe |
| Discord / Slack | cache, cookies, history, vacuum | exe |
| VS Code + forki | backup, cache, crash_reports, logs | code, codium, cursor, windsurf... |
| Claude (CLI) | backup, cache, file_history, logs, memory!, session!, shell_snapshots, tmp | claude |
| Android Studio | gradle_cache!, ide_cache, logs | studio |
| Docker Desktop | cache, logs, session | exe |
| Zoom | cache, logs, recordings | zoom |
| Thumbnails | cache (`~/.thumbnails/`, `$XDG_CACHE_HOME/thumbnails/`) | brak |
| KDE | cache, tmp, recent_documents (`~/.kde*/cache-*`) | brak |
| GNOME | run, search_history | brak |
| Nautilus / Rhythmbox | history! | exe |
| Powłoki / REPL | bash, zsh, fish, python, sqlite3, octave, vim, mc: history | brak |
| Java | cache (`~/.java/deployment/cache/`, icedtea) | brak |
| Wine / winetricks | tmp / temporary_files | brak |
| X11 | debug_logs | brak |
| GIMP | tmp, cache | gimp-2.x |
| LibreOffice / OpenOffice | history / cache, recent_documents | soffice.bin |
| VLC / Audacious / xine | mru / cache, log | vlc |
| Transmission | blocklists!, torrents, history | daemon/gtk/qt |
| pacman | cache (`paccache -rk0`) | pacman, paccache, `db.lck` |
| journald | clean (`journalctl --vacuum-size=1`) | journalctl |
| apt / dnf / yum / snap | clean, autoclean, autoremove!, disabled | apt/dnf |
| deepscan | angular!, backup!, ds_store!, node_modules!, venv!, pycache!, thumbs_db!, tmp!, vim_swap_user! | brak |
| Stare aplikacje | aMSN, aMule, Beagle, d4x, emesene, gFTP, Miro, Pidgin, Skype, RealPlayer, Second Life, gl-117, Nexuiz, Tremulous, Warzone2100... | przeważnie brak |

Wbudowany moduł `System` (Python, `bleachbit/Cleaner.py`, klasa `System`), opcje na Linuksie:

- `cache`: wszystko w `~/.cache/` (`children_in_directory(..., same_device=True)`, ze sprawdzeniem listy wyjątków)
- `tmp`: `/tmp`, `/var/tmp` — tylko zwykłe pliki należące do bieżącego uid, spoza listy wyjątków, nieotwarte przez żaden proces
- `trash`: `~/.Trash`, `$XDG_DATA_HOME/Trash`, kosz snap, `.Trash/$uid` na każdym montowaniu (tylko gdy sticky i nie link) i `.Trash-$uid`
- `recent_documents`: `~/.local/share/recently-used.xbel*` (nadpisywane shredem) + czyszczenie GTK RecentManager
- `rotated_logs`: pliki w `/var/log` pasujące do `\.(\d+|bz2|gz|lz4|xz|zst|old)$` albo `-YYYYMMDD`, zakotwiczone tak, żeby aktywne logi przetrwały
- `desktop_entry`: zepsute pliki `.desktop` w katalogach menu; `localizations`: niepotrzebne pliki lokalizacji
- `custom`: pliki i katalogi wskazane przez użytkownika
- `memory` (czyszczenie swapu i wolnego RAM, oznaczone jako eksperymentalne), `empty_space` (nadpisywanie wolnego miejsca), `dns_cache` (opróżnianie)

### 1.3 Jak BleachBit unika usunięcia niewłaściwych rzeczy

- **Twarde minimum** `_is_system_critical_posix()` (`FileUtilities.py`): `/`, `/proc`, `/sys`, `/run` (poza `/run/media`) nigdy
  nie są usuwane, nawet gdy lista wyjątków jest pusta albo moduł ma błąd.
- **Lista wyjątków („whitelist”)** `whitelisted_posix()`: sprawdza zarówno nazwę symlinka, jak i jego `realpath`; wpisy
  plików i katalogów; dopasowanie prefiksu dla katalogów.
- **Chronione ścieżki** (`share/protected_path.xml`, schemat `doc/protected_path.xsd`): nie są automatycznie wyjątkami, ale
  program ostrzega, gdy użytkownik doda je do własnej listy usuwania. Mają `depth` (`0`, `1`, `2`, `any`) i `case`. Wpisy na
  Linuksie to m.in. `~`, `~/Documents`, `~/.config` (głębokość 1), `~/.local/share` (głębokość 0), `~/.mozilla`, `~/.ssh`,
  `~/.gnupg`, `~/.aws`, `~/.kube`, `~/.password-store`, `~/.docker`, `~/.nvm`, `~/.vscode`, `~/Pictures`, `~/Music`, `~/Videos`,
  `~/projects`, `~/backup(s)`, `~/Nextcloud`, `~/VMs` oraz każde `.git/.hg/.svn/.bzr`. Nagłówek pliku wprost mówi, że `~/.cache`,
  `.cargo`, `.npm` i `.venv` **nie** należą do tej listy (da się je odtworzyć).
- **Blokada działającego procesu** (`Process.py` `is_process_running(exename, require_same_user)`; `Cleaner.is_process_running()`):
  - wyliczanie procesów przez psutil, zapasowo `/proc/*/exe`, zapasowo `ps aux`
  - dopasowuje `comm`, `basename(exe)` **i** `basename(argv[0])` (binarki z wersją w nazwie)
  - blokady `pathname` szukają globem pliku blokady; wiszące symlinki blokad (`IP:+PID` w Gecko, `host-PID` w Chromium) są
    rozwiązywane, a PID sprawdzany (`_lock_link_is_live`), tylko dla tego samego użytkownika (PID-y Flatpaka mogą wskazywać
    wątki kernela hosta)
- **Sprawdzanie otwartych plików** (`FileUtilities.open_files_linux()`, przegląda `/proc/*/fd`): używane dla `/tmp` i
  głębokiego skanu (`skip_open`); jeśli listy otwartych plików nie da się odczytać, plik jest **zostawiany** (bezpieczny domyślny).
- **Prymityw usuwania** `FileUtilities.delete()` / `_delete()`:
  - typ sprawdzany przez `lstat`; symlinki i FIFO są usuwane `unlink`, nigdy nie podąża się za nimi
  - katalogi usuwane wyłącznie przez `rmdir` (zawartość wcześniej od dołu); `ENOTEMPTY` i `EBUSY` (punkt montowania) są logowane i pomijane
  - pliki specjalne (gniazda, urządzenia) są odrzucane
  - operacje względem `dir_fd` i `children_below(..., same_device=True, top=...)`, żeby linki podrzucone w katalogu tmp nie mogły przekierować usuwania
- **Głęboki skan** (`DeepScan.py`): `os.walk` po korzeniach wybranych przez użytkownika, regex po nazwie bazowej i pełnej ścieżce
  z regexem negatywnym (np. `node_modules` wykluczone pod `~/.<ukryty>/` i `AppData`); każda opcja deepscan ma ostrzeżenie.
- **Tryb podglądu**: każde polecenie da się wylistować bez usuwania (przycisk „Preview” w GUI, w CLI `--preview`).

### 1.4 Funkcje, których XCleaner NIE będzie miał

- Czyszczenie pamięci i swapu (`Memory.py` `wipe_memory`, `wipe_swap_linux`) — eksperymentalne, wymaga roota, ryzykowne.
- Nadpisywanie wolnego miejsca (`empty_space`), nadpisywanie `shred` — bez sensu na SSD i btrfs z CoW, wolne.
- Opróżnianie DNS, rejestr Windows, `process.terminate`.

### 1.5 Licencja (GPL-3.0-or-later) — co repo na MIT może wykorzystać

To nie porada prawna; ostrożna interpretacja.

- **Wolno:** fakty — że aplikacja X trzyma cache w ścieżce Y, nazwy plików blokad, nazwy procesów, idea listy wyjątków,
  poziomy głębokości chronionych ścieżek, tryb podglądu, ostrzeżenia. Każdą ścieżkę trzeba ponownie sprawdzić na prawdziwym
  systemie i napisać własny tekst reguły.
- **Wolno:** ogólne algorytmy opisane prozą (np. „sprawdź PID w symlinku blokady”, „rmdir dopiero po dzieciach”).
- **Nie wolno:** kopiować dosłownie `cleaners/*.xml`, `protected_path.xml`, regexów, kodu Pythona ani tłumaczeń (`po/`), ani
  mechanicznie konwertować XML na nasz TOML. To tworzy utwór zależny wymagający GPL.
- **Nie wolno:** dostarczać XML BleachBita jako opcjonalnych danych w binarce na MIT. (Osobny importer instalowany przez
  użytkownika, który w czasie działania czyta jego własną instalację BleachBita, to szara strefa; pomijamy.)
- To samo dotyczy Stacera (GPL-3.0) oraz aplikacji krokiet/cedinia (`LICENSE_GPL_APP`). `czkawka_core` jest na MIT (do
  wykorzystania z informacją o licencji). Sweeper jest na LGPL-2.1+. kondo, npkill, cargo-sweep i gdu są na MIT; dust na Apache-2.0.

---

## 2. Inne programy czyszczące i analizatory

### 2.1 Stacer (C++/Qt, GPL-3.0, praktycznie martwy od wydania z 2019)

- System Cleaner (`stacer-core/Info/system_info.cpp`, `stacer/Managers/tool_manager.cpp`):
  - cache paczek (APT, pacman; uwaga: gałąź YUM/DNF wywołuje funkcję pacmana — błąd)
  - raporty awarii: wszystkie pliki w `/var/crash`
  - logi aplikacji: **wszystkie pliki** w `/var/log` (najwyższy poziom, łącznie z aktywnymi logami)
  - cache aplikacji: każdy wpis w `~/.cache`
  - kosz
- Brak sprawdzania działających procesów, brak listy wyjątków. Wniosek: unikać reguł w stylu „usuń cały /var/log”.

### 2.2 KDE Sweeper (C++/Qt, LGPL-2.1+)

- Tylko prywatność, 11 akcji (`sweeper.cpp` `InitActions`): historia schowka, ostatnie dokumenty, historia uruchamianych
  poleceń, ostatnie aplikacje, miniatury, ciasteczka, favikony, historia stron, cache stron, podpowiedzi formularzy, polityki ciasteczek.
- Jedno okno potwierdzenia: „You are deleting data that is potentially valuable to you.” Głównie pod KDE/Konquerora.

### 2.3 czkawka / krokiet (Rust; rdzeń MIT, aplikacje GUI GPL)

- Workspace: `czkawka_core` (biblioteka), `czkawka_cli`, `czkawka_gui` (GTK4, starsze), `krokiet` (Slint, obecne GUI),
  `cedinia` (mobilne). Rdzeń `rust-version = 1.94.1`, edycja 2024.
- Narzędzia (`czkawka_core/src/tools/`): duplicate, empty_files, empty_folder, big_file, temporary, similar_images,
  similar_videos, same_music, invalid_symlinks, broken_files, bad_extensions, bad_names, exif_remover, video_optimizer,
  disk_usage.
- **Przechodzenie** (`common/dir_traversal.rs`): bez crate do przechodzenia. BFS poziom po poziomie:
  `folders_to_check.into_par_iter().with_max_len(2)` po `std::fs::read_dir`, z `DirEntry::file_type()` (d_type, bez dodatkowego
  stat) i stat tylko dla plików, które przejdą filtry. Flaga zatrzymania sprawdzana przy każdym wpisie. Narzędzie disk usage
  (`tools/disk_usage/scan/portable.rs`) to rekurencyjne `par_extend` na podkatalog, rozmiary = `metadata.len()` (rozmiar
  pozorny), pomija inne systemy plików.
- Crate’y: `rayon 1.12`, `crossbeam-channel`, `blake3`, `xxhash-rust` (xxh3), `crc32`, `trash 5.3` (opcjonalny kosz przez portal
  `ashpd` dla Flatpaka), `file-id` (filtr twardych linków).
- **Pliki tymczasowe**: lista rozszerzeń `#, thumbs.db, .bak, ~, .tmp, .temp, .ds_store, .crdownload, .part, .cache, .dmp,
  .download, .partial`.
- **Puste katalogi**: katalog jest pusty, jeśli zawiera tylko puste katalogi; usuwanie przez
  `remove_folder_if_contains_only_empty_folders` (ponowne sprawdzenie w chwili usuwania).
- **Nieprawidłowe symlinki**: zgłasza nieistniejący cel i nieskończoną rekurencję.
- **Duplikaty**: grupy po rozmiarze -> filtr twardych linków po identyfikatorze pliku -> wstępny hash pierwszych
  `2 * PREHASHING_BUFFER_SIZE` bajtów -> pełny hash (domyślnie Blake3, Crc32, Xxh3); wyniki wstępnego i pełnego hasha
  zapisywane na dysku z kluczem ścieżka/rozmiar/mtime.
- Domyślne wykluczenia: katalogi `/proc /dev /sys /snap`; pozycje `*/.git/*, */node_modules/*, */lost+found/*, */Trash/*, */.Trash-*/*`.
- Opcja „przenieś do kosza” zamiast usuwania.

### 2.4 kondo (Rust, MIT) — czyszczenie artefaktów projektów

- Przechodzenie: `walkdir` (opcje `follow_links`, `same_file_system`); `ignore` do skanu daty ostatniej zmiany. Opcja `--older`.
- Znaczniki projektów -> katalogi artefaktów (`kondo-lib/src/lib.rs`):

| Znacznik | Typ | Artefakty |
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

- Zgłoszenia: #149 Unity rozpoznawane jako .NET; #55 `kondo ~` nic nie robiło; #127 prośba o tryb próbny.

### 2.5 cargo-sweep (Rust, MIT, według README nieutrzymywany; poleca `cargo-clean-all`)

- Wybiórcze czyszczenie `target/`: `--time N` (starsze niż N dni), `--installed` (artefakty niezbudowane przez zainstalowane
  toolchainy, przez dopasowanie hasha wersji rustc zgodnego z `StableSipHasher128` Cargo), `--toolchains`, `--maxsize 10GiB`,
  `--dry-run`, rekurencyjnie.
- Pomysł dla XCleanera: pokrętła „zostaw artefakty nowsze niż N dni” i „zmniejsz do rozmiaru” dla reguł `build-artifact`.

### 2.6 npkill (TypeScript, MIT, v1.0.0)

- Profile (`src/core/constants/profiles.constants.ts`): node, python, java, android, dotnet, rust, elixir, haskell, scala,
  cpp, unity, unreal, godot, clojure, elm, php, dart, swift, zig, terraform.
- Dopasowania celów: `{ target: 'target', nextTo: ['Cargo.toml'] }`, `{ target: 'obj', contains: ['project.assets.json'] }`,
  `{ target: 'Library', nextTo: ['Assets', 'ProjectSettings'] }` — precyzyjniejsze niż w kondo.
- `GLOBAL_IGNORE`: nigdy nie wchodzi do `.git .svn .hg .Trash .nvm .rustup .pyenv .asdf .deno .volta .bun .vscode .idea
  node_modules __pycache__ .cache .venv venv` (ale nadal je zgłasza, jeśli pasują do celu).
- Oznaczanie wrażliwych (`files.service.ts`): wyniki w `~/.config`, `~/.local/share`, `~/.cache` albo dowolnym ukrytym katalogu
  najwyższego poziomu `~/.*` (poza `.npm`, `.pnpm`) są oznaczane `isSensitive` z powodem. Wątki robocze do liczenia rozmiarów.

### 2.7 Narzędzia do zajętości dysku: ncdu, gdu, dust, dua

| Narzędzie | Równoległość | Podstawa rozmiaru | Twarde linki | Montowania |
|---|---|---|---|---|
| ncdu 2.9 | `-t N` (domyślnie 1) | bloki; `--apparent-size` | liczone raz | `-x`, `--exclude-kernfs` |
| gdu (Go) | 2×GOMAXPROCS miejsc | `st_blocks*512` | `(dev,ino)`, gdy nlink>1 | `-x`; ignoruje `/proc /dev /sys /run` |
| dust (Rust) | rayon | bloki; `-s` pozorny | `(ino,dev)` FxHashSet | `-x` przez zbiór dev |
| dua-core (Rust) | pula z kradzieżą zadań | wybiera wywołujący | wywołujący | wywołujący |

- ncdu ma też `--exclude-caches` (pomija katalogi z `CACHEDIR.TAG`, https://bford.info/cachedir/) i oznacza twarde linki `H`.
- dust: deduplikuje po (inode, device) tylko w trybie zajętości dysku; wstępnie pobiera metadane, żeby uniknąć podwójnego stat na
  systemach plików bez d_type.
- Wnioski: raportować rozmiar **przydzielony** (`st_blocks * 512`) jako „zwolni się na dysku”, a pozorny jako drugorzędny;
  deduplikować twarde linki po `(st_dev, st_ino)`, gdy `st_nlink > 1`; domyślnie nigdy nie wchodzić do innych systemów plików.
- **Uwaga btrfs (zaobserwowane):** `/` (`@`) i `/home` (`@home`) na tej samej partycji zgłaszają różne `st_dev` (34 i 53).
  Według dokumentacji btrfs każdy podwolumin ma własną przestrzeń i-węzłów, więc zagnieżdżone podwoluminy (snapshoty, magazyn
  kontenerów) też będą wyglądać na „inny system plików” przy sprawdzaniu `st_dev` (niezweryfikowane dla zagnieżdżonych,
  niezamontowanych podwoluminów). Do tego ekstenty współdzielone przez reflinki i snapshoty są w całości liczone przez
  `st_blocks`, więc „zwolnione miejsce” na btrfs może być zawyżone (niezweryfikowane; dokładne liczby dają
  `compsize`/`btrfs fi du`).

---

## 3. CCleaner (Windows) przeniesiony na Linuksa

Moduły wymienione w Products Policy CCleanera i artykule o Health Check: Health Check (zakładki Junk / Privacy / Speed / Update),
Custom Clean, Smart Clean, Scheduled Cleaning, Registry Cleaner, Disk Analyzer, Drive Wiper, Uninstaller, Startup Manager,
Browser Plugin Manager, Software Updater, Driver Updater, Performance Optimizer, Duplicate Finder, Cloud Drive Cleaner.
(Wątek społeczności spiera się, czy Drive Wiper nadal istnieje w v7; nie da się tego sprawdzić na oficjalnych stronach.)

| Funkcja CCleanera | Odpowiednik na Linuksie | XCleaner |
|---|---|---|
| Pliki tymczasowe | `~/.cache`, własne pliki w `/tmp` | Faza 1 |
| Cache przeglądarek | `cache2` profilu, `Cache` | Faza 1 |
| Ciasteczka / historia | bazy SQLite przeglądarek | Faza 1, na wyraźny wybór |
| Kosz | Kosz XDG | Faza 1 |
| Cache miniatur | `~/.cache/thumbnails` | Faza 1 |
| Logi aplikacji (użytkownik) | `~/.local/state`, logi aplikacji | Faza 1 |
| Zrzuty awarii (użytkownik) | katalogi awarii aplikacji | Faza 1 |
| Zrzuty awarii (system) | `coredumpctl`, `/var/lib/systemd/coredump` | Faza 2 |
| Logi systemowe | vacuum journald, zrotowane `/var/log` | Faza 2 |
| Cache paczek | `paccache`, cache pomocników AUR | Faza 2 (`~/.cache/yay` w fazie 1) |
| Osierocone pakiety | `pacman -Qdtq` | Faza 2, raport |
| Disk Analyzer | treemap / drzewo rozmiarów | Faza 1 |
| Duplicate Finder | porównanie hashy | Później (opcjonalnie) |
| Health Check | bezpieczny zestaw jednym kliknięciem | Faza 1 (jako „Szybkie czyszczenie”) |
| Smart/Scheduled | timer użytkownika systemd | Później |
| Startup Manager | `~/.config/autostart`, jednostki użytkownika | Nigdy (poza zakresem) |
| Uninstaller | pacman/flatpak | Nigdy (XUninstaller) |
| Wtyczki przeglądarek | rozszerzenia | Nigdy |
| Registry Cleaner | nie dotyczy | Nigdy |
| Drive Wiper | `shred`, nadpisywanie wolnego miejsca | Nigdy |
| Software/Driver Updater | pacman, fwupd | Nigdy |
| Performance Optimizer | nie dotyczy | Nigdy |
| Cloud Drive Cleaner | nie dotyczy | Nigdy |

---

## 4. Szybkie skanowanie w Rust

### 4.1 Stan crate’ów (API crates.io, 2026-11-22)

| Crate | Wersja | MSRV | Stan |
|---|---|---|---|
| walkdir | 2.5.1 (2026-11-21) | nie ustawiono (edycja 2018) | utrzymywany |
| jwalk | 0.9.0 (2026-08-05) | nie ustawiono | **zarchiwizowany**, „use dua-core” |
| dua-core | 6.1.0 (2026-11-03) | 1.88 | nowy (2026-08), utrzymywany |
| ignore | 0.4.34 (2026-10-27) | 1.88 | utrzymywany (ripgrep) |
| rayon | 1.12.0 | 1.80 | utrzymywany |
| rustix | 1.1.6 | 1.65 | utrzymywany |
| nix | 0.32.0 | 1.69 | utrzymywany |
| libc | 0.2.191 | 1.65 | utrzymywany |
| globset | 0.4.21 | 1.88 | utrzymywany |
| trash | 5.4.0 | 1.87.0 | utrzymywany |
| crossbeam-channel | 0.5.17 | 1.60 | utrzymywany |

Lokalny toolchain: rustc 1.100.0 (Arch), więc wszystkie MSRV są spełnione.

### 4.2 Porównanie

| Podejście | Równolegle | Koszt stat | Uwagi |
|---|---|---|---|
| walkdir | nie | `lstat` po pełnej ścieżce | proste, posortowane, `same_file_system` |
| jwalk | rayon | stat po pełnej ścieżce | zarchiwizowany; unikać |
| dua-core | kradzież zadań | std (dirfd) | nowe API, młode |
| ignore WalkParallel | tak, `threads(n)` | stat po pełnej ścieżce | logika gitignore zbędna |
| std::fs + rayon | tak | `statx` względem dirfd | pełna kontrola, ~150 linii |
| surowe getdents64/statx | tak | minimalny | unsafe, mały zysk |

Sprawdzone szczegóły:

- `walkdir::DirEntry::metadata()` na Uniksie = `fs::symlink_metadata(&self.path)` (ścieżka za każdym razem rozwiązywana od korzenia).
- Równoległy `DirEntry` z `ignore` też używa `fs::metadata/symlink_metadata(&self.path)`. API:
  `WalkBuilder::new(p).standard_filters(false).same_file_system(true).threads(n).build_parallel().run(|| Box::new(|e| WalkState::Continue))`.
- `std::fs::DirEntry::metadata()` na Linuksie używa `dirfd(dir)` + nazwy (`metadata_c`, względne `statx`/`fstatat`), czyli to już
  wzorzec „openat + statx” bez kodu unsafe. `DirEntry::file_type()` używa `d_type` (bez wywołania systemowego), gdy system plików
  go podaje.
- dua-core: „Symbolic links are reported but never followed”, predykat `descend`, `#![deny(unsafe_code)]`, na Linuksie używa
  metadanych std.
- Surowe `getdents64` (rustix `fs::Dir`/`RawDir`) oszczędza tylko buforowanie readdir w libc; dominują wywołania stat.

Orientacyjny pomiar **(zaobserwowane, ciepły cache, 1,14 mln wpisów w `$HOME`)**:

| Polecenie | Czas |
|---|---|
| `du -sx ~` (jeden wątek, stat wszystkiego) | 3,7 s |
| `fd` 8 wątków (bez stat) | 0,57 s |
| `fd -j1` (bez stat) | 2,6 s |

Równoległość daje więc ~4,5× na readdir przy 8 wątkach; liczby dla zimnego cache na NVMe trzeba jeszcze zmierzyć.

### 4.3 Rekomendacja

- Napisać mały własny walker: **`std::fs::read_dir` + `DirEntry::file_type()` + `DirEntry::metadata()` (statx względem dirfd)
  na puli rayon**, rekurencyjne `par_iter` po podkatalogach (wzorzec dust/czkawka). Zależności: tylko `rayon`.
- Dla każdego wpisu użyć `std::os::unix::fs::MetadataExt`: `blocks()*512` (przydzielony), `len()` (pozorny), `dev()`, `ino()`, `nlink()`.
- Deduplikacja twardych linków: globalny `HashSet<(dev, ino)>` tylko dla plików z `nlink > 1` (tanie, rzadkie).
- Nie przekraczać `st_dev` korzenia skanu; zapisywać pominięte punkty montowania (i podwoluminy btrfs), żeby UI mogło je pokazać.
- Nigdy nie podążać za symlinkami; liczyć tylko sam link.
- „~50 znanych katalogów”: uruchamiać je jako niezależne zadania rayon w tej samej puli (`par_iter` po korzeniach); rozmiary są
  niezależne, a wiele katalogów jest maleńkich, więc jedna wspólna pula jest lepsza niż wątek na korzeń. Od razu kończyć dla
  korzeni, które nie istnieją.
- „Pełny rozkład `$HOME`”: ten sam walker, sumowanie bajtów na katalog od dołu, w pamięci tylko top-N dzieci każdego węzła
  (żeby nie trzymać drzewa z milionem węzłów), postęp strumieniowany przez `crossbeam-channel`/`async-channel` do pętli GTK
  (`glib::spawn_future_local`), ograniczony do ~10 Hz. Anulowanie przez `AtomicBool`.
- Ponowne użycie: wyniki znanych katalogów pochodzą z tego samego przejścia, gdy działa rozkład (dopasowanie ścieżek w trakcie).
- Wrócić do `dua-core`, gdy będzie miał kilka wydań; `ignore`/`globset` zostawić tylko do dopasowywania globów reguł (globset
  wystarcza; silnik przechodzenia niepotrzebny).

---

## 5. Rekomendacje dla XCleanera

### 5.1 Format reguł (własny TOML, osadzany przez `include_str!`)

Reguły pisać od zera, sprawdzone na prawdziwych systemach; nigdy nie przepisywać XML BleachBita.

```toml
[[rule]]
id          = "firefox.cache"
app         = "Firefox"
category    = "browser"          # browser | dev | system | app | trash | logs
kind        = "cache"            # cache | log | dump | trash | build-artifact | history | thumbnail
regenerates = true               # aplikacja odbudowuje to sama
risk        = "low"              # low | medium | high (high = domyślnie wyłączone + ostrzeżenie)
paths       = ["$XDG_CACHE_HOME/mozilla/firefox/*/cache2",
               "~/.var/app/org.mozilla.firefox/cache/mozilla/firefox/*/cache2"]
match       = "contents"         # contents (zostaw katalog główny) | self | files-only
older_than  = "0d"               # opcjonalny filtr wieku (mtime)
exclude     = ["**/.parentlock"]
warning     = ""                 # pokazywane po zaznaczeniu; wymagane dla risk=high

[rule.guard]
processes   = ["firefox", "firefox-bin"]       # comm / nazwa bazowa exe / argv0
lock_files  = ["~/.mozilla/firefox/*/lock"]    # wiszący symlink -> sprawdź PID
on_running  = "skip"             # skip | warn
```

Reguły artefaktów budowania (precyzja w stylu npkill, a nie same nazwy katalogów):

```toml
[[project]]
id        = "cargo"
marker    = { next_to = ["Cargo.toml"] }
targets   = [{ dir = "target", contains_any = ["CACHEDIR.TAG", ".rustc_info.json"] }]
kind      = "build-artifact"
regenerates = true
risk      = "low"
min_age   = "14d"                # domyślnie pomijaj niedawno budowane projekty
```

Reguły silnika:

- Rozwijać `~`, `$XDG_*` (z domyślnymi wartościami ze specyfikacji), Flatpak `~/.var/app/<id>/cache`, Snap `~/snap/<app>/common/.cache`.
- Każdy rozwiązany cel musi być wewnątrz `$HOME` po `realpath` jego **rodzica** (twarde sprawdzenie w fazie 1).
- Twarda lista zakazów (wkompilowana, nie do konfiguracji): samo `~`, `~/.ssh`, `~/.gnupg`, `~/.password-store`,
  `~/.local/share/keyrings`, katalogi użytkownika XDG (Dokumenty, Obrazy, Muzyka, Wideo, Pulpit, Pobrane — Pobrane tylko przez
  jawną regułę), każde `.git`, `/`, `/proc`, `/sys`, `/run`, `/dev`.
- Lista wyjątków użytkownika (ścieżki + globy), sprawdzana zarówno na ścieżce linku, jak i rozwiązanej.
- Świadomość CACHEDIR.TAG: katalog z poprawnym znacznikiem jest kandydatem na cache (Cargo `target`, ccache itd.).
- Respektować `.nobackup`? Nie — to nie jest sygnał do usuwania.

### 5.2 Semantyka usuwania (lekcje z `FileUtilities.delete` BleachBita)

- Skanować raz, zapisać `(dev, ino, ścieżka, rozmiar)`; w chwili usuwania ponownie zrobić `lstat` i przerwać pozycję, jeśli
  inode/dev się zmieniły (TOCTOU).
- Usuwać operacjami względnymi `openat`/`unlinkat` na dirfd otwartym z `O_NOFOLLOW|O_DIRECTORY` (rustix `fs::openat`,
  `unlinkat`, `AT_REMOVEDIR`); nigdy nie podążać za symlinkami na **żadnym** poziomie, także najwyższym (BleachBit #1889: link
  na pierwszym poziomie Kosza usunął `C:\Windows`).
- Nie przekraczać `st_dev` podczas usuwania; po cichu pomijać punkty montowania (`EBUSY`).
- Katalogi usuwać tylko od dołu, po dzieciach; zostawiać katalog główny reguły (`match = "contents"`), żeby nie psuć aplikacji
  ze stałymi uprawnieniami i ACL.
- Dla rodzajów `tmp`/`log` pomijać pliki otwarte przez jakikolwiek proces (jedno przejście `/proc/*/fd` na uruchomienie);
  bezpieczny domyślny = zostawić.
- Opcjonalnie „Przenieś do kosza” dla średniego i wysokiego ryzyka (kosz gio przez `gio::File::trash`, już w zależnościach GTK);
  trwałe usuwanie dla cache do odtworzenia.
- Logować każdą usuniętą ścieżkę do `$XDG_STATE_HOME/xcleaner/log/<znacznik czasu>.jsonl` (użytkownik może to sprawdzić).

### 5.3 UX, który warto skopiować

- Zawsze podgląd przed usunięciem (Preview w BleachBicie, `--dry-run` w kondo/cargo-sweep); przycisk „Wyczyść” nieaktywny do
  czasu skanu.
- Jeden zestaw „Szybkie czyszczenie” = tylko reguły `risk=low, regenerates=true` (pomysł z Health Check CCleanera, ale
  przejrzysty i edytowalny).
- Grupowanie po aplikacjach z jednym polem wyboru na aplikację i rozwijanymi opcjami; **jedna pozycja na aplikację** (bez
  zdublowanych kategorii Firefoksa — BleachBit #1737).
- Plakietki: „Odtwarza się”, „Działa — pominięto”, „Może wylogować”, „Ostrzeżenie” (`<warning>` BleachBita, powód `isSensitive` w npkill).
- Pokazywać zwolniony rozmiar przydzielony i liczbę pozycji; pokazywać pominięte montowania i podwoluminy.
- Widok rozkładu dysku (ncdu/gdu): sortowalna lista z „otwórz w Plikach” i „dodaj do listy wyjątków”.
- Strona artefaktów projektów (kondo/npkill): ostatnia zmiana, ikona typu projektu, filtr „starsze niż N dni”.
- `adw::Toast` z podsumowaniem + „Otwórz log”; potwierdzenie przez `adw::AlertDialog` z jawną listą pozycji wysokiego ryzyka.

### 5.4 Pułapki ze zgłoszeń błędów

| Zgłoszenie | Co się stało | Wniosek |
|---|---|---|
| BleachBit #1737 | dwie pozycje Firefoksa, utracone 200 kart | jedna pozycja na aplikację |
| BleachBit #1838 | zniknęły profile Chrome | nigdy nie ruszać korzeni profili |
| BleachBit #2321 | głęboki skan zepsuł venv ComfyUI | venv = wysokie ryzyko, tylko na wyraźny wybór |
| BleachBit #1616 | usunięte poprawne `.desktop` Wine/MenuLibre | bez heurystyk „zepsute” w fazie 1 |
| BleachBit #1798 | miniatury także w System→cache | reguły bez nakładania się |
| BleachBit #1889 | link w koszu usunął `C:\Windows` | nigdy nie podążać za linkami |
| BleachBit #1433 | usunięte pliki `.exe` (Windows) | wąskie globy, dziennik audytu |
| BleachBit #2392 | pytanie o linki do innych dysków | to samo urządzenie + bez podążania |
| kondo #149 | Unity widziane jako .NET | znacznik + sprawdzenie zawartości |
| kondo #55 | `kondo ~` nic nie robiło | testować `$HOME` jako korzeń |
| kod Stacera | usuwa wszystkie pliki `/var/log` | tylko zrotowane w fazie 2 |

Bardziej szczegółowe pułapki:

- Hurtowe usunięcie `~/.cache` (`System.cache` w BleachBicie, Stacer) usuwa też rzeczy, na których użytkownikom zależy: cache
  shaderów Flatpaka i Steama (wolne pierwsze uruchomienie), klony PKGBUILD w `~/.cache/yay`, indeksy wyszukiwania KDE/GNOME,
  cache shaderów Mesy. Zamiast jednej opcji „wszystko w ~/.cache” proponować reguły dla podkatalogów; zostawić opcję „Reszta
  ~/.cache” ze średnim ryzykiem.
- `session`, `cookies`, `site_data`, `passwords`, `history` przeglądarek: to utrata danych, nie „śmieci”. Domyślnie wyłączone,
  `risk=high`.
- `User/History` VS Code („backup” w BleachBicie) to lokalna historia plików — nie śmieci.
- `paccache -rk0` (BleachBit) usuwa wszystkie paczki z cache i uniemożliwia downgrade; faza 2 powinna domyślnie używać czegoś w
  stylu `paccache -rk2`. `journalctl --vacuum-size=1` usuwa wszystkie dzienniki; lepsze `--vacuum-time=2weeks`.
- Steam: nigdy nie ruszać `steamapps/` (gry, zapisy); opcjonalnie tylko `shadercache`/`depotcache`, z ostrzeżeniem.
- Cache deweloperskie (`~/.cargo/registry`, `~/.npm/_cacache`, `~/.cache/pip`, `~/.gradle/caches`, Go `~/.cache/go-build`) da się
  odtworzyć, ale kosztem transferu i czasu: `risk=medium`, pokazywać rozmiar ponownego pobrania.
- Duplikaty ścieżek aplikacji w Flatpaku i Snapie (`~/.var/app`, `~/snap`) muszą należeć do tej samej pozycji aplikacji.
- Snapshoty btrfs: usunięcie plików może nic nie zwolnić, jeśli snapshoty trzymają ekstenty; po wykryciu konfiguracji
  Snapper/Timeshift mówić „miejsce może się nie zwolnić, jeśli istnieją snapshoty” (metoda wykrywania niezweryfikowana).

### 5.5 Podsumowanie wyborów

1. Własne reguły TOML osadzane przy budowaniu; pola: id, app, category, kind, regenerates, risk, paths, match, guard, warning.
2. Fakty wolno poznawać z BleachBita; nigdy nie kopiować jego XML, regexów ani kodu (GPL-3.0).
3. Silnik przechodzenia: std `read_dir` + rayon, najpierw `d_type`, `statx` względem dirfd, `st_blocks*512`, deduplikacja `(dev,ino)`,
   bez przekraczania montowań, bez podążania za symlinkami.
4. Usuwanie: względem dirfd, `O_NOFOLLOW`, ponowna weryfikacja i-węzła przed usunięciem, od dołu, zostawianie korzeni reguł,
   pomijanie otwartych plików.
5. Blokada działającej aplikacji: nazwa procesu (comm/exe/argv0) + sprawdzenie PID w pliku blokady; domyślnie „pomiń”.
6. Twarda lista zakazów + lista wyjątków użytkownika + sprawdzenie zawierania w `$HOME` po realpath.
7. Domyślny zestaw = tylko cache niskiego ryzyka, które się odtwarzają; dane przeglądarek, venv i cache deweloperskie na
   wyraźny wybór, z ostrzeżeniami.
8. Artefakty budowania ze znacznikami `next_to`/`contains` w stylu npkill, CACHEDIR.TAG, filtr wieku.
9. Obowiązkowy podgląd, dziennik audytu, opcjonalne przeniesienie do kosza.
10. Poza zakresem: czyszczenie pamięci, swapu i wolnego miejsca, nadpisywanie, rejestr, menedżer autostartu, aktualizatory.
