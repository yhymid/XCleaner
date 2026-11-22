# Pulpity, dystrybucje i terminale — research na v0.2.0 (2026-11-22)

[English](DISTRO-DESKTOPS-TERMINALS-2026-11.md) | **Polski**

Zakres: `$HOME` na popularnych dystrybucjach i pulpitach oraz uruchamianie `sudo <cmd>` w nowym oknie terminala (`root.rs`).
Metoda: kod źródłowy upstream (shallow clone albo raw z GitHub/GitLab), specyfikacje freedesktop (repo `xdg/xdg-specs`) i strony man. Nic nie było testowane na innej dystrybucji. Pozycje z dopiskiem **[NZ]** są niezweryfikowane: oparte na wiedzy albo rozumowaniu, bez źródła pierwotnego.
Kodu ani danych z GPL (BleachBit, Stacer) nie kopiowano. Opisane są tylko fakty z kodu upstream.

Oznaczenia ryzyka są takie same jak w `rules.rs`: **Safe** (wolno zaznaczyć domyślnie), **Low** (oferować, ale nie zaznaczać), **Review** (tylko pokazać, decyzja należy do użytkownika), **Never** (blokada w `protect.rs`).

---

## 0. Najważniejsze wnioski dla właściciela

1. **`~/.cache` nie jest w całości cache.** Właściciel ustalił, że czyścimy go prawie w całości. Jest jednak kilka katalogów od pulpitów i usług, które w `~/.cache` trzymają **stan albo dane użytkownika**. Wszystkie potwierdziłem w źródłach. Trzeba je dopisać do `CACHE_STATEFUL` (Review) albo zrobić dla nich osobne reguły:
   - `sessions`: zapisane sesje XFCE (`xfce4-session-<display>`) i miniatury sesji.
   - `xfce4`: historia schowka clipmana, historia i frecency appfindera.
   - `obexd`: **pliki odebrane przez Bluetooth** (domyślny katalog obexd).
   - `ibus`: `ibus/libpinyin` i `ibus/libbopomofo` to **wyuczony słownik użytkownika** (chińskie IME).
   - `JetBrains`: katalog „system” IDE zawiera też **Local History**.
   - `tracker3`: indeks LocalSearch. Da się go odbudować, ale reindeksacja zajmuje dużo CPU i I/O, a do tego dochodzą tagi (patrz §1.1). Dlatego Low, nie Safe.
   - `kioexec`: lokalne kopie zdalnych plików, które są właśnie edytowane (KIO). Patrz §1.2.
   - `evolution`: offline’owa kopia poczty, kalendarza i kontaktów. **[NZ]** Mogą tam czekać zmiany zrobione offline, jeszcze niezsynchronizowane.
2. **Ochrona Steam jest przypięta do jednej ścieżki.** `protect.rs` chroni tylko `.local/share/Steam/{userdata,config,…}`. Na Debianie/Ubuntu Steam ma root w `~/.steam/debian-installation`, a we Flatpaku w `~/.var/app/com.valvesoftware.Steam/...`. Tam ochrona nie działa. Trzeba wykrywać root Steam i stosować te same listy względem każdego wykrytego roota (§2.3).
3. **Terminal: obecny mechanizm „czekaj, gdy terminal wrócił w < 1,5 s” jest kruchy.** Część terminali przy działającej już instancji domyślnie wraca od razu: gnome-terminal, kgx, ptyxis `--new-window`, xfce4-terminal, terminator, tilix, konsole z `UseSingleInstance`, `wezterm start`, footclient. Jeśli taki terminal wróci po 1,6 s, GUI uzna akcję za anulowaną. Jeśli użytkownik zamknie okno, GUI czeka 600 s. Rekomendacja w §4.3: flaga blokująca per terminal plus własny wrapper, który zapisuje do pliku raportu PID i kod wyjścia.

---

## 1. Pulpity

### 1.1 GNOME

| Ścieżka | Czym jest | Odtwarza się tak samo? | Gdy działa | Ryzyko / rekomendacja |
|---|---|---|---|---|
| `~/.cache/tracker3/files/` (+`errors/`) | Baza indeksu LocalSearch (dawniej tracker-miner-fs-3) | Tak, przez pełną reindeksację (CPU/IO, minuty–godziny) | **Nie**: `localsearch reset` najpierw zabija indekser (`SIGKILL` procesu właściciela `org.freedesktop.LocalSearch3`), dopiero potem usuwa katalog | **Low**. Odpowiednik upstream: `localsearch reset -s` (= kill + usunięcie `tracker3/files/errors` i `tracker3/files`). Strażnik: `localsearch-3`, `tracker-miner-fs-3`. Man mówi „Other than tags, actual data … should not be affected”, więc **tagi z indeksu przepadają** |
| `~/.cache/tracker/` | Tracker 2 (stary) | Tak | – | Safe, gdy tracker2 nie jest zainstalowany (pozostałość) [NZ] |
| `~/.local/share/nautilus/tags` | **Gwiazdki („Starred”) Nautilusa** | Nie | – | **Never**. Nautilus trzyma je we własnej bazie, nie w indeksie tracker3 (`nautilus-tag-manager.c`) |
| `~/.local/share/gvfs-metadata/` | Metadane GVfs: emblematy, własne ikony, ustawienia widoku folderów, pozycje | **Nie** (to dane użytkownika, nie cache) | – | **Never** (dopisać do `NEVER_INSIDE`) [NZ co do pełnej listy kluczy] |
| `~/.cache/gnome-software/` | appstream, ikony, oceny ODRS, metadane fwupd | Tak (pobierze ponownie) | gnome-software działa w tle jako usługa | Low/Safe. Strażnik: `gnome-software` [NZ: zawartość według układu katalogów] |
| `~/.cache/evolution/` | Kopie offline IMAP/EWS/CalDAV/CardDAV | Tak, z serwera | **Nie**: działają `evolution-source-registry`, `evolution-calendar-factory`, `evolution-addressbook-factory` | **Review**. POP i „Na tym komputerze” trzymane są w `~/.local/share/evolution` (Never). Zmiany offline mogą być niezsynchronizowane [NZ] |
| `~/.local/share/recently-used.xbel` | Lista ostatnich plików GTK (spec recent-files) | Pusta lista | Programy GTK obserwują plik i go przepisują | Review (prywatność). Działający program może dopisać wpis z pamięci [NZ] |
| `~/.local/share/gnome-shell/application_state` | Statystyka „często używane” | Nie | – | Never [NZ] |
| `~/.cache/thumbnails` | Miniatury (§3.1) | Tak | Tak | Safe (już jest) |

### 1.2 KDE Plasma (5/6)

| Ścieżka | Czym jest | Odtwarza się tak samo? | Gdy działa | Ryzyko / rekomendacja |
|---|---|---|---|---|
| `~/.local/share/baloo/index` (+`index-lock`) | Indeks Baloo (LMDB, często kilka GB) | Tak, przez reindeksację | **Nie**. `balooctl purge` czeka na wyrejestrowanie usługi D-Bus, usuwa **tylko plik `index`** i restartuje demona, jeśli wcześniej działał | **Low**. Zalecane: wywołać `balooctl6 purge` (Plasma 6; w Plasma 5 `balooctl`), a nie kasować samemu. Strażnik: `baloo_file`, `baloo_file_extractor`. Ustawienia są w `~/.config/baloofilerc`; tagi i oceny KDE leżą w xattr plików, nie w indeksie [NZ co do xattr] |
| `~/.cache/ksycoca6_*` (Plasma 5: `ksycoca5_*`) | Cache bazy usług i MIME (KService) | Tak, kbuildsycoca6 odbuduje go automatycznie | Tak (procesy mają go zmapowanego, a unlink nie psuje mapowania) | **Safe**. Ścieżka: `GenericCacheLocation + "/ksycoca6" + suffix` (`ksycoca.cpp`) |
| `~/.cache/<app>/qmlcache`, np. `plasmashell/qmlcache`, `kwin/qmlcache` | Skompilowany QML (`.qmlc`) | Tak | Tak | Safe [NZ: zachowanie Qt na podstawie dokumentacji QML disk cache] |
| `~/.cache/plasma_theme_*.kcache`, `icon-cache.kcache`, `plasma-svgelements*` | KSharedDataCache/KImageCache | Tak | Tak (mmap) | Safe [NZ] |
| `~/.cache/kioexec/krun/<pid>_<n>/<plik>` | **Lokalna kopia zdalnego pliku otwartego przez KIO.** Po zamknięciu programu kioexec pyta „upload the changes?” | Nie, jeśli kopia jest zmodyfikowana | **Nie** | **Review**. Katalog z żywym `<pid>` pomijać. Dla martwego PID oznaczać jako Low i ostrzegać, że to może być jedyna kopia zmian (`kio/src/kioexec/main.cpp`) |
| `~/.local/share/akonadi/` (`db_data`, `file_db_data`, `search_db`) | Baza Akonadi (KMail, KOrganizer) | Częściowo, przez resync | Nie | **Never**. Zamiast tego proponować `akonadictl vacuum`/`fsck` [NZ] |
| `~/.local/share/kactivitymanagerd/resources/database` | Historia aktywności **oraz ulubione w Kickoffie** | Nie | – | Never [NZ] |
| `~/.local/share/RecentDocuments/*.desktop` | Ostatnie dokumenty KDE | Pusta | Tak | Review (prywatność) [NZ] |
| `~/.local/share/klipper/history2.lst` | Historia schowka | Nie | – | Review (prywatność) [NZ] |
| `~/.cache/krunner/`, `~/.cache/drkonqi/` | Kopie baz zakładek przeglądarek dla krunnera; raporty awarii | Tak | Tak | Safe [NZ] |
| `~/.local/share/kwalletd/` | Portfel | – | – | Never (już jest) |

### 1.3 XFCE

| Ścieżka | Czym jest | Ryzyko / rekomendacja |
|---|---|---|
| `~/.cache/sessions/xfce4-session-<display>` i `sessions/thumbs-<display>/*.png` | **Zapisane sesje** („save session on logout”) i ich podglądy. `xfsm-util.c` i `xfsm-logout-dialog.c` zapisują przez `XFCE_RESOURCE_CACHE` | **Review / CACHE_STATEFUL**. Usunięcie kasuje zapisaną sesję |
| `~/.cache/sessions/<client>-*` | Pliki stanu klientów sesji (`xfce-session-client.c`) | Review |
| `~/.cache/xfce4/clipman/` | **Historia schowka** (`clipman/plugin.c`: `XFCE_RESOURCE_CACHE "xfce4/clipman/"`) | Review (prywatność, mogą tam być hasła) |
| `~/.cache/xfce4/appfinder/{history,frecency}` | Historia i ranking appfindera (`appfinder-model.c`) | Review |
| `~/.cache/thumbnails` | Tumbler używa standardowego katalogu | Safe |
| `~/.config/xfce4/**` | Konfiguracja, układ ikon pulpitu | Never |

### 1.4 Cinnamon, MATE, LXQt, Budgie, COSMIC

| Pulpit | Ścieżka | Ocena |
|---|---|---|
| Cinnamon | `~/.cache/cinnamon/` (pobrane indeksy i archiwa „spices”) | Safe [NZ] |
| Cinnamon | `~/.local/share/cinnamon/{applets,desklets,extensions,search_providers}` | **Never**: to zainstalowane dodatki [NZ] |
| Cinnamon/Mint | Nemo używa `~/.local/share/gvfs-metadata` i thumbnails jak GNOME | jak §1.1 |
| MATE | Caja: thumbnails standardowe, `~/.config/caja` to konfiguracja | Never dla config; brak specyficznego cache [NZ] |
| LXQt | pcmanfm-qt: thumbnails standardowe; `~/.cache/lxqt*` | Safe [NZ] |
| Budgie | Brak własnych dużych cache; korzysta z GNOME i GTK | jak GNOME [NZ] |
| COSMIC | Konfiguracja w `~/.config/cosmic/` (cosmic-config, wersjonowane klucze), stan w `~/.local/state/cosmic/` | Never [NZ: nie sprawdzałem cache COSMIC] |

### 1.5 Inne pozycje w `~/.cache`, które nie są cache (niezależne od pulpitu)

| Ścieżka | Dowód | Rekomendacja |
|---|---|---|
| `~/.cache/obexd/` | bluez `obexd/src/main.c`: `option_root = g_build_filename(g_get_user_cache_dir(), "obexd", …)`, opcja „Default $XDG_CACHE_HOME” | **CACHE_STATEFUL, Review**: to pliki odebrane przez Bluetooth |
| `~/.cache/ibus/libpinyin`, `~/.cache/ibus/libbopomofo` | ibus-libpinyin `PYLibPinyin.cc`: `userdir = g_get_user_cache_dir()/ibus/libpinyin` | **Review** (słownik użytkownika). `~/.cache/ibus/bus/registry` to zwykły cache (`ibusregistry.c`) |
| `~/.cache/JetBrains/<product><ver>/` | Dokumentacja JetBrains: „system directory contains caches and **local history** files”, na Linuksie `~/.cache/JetBrains/<product><version>` | **Review** dla całości. Bezpieczniej czyścić tylko podkatalogi `caches/`, `index/`, `log/` (Low), a nie `LocalHistory/` [NZ co do nazw podkatalogów]. Katalogi starszych wersji IDE, gdy istnieje nowsza, to pozostałość (Review) |
| `~/.cache/keepassxc` | Już w `CACHE_STATEFUL` | bez zmian |

---

## 2. Dystrybucje i formaty pakietów

### 2.1 Snap (Ubuntu i pochodne)

| Ścieżka | Fakty (źródło) | Rekomendacja |
|---|---|---|
| `~/snap/<app>/<rev>/` | `SNAP_USER_DATA`, czyli `$HOME` aplikacji. Przy odświeżeniu snapd **kopiuje dane starej rewizji do nowej** (`snapdata.go: copySnapData`), a przy usuwaniu rewizji usuwa jej katalog (`RemoveSnapData`) | Stare rewizje służą do `snap revert`. **Review** i nigdy nie zaznaczać. Pewna pozostałość to tylko katalog `<rev>`, którego nie ma w `/snap/<app>/` (rewizja nie jest zainstalowana). Właściwe czyszczenie to `snap remove --revision` (root, Faza 2) |
| `~/snap/<app>/<rev>.old` | Kosz snapd w trakcie operacji (`trashPath = path + ".old"`, przywracany przez `untrash` przy błędzie) | Pomijać, gdy działa `snapd` i trwa zmiana. Osierocony `.old` oznaczać jako Low |
| `~/snap/<app>/common/.cache/` | Pomocniki desktopowe ustawiają `XDG_CACHE_HOME=$SNAP_USER_COMMON/.cache` (`snapcraft-desktop-helpers/common/desktop-exports`, także `snapcraft-desktop-integration`) | Traktować **jak `~/.cache`**: po jednym elemencie na wpis, te same wyjątki `CACHE_STATEFUL`, `media_check`. Strażnik: proces z `snap.<app>.` w `/proc/<pid>/cgroup` [NZ: format cgroup] |
| `~/snap/<app>/<rev>/.cache` | Snapy bez desktop-helpers (HOME = SNAP_USER_DATA) | Jak wyżej, ale tylko dla rewizji `current` |
| `~/snap`, `~/snap/<app>`, `~/snap/<app>/common`, `~/snap/<app>/current` | Kontenery | `NEVER_EXACT` (dopisać) |

### 2.2 Flatpak

- `~/.var/app/<id>/cache` jest w sandboxie montowany jako `/var/cache`, a `cache/tmp` jako **`/var/tmp`** (`flatpak-run.c`, tablica `mounts[]`). Reguła `flatpak_cache_items` jest więc poprawna co do zasady.
- Poprawki:
  1. **Wyjątki `CACHE_STATEFUL` trzeba stosować także wewnątrz** `~/.var/app/<id>/cache/` (np. `org.keepassxc.KeePassXC`, aplikacje z ibus czy obexd w sandboxie). Dziś cały katalog `cache` jest jednym elementem Safe.
  2. Strażnik oparty na ostatnim członie id (`steam`, `firefox`) jest słaby. Lepszy sposób bez uruchamiania podprocesów: pliki `/run/user/$UID/.flatpak/<instance>/info` zawierają `[Application] name=<id>`. Działająca instancja oznacza pominięcie elementu. To ten sam mechanizm, którego używa `flatpak ps` [NZ: nazwa pliku według znanego układu].
  3. `cache/tmp` (= `/var/tmp` aplikacji) czyścić wyłącznie przy niedziałającej aplikacji.
- `~/.local/share/flatpak/repo/objects`: nigdy (twarde linki). Bez zmian względem katalogu HOME-JUNK.

### 2.3 Steam: różne lokalizacje roota

| Instalacja | Root Steam | Uwagi |
|---|---|---|
| Arch/CachyOS/Fedora RPM/Bazzite/SteamOS | `~/.local/share/Steam` (`~/.steam/root` i `~/.steam/steam` to symlinki) | SteamOS i Bazzite mają Steam natywny [NZ dla Bazzite] |
| Debian/Ubuntu (`steam-installer`/`steam-launcher`) | `~/.steam/debian-installation` | Spotykane w raportach użytkowników [NZ: brak dokumentu Valve] |
| Flathub `com.valvesoftware.Steam` | `~/.var/app/com.valvesoftware.Steam/...` | Manifest ma `--persist=.`, a wrapper tworzy symlinki `~/.steam/{root,steam}` → `.` względem katalogu danych (`steam_wrapper.py: STEAM_SYMLINKS`). W praktyce `.../.local/share/Steam` albo `.../data/Steam`. **Wykrywać przez symlink `~/.var/app/com.valvesoftware.Steam/.steam/root`** [NZ co do ostatecznej ścieżki] |
| Snap `steam` | `~/snap/steam/common/.local/share/Steam` | [NZ] |

**Rekomendacja:** dodać `steam_roots()`. Zbiera kandydatów (`~/.steam/root`, `~/.steam/steam`, `~/.steam/debian-installation`, `~/.local/share/Steam`, `~/.var/app/com.valvesoftware.Steam/{.steam/root,.local/share/Steam,data/Steam}`, `~/snap/steam/common/.local/share/Steam`), dla każdego rozwiązuje symlinki **w obrębie HOME**, odrzuca kandydatów bez `steamapps/` i usuwa duplikaty po `(dev, ino)`. Reguły S1–S9 i listy ochronne z `protect.rs` (`userdata`, `config`, `steamapps/common`, `compatdata`, `workshop`) trzeba wtedy stosować **względem każdego roota**. Dziś ochrona obejmuje tylko `.local/share/Steam/…`. Strażnik procesu jest ten sam (`steam`, `steamwebhelper`). Dla Flatpaka dochodzi do tego sprawdzenie instancji (§2.2).

### 2.4 Ubuntu i Mint

- Ubuntu ≥ 25.04 wybiera terminal przez `xdg-terminal-exec` z plikiem `~/.config/ubuntu-xdg-terminals.list` (dokumentacja Ubuntu 26.04, „change the default terminal”). Patrz §4.
- Mint: cache Cinnamona i Nemo jak w §1.4. Dodatkowych katalogów w HOME nie znalazłem [NZ].
- Na Ubuntu domyślny jest Snap (Firefox i Thunderbird jako snap). Cache Firefoksa leży w `~/snap/firefox/common/.cache/mozilla/firefox/<profil>`, a profil w `~/snap/firefox/common/.mozilla/firefox/`. Ten drugi to **Never**, analogicznie do `~/.mozilla` [NZ co do dokładnych ścieżek].

---

## 3. Specyfikacje freedesktop

### 3.1 Miniatury (Thumbnail Managing Standard)
- Katalogi: `$XDG_CACHE_HOME/thumbnails/{normal,large,x-large,xx-large}` oraz `fail/<program>/`. Rozmiary x-large i xx-large doszły w nowszej wersji, więc trzeba objąć cały `thumbnails/`.
- `fail/` zawiera rekordy nieudanego generowania. Ich usunięcie powoduje tylko ponowną próbę, więc **Safe**.
- **Wspólne repozytoria `.sh_thumbnails/`** w katalogach z plikami (np. na nośnikach) są według spec **tylko do odczytu** („A program should never…”). XCleaner nie powinien ich ruszać.
- Spec, sekcja „Deleting Thumbnails”: miniaturę należy usunąć, gdy oryginał nie istnieje. Opcjonalnie, w przyszłości: tryb „tylko osierocone miniatury” (odczyt `Thumb::URI` z PNG). Kosztowne, więc dziś zostaje czyszczenie całości.
- Stary katalog `~/.thumbnails` (przed XDG): Safe, pozostałość [NZ].

### 3.2 Ostatnie pliki
`recently-used.xbel` (GTK/GIO, `~/.local/share`). Stary spec recent-files mówi o `~/.recently-used`, ale ten plik jest przestarzały. Obie ścieżki: Review (prywatność).

### 3.3 Kosz (Trash spec 1.0)
- Kosz domowy: `$XDG_DATA_HOME/Trash/{files,info}` oraz opcjonalny `directorysizes`. `expunged/` to rozszerzenie GVfs.
- **`directorysizes`** to cache rozmiarów katalogów w koszu. Zapisywać go wolno tylko przez plik tymczasowy i atomowy `rename()`. Przy opróżnianiu całego kosza **usuwać też `directorysizes`**, bo dziś reguła `trash` go pomija i zostają nieaktualne wpisy.
- Spec zaleca usuwanie katalogów z kosza w całości, a nie plik po pliku. Usuwać parę `files/X` + `info/X.trashinfo`.
- **Kosze na innych nośnikach** (`$topdir` = punkt montowania):
  - (1) `$topdir/.Trash/$uid`. Implementacja **MUSI** sprawdzić, że `.Trash` ma ustawiony sticky bit i **nie jest symlinkiem**. Jeśli test nie przejdzie, katalogu **nie wolno używać** (także do opróżniania).
  - (2) `$topdir/.Trash-$uid`, gdy `.Trash` nie istnieje.
  - Przy listowaniu trzeba pokazywać oba warianty.
  - Udziały CIFS i FTP nie są objęte spec.
- **Rekomendacja dla XCleanera:** `fsops::Home` celowo nie przekracza montowań, więc inne dyski nie są dziś objęte. Plan:
  1. Czytać `/proc/self/mountinfo` i pomijać `nfs`, `cifs`, `smb3`, `fuse.sshfs`, montowania `ro`, `/proc`, `/sys`, `/run` (poza `/run/media`) i `/`.
  2. Dla każdego `$topdir` otwierać fd korzenia i dalej działać przez `openat2(RESOLVE_BENEATH|RESOLVE_NO_SYMLINKS|RESOLVE_NO_XDEV)`. Sprawdzać `.Trash` (sticky, nie symlink, `$uid` należy do użytkownika) i `.Trash-$uid` (właściciel = uid, nie symlink).
  3. Pokazywać to jako osobny element „Kosz na <etykieta dysku>”: **Review**, z listą nazw i dat z `.trashinfo`. Ścieżki w `info` na nośnikach są względne wobec `$topdir`.
  4. Element z nośnika wyjmowanego pomijać, jeśli zniknął między skanem a usunięciem (porównanie `st_dev`).
  5. To wykracza poza zasadę „HOME-only” Fazy 1, więc potrzebna jest osobna decyzja właściciela.

---

## 4. Terminale: `sudo <cmd>` w nowym oknie

### 4.1 Tabela argv (zweryfikowane w kodzie lub man upstream)

`T` = tytuł, `CMD…` = program i argumenty jako osobne elementy argv.

| Terminal | argv (zalecany) | Tytuł | Blokuje do końca komendy? | Źródło |
|---|---|---|---|---|
| **xdg-terminal-exec** | `xdg-terminal-exec [--title=T] CMD…` | `--title=` (gdy terminal ma `X-TerminalArgTitle`) | Robi `exec` wybranego terminala, więc **blokuje tylko wtedy, gdy blokuje wybrany terminal**. Spec tego nie definiuje | README/spec (status: **propozycja**, klucze `X-`) |
| **x-terminal-emulator** (Debian) | `x-terminal-emulator -T T -e CMD…` | `-T` | Polityka nic nie mówi o czekaniu | Debian Policy 11.8.3: `-e` przyjmuje wiele argumentów „as though passed directly to execvp” |
| **gnome-terminal** | `gnome-terminal --wait --title=T -- CMD…` | `--title`/`-t` | **Tylko z `--wait`** („Wait until the terminal's child exits”). Bez niego klient wraca od razu, bo okno otwiera serwer | man `gnome-terminal.xml`; `-e`/`-x` są przestarzałe |
| **kgx** (GNOME Console) | `kgx -T T -- CMD…` (albo `-e CMD…`) | `-T`/`--title` | **Nie**. `--wait` jest zdefiniowane, ale ma opis „(TODO)” i nie jest obsługiwane. Przy działającej instancji (GApplication unique) wraca od razu | `kgx-application.c`, `kgx-utils.c: kgx_filter_arguments` |
| **ptyxis** | `ptyxis -T T -- CMD…` | `-T`/`--title` („for new tab”) | **Tak** w aktualnym main: `--` (albo `-e`, zamieniany na `--`) bez `--tab`/`--new-window` włącza tryb standalone (`G_APPLICATION_NON_UNIQUE`). **Nie dodawać `--new-window`**, bo używa istniejącej instancji. Starsze wydania (≤ 47) mogą działać inaczej [NZ] | `ptyxis-early-options.c`, `main.c` |
| **konsole** | `konsole --separate -p tabtitle=T -e CMD…` | `-p tabtitle=…` (`-T` jest przyjmowane i **ignorowane**: „Debian policy compatibility, not used”) | Domyślnie tak (`UseSingleInstance` = false). Gdy użytkownik włączył „jeden proces” i nie ma tty (start z GUI), wraca od razu. **`--separate`/`--nofork` wymusza nowy proces** | `main.cpp: shouldUseNewProcess`, `konsole.kcfg`, `Application.cpp`, `Profile.cpp` |
| **xfce4-terminal** | `xfce4-terminal --disable-server -T T -x CMD…` | `-T` | Tylko z `--disable-server`. Bez tego okno otwiera działająca instancja przez D-Bus | docs.xfce.org (`-x` = reszta linii, `-e` = jeden string) |
| **mate-terminal** | `mate-terminal --disable-factory -t T -x CMD…` (albo `--`) | `-t`/`--title` | Tylko z `--disable-factory` | `terminal-options.c` |
| **lxterminal** | `lxterminal --no-remote -t T -e CMD…` | `-t`/`-T`/`--title=` | Z `--no-remote` tak [NZ co do domyślnego trybu zdalnego] | `lxterminal.c`: `-e` zbiera resztę argv; `--command=` jest parsowane przez `g_shell_parse_argv` |
| **qterminal** | `qterminal -e CMD…` | **brak opcji** | Raczej tak [NZ] | `main.cpp`: `-e`, pierwszy argument przez `parse_command` (dzielony na słowa!), reszta argv surowo |
| **terminator** | `terminator -u -T T -x CMD…` | `-T`/`--title` | Tylko z `-u`/`--no-dbus`. Inaczej, jeśli istnieje master, wysyła żądanie przez D-Bus i kończy | `optionparse.py`, skrypt `terminator` |
| **tilix** | `tilix --new-process -t T -e CMD…` | `-t`/`--title` | Tylko z `--new-process` („Not Recommended”). Domyślnie GApplication unique | `app.d`: `-e`/`-x` sklejane w **jeden string** (argumenty ze spacją są cytowane), potem dzielone ponownie; `application.d` |
| **alacritty** | `alacritty -T T -e CMD…` | `-T`/`--title` | Tak (nowy proces) | man alacritty(1): `-e` „must be last argument” |
| **kitty** | `kitty -T T CMD…` | `-T`/`--title` | Tak, o ile nie podano `-1/--single-instance` | `kitty --help` |
| **foot** | `foot -T T CMD…` | `-T`/`--title=` | Tak. `footclient` też czeka, chyba że dostanie `-N/--no-wait` | man foot(1), footclient(1) |
| **wezterm** | `wezterm start --always-new-process -- CMD…` (albo `wezterm -e CMD…`) | brak opcji CLI (`--class` zmienia app_id) | `start` **bez** `--always-new-process` deleguje do działającego GUI i wraca. `-e` to ukryty `BlockingStart` (zawsze nowy proces) | `wezterm-gui/src/main.rs`, `wezterm-gui-subcommands` |
| **ghostty** | `ghostty --title=T -e CMD…` | `--title=` (**przed** `-e`) | Tak. `-e` wymusza `gtk-single-instance=false` i `quit-after-last-window-closed=true` | `src/config/Config.zig` |
| **cosmic-term** | `cosmic-term -e CMD…` (albo `--`) | **brak** (nieznane flagi są ignorowane z ostrzeżeniem) | Tak, o ile stdin **nie jest** tty: przy tty robi `fork::daemon` i wraca. XCleaner daje `stdin(Stdio::null())`, więc OK. Jest też `--no-daemon` | `cosmic-term/src/main.rs` |
| **xterm** | `xterm -T T -e CMD…` | `-T` | Tak | xterm(1), wzorzec dla Debian Policy |

**Terminal Intent (D-Bus `org.freedesktop.Terminal1.LaunchCommand`)** jest w repo xdg-specs jako „0.1 (DRAFT)”. Przekazuje `exec` jako wektor argv, ale **nie ma sygnału zakończenia**, więc i tak potrzebny jest plik raportu. Na razie nie implementować.

### 4.2 Domyślne terminale

| Dystrybucja / pulpit | Domyślny |
|---|---|
| Fedora Workstation 41+ | Ptyxis |
| Ubuntu 25.10+ (GNOME) | Ptyxis (+ `xdg-terminal-exec`, `ubuntu-xdg-terminals.list`). ≤ 25.04: gnome-terminal |
| GNOME upstream (≥ 42), Arch `gnome` | GNOME Console (kgx) |
| Debian 13 GNOME | gnome-terminal albo kgx [NZ: nie sprawdziłem `task-gnome-desktop`] |
| KDE (Neon, Kubuntu, Fedora KDE, SteamOS desktop) | konsole |
| Bazzite | Ptyxis [NZ] |
| Mint Cinnamon | gnome-terminal [NZ] |
| XFCE / Xubuntu | xfce4-terminal |
| MATE | mate-terminal |
| LXQt / Lubuntu | qterminal |
| LXDE | lxterminal |
| Ubuntu Budgie | Tilix albo gnome-terminal [NZ] |
| COSMIC (Pop!_OS 24.04) | cosmic-term |
| CachyOS/Hyprland (u właściciela) | kitty (zależy od edycji) [NZ] |

### 4.3 Rekomendacje dla `root.rs`

1. **Poprawić argv** w `terminal_args`:
   - `gnome-terminal`: dodać `--wait`.
   - `kgx`: obecne `--title T --` jest poprawne (`-T` i `--title` to ta sama opcja). kgx nie blokuje, więc potrzebny jest punkt 3.
   - `ptyxis` (nowy wpis): `-T T --`.
   - `konsole`: `--separate -p tabtitle=T -e`.
   - `xfce4-terminal`: `--disable-server -T T -x`.
   - `terminator`: `-u -T T -x`.
   - `wezterm`: `start --always-new-process --`.
   - Nowe wpisy: `mate-terminal --disable-factory -t T -x`, `tilix --new-process -t T -e`, `lxterminal --no-remote -t T -e`, `ghostty --title=T -e`, `cosmic-term -e`, `qterminal -e`, `x-terminal-emulator -T T -e`.
   - Gałąź domyślna `_ => -e` jest zgodna z Debian Policy, więc może zostać.
2. **Kolejność wyboru:**
   - `$TERMINAL`, potem `xdg-terminal-exec` (tylko jeśli istnieje plik `*xdg-terminals.list` w `~/.config` lub `/etc/xdg`; bez konfiguracji wybór jest nieokreślony).
   - Potem terminal pasujący do `XDG_CURRENT_DESKTOP` (GNOME → ptyxis/kgx/gnome-terminal, KDE → konsole, XFCE → xfce4-terminal, MATE, LXQt, COSMIC, X-Cinnamon → gnome-terminal).
   - Potem `x-terminal-emulator`, a na końcu obecna lista.
3. **Niezawodne wykrywanie końca** (najważniejsze):
   - Zamiast `sudo …` uruchamiać w terminalu **własny wrapper** jako użytkownik, np. `<self> --term-wrap --report R -- sudo env … <self> --root …`.
   - Wrapper (a) dopisuje `wrapper\t<pid>` do pliku raportu, (b) uruchamia `sudo` jako dziecko, (c) po jego zakończeniu dopisuje `exit\t<kod>`.
   - GUI zawsze odpytuje raport, niezależnie od czasu powrotu terminala, i kończy, gdy pojawi się `done` albo `exit`, albo gdy PID wrappera zniknie z `/proc` (zamknięte okno: SIGHUP zabija wrappera).
   - Gdy wrapper nie zapisał się w ciągu np. 30 s, terminal się nie uruchomił.
   - Rozwiązuje to: gnome-terminal bez `--wait`, kgx, tryby single-instance, czekanie 600 s po zamknięciu okna i fałszywe „anulowano” po wolnym starcie.
4. **Środowisko:**
   - Terminale z serwerem (gnome-terminal-server, kgx, tilix, terminator, xfce4-terminal bez `--disable-server`, konsole single-instance) uruchamiają komendę w **środowisku serwera**, nie XCleanera. `env_remove(FOREIGN_ENV)` na nie nie działa. Nie jest to krytyczne, bo `sudo` resetuje env, a `APPIMAGELAUNCHER_DISABLE` idzie przez `env` w argv. Dlatego warto preferować flagi „nowy proces”.
   - **qterminal i tilix sklejają lub dzielą argumenty przez powłokę.** Ścieżki z `"`, `'`, `$` lub spacją (AppImage w `~/Pobrane/Moje Programy/`) mogą się rozjechać. Dla nich sprawdzać, czy `me` i ścieżka raportu zawierają wyłącznie bezpieczne znaki, a w przeciwnym razie pomijać ten terminal.
   - lxterminal `-e` przekazuje surowe argv (OK).

---

## 5. Proponowane nowe reguły i zmiany (podsumowanie)

| # | Zmiana | Ryzyko | Dowód |
|---|---|---|---|
| 1 | `CACHE_STATEFUL += sessions, xfce4, obexd, ibus, JetBrains, kioexec, evolution, tracker3` | Review | §1.1–1.5 |
| 2 | Reguła `kde-sycoca`: `~/.cache/ksycoca5_*`, `ksycoca6_*` | Safe | `ksycoca.cpp` |
| 3 | Reguła `kde-qmlcache`: `~/.cache/*/qmlcache`, `plasma_theme_*.kcache`, `icon-cache.kcache` | Safe | [NZ częściowo] |
| 4 | Reguła `baloo-index`: plik `~/.local/share/baloo/index` (strażnik `baloo_file`; najlepiej `balooctl6 purge`) | Low (reindeks) | `purgecommand.cpp` |
| 5 | Reguła `localsearch-index`: `~/.cache/tracker3/files` (strażnik `localsearch-3`, `tracker-miner-fs-3`; najlepiej `localsearch reset -s`) | Low (reindeks, tagi) | `tracker-reset.c`, man |
| 6 | `NEVER_INSIDE += .local/share/gvfs-metadata, .local/share/nautilus, .local/share/akonadi, .local/share/kactivitymanagerd, .local/share/cinnamon, .local/share/evolution, .var/app/*/data, .var/app/*/config` (te dwa ostatnie wymagają dopasowania wzorcem) | Never | §1, §2.2 |
| 7 | `NEVER_EXACT += snap, snap/<app>, snap/<app>/common, snap/<app>/current` | – | §2.1 |
| 8 | Snap cache `~/snap/<app>/common/.cache/*` jako drugi „~/.cache” | Safe z wyjątkami | desktop-exports |
| 9 | Snap stare rewizje (katalog `<rev>` bez `/snap/<app>/<rev>`) | Review | `snapdata.go` |
| 10 | Flatpak: wyjątki stateful wewnątrz `cache/`, strażnik przez `/run/user/$UID/.flatpak/*/info` | – | `flatpak-run.c` |
| 11 | `steam_roots()` + ochrona względem roota (Debian, Flatpak, Snap) | – | §2.3 |
| 12 | Kosz: usuwać też `Trash/directorysizes`; Kosze `.Trash-$uid` na innych dyskach jako Review (decyzja właściciela) | Review | Trash spec |
| 13 | Miniatury: nie ruszać `.sh_thumbnails`; dodać stary `~/.thumbnails` | Safe | Thumbnail spec |
| 14 | Prywatność (Review): KDE `RecentDocuments`, klipper, XFCE clipman | Review | §1.2–1.3 |
| 15 | `root.rs`: argv z §4.3, wrapper do wykrywania końca, wybór terminala według pulpitu | – | §4 |

---

## Źródła

- Debian Policy 11.8.3 x-terminal-emulator: https://www.debian.org/doc/debian-policy/ch-customized-programs.html
- xdg-terminal-exec (propozycja spec): https://github.com/Vladimir-csp/xdg-terminal-exec (README); MR: https://gitlab.freedesktop.org/terminal-wg/specifications
- Terminal Intent (draft 0.1): https://gitlab.freedesktop.org/xdg/xdg-specs/-/tree/master/terminal-intent
- gnome-terminal man: https://github.com/GNOME/gnome-terminal/blob/master/man/gnome-terminal.xml
- GNOME Console: https://github.com/GNOME/console/blob/main/src/kgx-application.c , `src/kgx-utils.c`
- Ptyxis: https://gitlab.gnome.org/chergert/ptyxis/-/blob/main/src/ptyxis-early-options.c , `src/main.c`, `src/ptyxis-application.c`; man: https://man.archlinux.org/man/extra/ptyxis/ptyxis.1.en
- Konsole: https://github.com/KDE/konsole/blob/master/src/main.cpp , `src/Application.cpp`, `src/settings/konsole.kcfg`, `src/profile/Profile.cpp`; https://docs.kde.org/stable_kf6/en/konsole/konsole/command-line-options.html
- xfce4-terminal: https://docs.xfce.org/apps/xfce4-terminal/command-line
- mate-terminal: https://github.com/mate-desktop/mate-terminal/blob/master/src/terminal-options.c
- lxterminal: https://github.com/lxde/lxterminal/blob/master/src/lxterminal.c
- qterminal: https://github.com/lxqt/qterminal/blob/master/src/main.cpp
- Terminator: https://github.com/gnome-terminator/terminator/blob/master/terminatorlib/optionparse.py , skrypt `terminator`
- Tilix: https://github.com/gnunn1/tilix/blob/master/source/app.d , `source/gx/tilix/application.d`
- Alacritty: https://man.archlinux.org/man/alacritty.1.en ; foot/footclient: https://man.archlinux.org/man/foot.1.en , https://man.archlinux.org/man/footclient.1.en ; kitty: `kitty --help` (lokalnie)
- WezTerm: https://github.com/wezterm/wezterm/blob/main/wezterm-gui/src/main.rs , `wezterm-gui-subcommands/src/lib.rs`
- Ghostty: https://github.com/ghostty-org/ghostty/blob/main/src/config/Config.zig
- COSMIC Terminal: https://github.com/pop-os/cosmic-term/blob/master/src/main.rs
- Ubuntu i Ptyxis: https://www.omgubuntu.co.uk/2025/05/ubuntu-25-10-to-include-new-image-viewer-terminal-apps , https://ubuntu.com/desktop/docs/en/26.04/how-to/change-the-default-terminal/ , https://phoronix.com/news/Ubuntu-Ptyxis-Recommended
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
- Trash spec 1.0 i Thumbnail spec: https://gitlab.freedesktop.org/xdg/xdg-specs (`trash/index.rst`, `thumbnail/thumbnail-spec.xml`, `recent-files/recent-file-spec.xml`)
