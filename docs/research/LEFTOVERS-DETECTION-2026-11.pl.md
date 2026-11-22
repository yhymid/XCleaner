# Pozostałości po odinstalowanych aplikacjach w HOME — projekt wykrywania (2026-11-22)

[English](LEFTOVERS-DETECTION-2026-11.md) | **Polski**

Pytanie właściciela (w skrócie): „Deinstalatory nigdy nie usuwają wszystkiego. Pozostałości po usuniętych aplikacjach
zawsze zostają w `~/.config` i `~/.local`. Jak sprawdzić albo udowodnić, które katalogi to pozostałości?”

Zakres: `~/.config/*`, `~/.local/share/*`, `~/.local/state/*`, `~/.var/app/*`, ukryte wpisy najwyższego poziomu `~/.*`.
Arch/CachyOS, Faza 1 (tylko HOME, nigdy root). Etykieta w UI: **„Pozostałości po odinstalowanych aplikacjach”**.
Nic tutaj niczego nie usuwa. Zasadę zaznaczania z góry ustala właściciel (patrz §8).

Metoda: research w sieci (adresy niżej, strony pobrane 2026-11-22), analiza silnika identyfikacji XUninstallera
(MIT, ten sam właściciel) i pomiar tylko do odczytu na tej maszynie (nazwy, rozmiary, mtime/atime/btime;
nie czytano treści plików osobistych). Skrypt pomiarowy to
`/tmp/claude-1000/xc-leftover/measure.py` (tymczasowy, nie ma go w repo).

---

## 0. W skrócie

- **Żadne istniejące narzędzie tego nie rozwiązuje.** Każde znalezione narzędzie to albo ręcznie prowadzona lista
  „program → ścieżki” (xdg-ninja, antidot, xdg-unused-data, BleachBit), albo lista plików bez właściciela tylko
  dla systemu (lostfiles, pacreport). Arch Wiki: „*Pacman will not remove configurations that the
  application itself creates (for example "dotfiles" in the home directory).*” Moderator forum
  dodaje: „*Generally having a generic command for this is borderline impossible*”.
- **Jedynym wyjątkiem jest Flatpak.** `~/.var/app/<id>` jest nazwany dokładnym ID aplikacji, więc
  „brak zainstalowanego refa z tym ID w żadnej instalacji” liczy się jako dowód. Flatpak definiuje to
  tak samo: `flatpak uninstall --delete-data` bez REF usuwa wszystkie dane „bez właściciela”
  (kod źródłowy sprawdzony niżej).
- **Co na Archu daje niemal dowód:** trzy niezależne fakty, które się zgadzają:
  1. **Nazwa:** znormalizowana nazwa wpisu równa się tożsamości pakietu, którego *ostatnie*
     zdarzenie w `pacman.log` to `removed`. Tożsamością może być nazwa pakietu, binarka, ID pliku desktop
     albo ostatni człon nazwy reverse-DNS. Binarki i ID desktop pochodzą z bazy plików repozytoriów
     (`pacman -Fl`) albo z rekordów XUninstallera.
  2. **Brak właściciela:** żadna zainstalowana rzecz nie używa tej nazwy. Chodzi o pakiet, binarkę w PATH
     (z właścicielem lub bez), ID desktop, WMClass, Flatpak, AppImage albo działający proces.
  3. **Czas:** **czas utworzenia (statx btime)** katalogu wypada kilka sekund po linii `installed`
     tego pakietu, a po usunięciu nic do niego nie zapisano.
     Na tej maszynie 13 z 23 dopasowań do usuniętych pakietów powstało **3 s do 28 min** po
     instalacji (discord +5 s, chromium +4 s, go +3 s, BSManager +30 s).
- **Zmierzone tutaj:** 218 wpisów.
  - 23 pasują do usuniętych pakietów. Wszystkie nazwy są przypisane poprawnie, ale jeden okazał się
    używany (patrz następny punkt).
  - Pozostałości to łącznie około **9,5 GB**: BSManager 5,4 GB, pnpm 3,0 GB, discord 708 MB,
    profil chromium + kopia psd 342 MB, shadPS4 66 MB i inne.
  - Około 3 GB więcej wygląda na pozostałości, ale ma tylko poziom DO PRZEJRZENIA: osu-winello 3,1 GB i
    Epic `com.epicgames.eglv2` 2,5 GB.
  - 45 wpisów zostaje **nieznanych**. Pochodzą z AppImage, ręcznych instalacji, gier i katalogów
    producentów. Nigdy nie wolno ich proponować jako pozostałości.
- **Pułapki, na które trafiliśmy:**
  - `Discord Key Storage` pasuje do usuniętego `discord`, ale powstał **po** usunięciu
    i należy do zainstalowanego `discord-canary`.
  - Cztery katalogi przeglądarek (`BraveSoftware`, `microsoft-edge`, `opera`, `vivaldi`) istnieją dla
    przeglądarek, których nigdy nie zainstalowano. Tworzy je instalator native messaging Claude Code.
  - `atime` jest bezużyteczny. Montowanie ma `relatime`, a każdy skan aktualizuje atime katalogu: wszystkie
    218 wpisów pokazuje 2026-11-21/22.
- **Do wzięcia z XU:** `identity/normalize.rs`, `lists.rs`, `removed.rs`, `score.rs`, budowanie
  identyfikatorów w `identifiers.rs` i dane xdg-ninja (`curated.rs`). To około 2,3 tys. linii kodu,
  tylko std + libc. Szczegóły w §3.

---

## 1. Istniejące rozwiązania (research w sieci)

| Narzędzie | Podejście | Znajduje resztki usuniętych aplikacji? | Licencja | Do wzięcia |
|---|---|---|---|---|
| xdg-ninja | ręczny JSON | nie (podpowiedzi XDG) | MIT | dane, zrobione w XU |
| antidot | ręczny YAML | nie | MIT | pominąć (podzbiór) |
| xdg-unused-data | ręczna lista + `which` | tak, z listy | MIT | dane, może |
| lostfiles | `/` kontra pacman | bez HOME | MIT | tylko idea |
| pacreport | pliki bez właściciela | bez HOME | MIT | tylko idea |
| BleachBit | CleanerML | nie (tylko śmieci) | GPL-3 | nie |
| flatpak `--delete-data` | dokładne ID aplikacji | tak, Flatpak | LGPL-2.1 | semantyka |
| Flatsweep | `~/.var/app` kontra katalogi aplikacji | tak, Flatpak | MPL-2.0 | tylko idea |
| mundus | ręczna baza | tak, z listy | GPL-3 | nie (martwy od 2016) |
| Revo / BCU | nazwa + punktacja | Windows | zamknięty / Apache | metoda (zrobione w XU) |

Szczegóły:
- **xdg-ninja** (https://github.com/b3nj5m1n/xdg-ninja, MIT, 3,4 tys. gwiazdek, HEAD `ebf481a`
  2026-10-04).
  - `programs/*.json` zawiera `{name, files:[{path, movable, help}]}` dla 655 programów.
  - Sprawdza, które znane ścieżki istnieją, i mówi, czy da się je przenieść do XDG.
  - Nie rozróżnia zainstalowanych i usuniętych. XU już importuje te dane z tego samego przypiętego
    SHA, bez tekstów `help` (`data/xdg-ninja-rules.json`, 52 KB, z podziękowaniem w
    `THIRD_PARTY.md`).
- **xdg-unused-data** (https://github.com/pawel-0/xdg-unused-data, MIT, 37 gwiazdek, ostatni push
  2024-09).
  - Jedyne znalezione narzędzie, które celuje dokładnie w nasze pytanie.
  - 336 plików `applications/*.json`: `{name, executables[], locations[{file:"$XDG_CONFIG_HOME/0ad"}]}`.
  - README: „*If the executable is missing but at least one of the associated files or
    folders exists, it is flagged*”.
  - Kiedyś miało funkcję usuwania i usunęło ją „*due to concerns about potential risks*”.
  - Wniosek: ręczna lista plus test „brak pliku wykonywalnego”. Zasięg ogranicza się do znanych
    aplikacji i brakuje historii pakietów, więc zmieniona nazwa binarki daje fałszywe trafienia.
  - Dane są na MIT i można je zaimportować jak xdg-ninja (wymagane podanie autorstwa). To
    opcjonalne, bo pomiar pokazuje, że dowody z pacmana pokrywają więcej.
- **antidot** (https://github.com/doron-cohen/antidot, MIT): około 111 reguł przenoszenia, podzbiór
  xdg-ninja. Pominąć.
- **lostfiles** (https://github.com/graysky2/lostfiles, plik MIT w repo).
  - Arch Wiki *Pacman/Tips and tricks → Identify files not owned by any package*:
    `comm -13 <(pacman -Qlq | sed 's,/$,,' | sort) <(find /etc /usr /opt … )`. „*The lostfiles
    script performs similar steps, but also includes an extensive blacklist to remove common
    false positives*”.
  - Tylko system, bez przypisywania. Przydatne tylko w Fazie 2.
- **pacreport --unowned-files** (pacutils, https://man.archlinux.org/man/pacreport.1.en).
  - Sprawdzanie plików bez właściciela z globami `IgnoreUnowned` i `PkgIgnoreUnowned` w
    `/etc/pacreport.conf`.
  - Przykład z podręcznika ustawia `IgnoreUnowned = home/*`, więc HOME jest celowo poza zakresem.
- **Głęboki skan BleachBita** (https://sources.debian.org/src/bleachbit/5.0.2-1/cleaners/deepscan.xml).
  - Pliki kopii (`*~`, `*.bak`), `.DS_Store`, `Thumbs.db`, pliki tymczasowe Office, pliki swap vima;
    nowsze wersje dodają `node_modules`/`venv`.
  - Bez logiki usuniętych aplikacji. GPL, więc niczego nie kopiujemy (AGENTS.md).
- **Flatpak** (https://man.archlinux.org/man/flatpak-uninstall.1.en): `--delete-data`
  „*Remove app data in ~/.var/app and in the permission store*”. „*When --delete-data is used
  without a REF, all 'unowned' app data is removed.*”
  - Kod `app/flatpak-builtins-uninstall.c`: przechodzi po `~/.var/app`. Pomija nie-katalogi.
    Pomija nazwę, gdy `flatpak_find_current_ref(name)` znajdzie ref w dowolnej instalacji.
    Pomija też nazwę, która nie jest poprawną nazwą Flatpaka. O każdy pozostały katalog pyta.
  - To DOKŁADNIE nasza reguła dla `~/.var/app`. XU celowo nigdy nie wywołuje `--delete-data`: to
    czyści też uprawnienia portali, a usuwać musi silnik deskryptorów.
- **Flatsweep** (https://github.com/giantpinkrobots/flatsweep, MPL-2.0, GTK4/libadwaita, Python).
  - Oznacza `~/.var/app/<katalog>`, który ma `cache`+`config`+`data`, nie ma spacji w nazwie i
    nie ma wpisu w `/var/lib/flatpak/app` ani `~/.local/share/flatpak/app`.
  - Flathub ostrzega, że może się mylić przy własnej ścieżce instalacji. Ignoruje
    `/etc/flatpak/installations.d`; my nie możemy.
- **Zasady ID aplikacji Flatpak** (https://docs.flatpak.org/en/latest/conventions.html): reverse-DNS,
  zasady nazw szyny D-Bus, domena małymi literami. Dane są w `~/.var/app/<app-id>/{data,config,cache}`.
- **ID plików desktop** (https://specifications.freedesktop.org/desktop-entry/latest/file-naming.html):
  - Nazwa bazowa przed `.desktop` „*should be a valid D-Bus well-known name*”, w formie reverse-DNS
    (`org.example.FooViewer`).
  - Nazwy w `~/.local/share/<rdns-id>` (tu `com.zarz.mornye`,
    `dev.stormix.deadlock-mod-manager`) odpowiadają więc ID desktop i ich ostatniemu członowi.
- **Electron** (https://www.electronjs.org/docs/latest/api/app): `userData` „*by default is the
  appData directory appended with your app's name*”. Nazwą jest `productName`, który „*will be
  preferred over name*”. To wyjaśnia `~/.config/Mullvad VPN`, `discord`, `VSCodium`.
- **Arch Wiki *System maintenance → Old configuration files*:** „*Remove unneeded configurations
  periodically, particularly in your home directory and ~/.config*”. Wiki nie podaje metody
  poza ręcznym przeglądem i własną listą (`rmshit.py`).
- **Fora (to samo pytanie zadawane wiele razy):**
  - https://bbs.archlinux.org/viewtopic.php?id=299467: propozycje to `find ~/.config -atime +720`
    (nie działa przy noatime), sortowanie po mtime, zostawienie `dconf`/`gtk-*`/katalogów sesji, usuwanie
    partiami, ponowne logowanie.
  - https://discuss.kde.org/t/app-for-deleting-obsolete-files-under-home-username/50476: funkcja fish
    dopasowuje nazwę pakietu w katalogach XDG i przenosi trafienia do kosza.
    Zgłoszone problemy: `ephoto-bl` kontra `ephoto`, `keepass` kontra `keepassxc` i „*Icons are
    usually part of icon themes*”.
  - https://forum.manjaro.org/t/how-to-clean-up-config-files-from-unninstalled-apps/169422:
    „*the user files of each app have its own static directory*” (Flatpak), bez pewnego
    sprawdzenia dla aplikacji natywnych.
  - https://discuss.cachyos.org/t/home-directory-orphan-file-management/35445: „*It really
    does require user input*”. pacman „*only knows about files in the package's inventory*”.
- **Wniosek:** nikt nie łączy **historii pakietów + czasu utworzenia + weta zainstalowanej tożsamości**.
  To połączenie jest przewagą XCleanera (i XU). Pomiar w §4 pokazuje, że działa.

---

## 2. Źródła dowodów na Arch/CachyOS (wszystkie czytelne bez roota)

| Źródło | Daje | Koszt tutaj |
|---|---|---|
| `/var/lib/pacman/local/*/{desc,files}` | zainstalowane nazwy, base, provides, pliki | 1537 pakietów |
| `/var/log/pacman.log` | czasy instalacji/usunięcia | 343 usunięcia |
| baza plików repozytoriów (`pacman -Fl`) | pliki usuniętych pakietów z repo | 3,4 s, 221/253 |
| `/var/cache/pacman/pkg` | archiwum usuniętego pakietu | tu 0 trafień |
| rekordy XU `~/.local/share/xuninstaller/applications/*.record` | aliasy, binarki, launchery zapisane przed usunięciem | 10 rekordów |
| `.desktop` (systemowe, użytkownika, eksporty Flatpak) | ID, argv0 z Exec, WMClass, Icon, Name | — |
| binarki w PATH, także bez właściciela | `cava` w `/usr/bin` nie ma właściciela | — |
| `/proc/*/comm` | działające aplikacje | — |
| instalacje Flatpak | ID aplikacji (użytkownika + systemowe + `installations.d`) | 1 aplikacja |
| AppImage (`~/Applications`, `~/.local/bin`) | nazwy | 3 |
| manifesty Steam | zainstalowane gry | — |
| statx btime | utworzenie katalogu | btrfs to ma |

Uwagi:
- `pacman.log` zaczyna się tu **2026-06-24**, a HOME był masowo przywracany
  2025-10-13 / 2026-09-04 / 2026-09-07. W te dni powstało 8–12 wpisów na każdy korzeń.
  Wszystko usunięte przed początkiem logu jest niewidoczne. Przykłady: `gnome-software`, pierwsze życie `yelp`,
  wcześniejsza historia `cava`.
- Cache pacmana **nie** miał archiwum żadnego usuniętego pakietu (czyści się go przez
  `paccache -ruk0` albo podobnie). To baza plików repozytoriów rozwiązuje `psd` →
  `profile-sync-daemon` i `plasmalogin` → `plasma-login-manager`.
  - To 82 MB tarów zstd/gzip. Celem XCleanera jest „bez wywoływania zewnętrznych programów”, więc
    czytanie jej w procesie wymaga `zstd` + `flate2` + czytnika tar.
  - Rekomendacja: v1 uruchamia `pacman -Fl <usunięte…>` raz, z wektorem argv, bez powłoki,
    i zapamiętuje wynik. To świadomy wyjątek dla zapytania tylko do odczytu. Jeśli właściciel
    tego zabroni, używać tylko dowodów z nazwy pakietu i rekordów XU.
  - Pakiety AUR (`dorion-bin`, `bs-manager-git`, `millennium`, `shadps4-*`) nie są w żadnej
    bazie plików. Dla nich pracę wykonują nazwa, btime i rekordy XU.
- **Atime nie nadaje się do użytku.** Montowanie ma `relatime`, a odczyt katalogu aktualizuje jego atime,
  więc własny skan XCleanera albo XU ustawia każdy katalog na „dziś”. Zmierzone: każdy wpis ma atime
  2026-11-21 albo 2026-11-22.
- **Mtime samego katalogu jest zanieczyszczony przez obcych piszących.** `~/.config/chromium` dostał
  `NativeMessagingHosts/com.anthropic.claude_code_browser_extension.json` 2026-11-20, cztery
  dni po usunięciu chromium. „Ostatnia aktywność” musi pomijać znane obce poddrzewa
  (`NativeMessagingHosts`, `.directory`, `.DS_Store`).

---

## 3. Silnik identyfikacji XUninstallera (MIT, wolno wykorzystać)

Ścieżka: repozytorium XUninstaller, `crates/xuninstaller-core/src/identity/`.
Dokumenty projektowe w tym samym repo w `docs/research/`: `FILE-IDENTIFICATION-2026-11.md` (synteza
10 agentów), `FILE-LEFTOVERS-2026-11.md` (model dowodów, przypadek Mullvad) i
`RESEARCH-WAVE2-2026-11.md` (sekcje 06 Electron, 17 cache AUR, 20/21 KDE/GNOME).

**Dowody, których używa:**
- **Własność pakietu.** Czyta bezpośrednio `desc`/`files` z lokalnej bazy pacmana, tylko pliki należące
  do roota, z `O_NOFOLLOW`. Buduje `PackageIdentity` dla każdego zainstalowanego pakietu z
  kilku identyfikatorów:
  - pkgname i pkgbase;
  - nazwy bazowe z `/usr/bin` i cele wrapperów w `/opt`;
  - ID `.desktop`, argv0 z `Exec`, `StartupWMClass`, `Icon`, `Name`;
  - ID AppStream i binarki;
  - ID i ścieżki gschema, ID polkit, nazwy D-Bus.
- **Usunięte pakiety.** `removed.rs::parse_pacman_log` zwraca pakiety, których **ostatnie**
  zdarzenie to `removed`, z czasem. Zapisane rekordy XU (`application_history`) dodają aliasy,
  nazwy wyświetlane, binarki i ID launcherów zebrane *przed* usunięciem.
- **Statyczne ekstraktory** (`extract.rs`):
  - Electron `app.asar` → `productName`/`name` z `package.json` (parser nagłówka asar, bez
    zależności);
  - VS Code `product.json` `dataFolderName` (→ `~/.vscode-oss`);
  - Unity `app.info` (Company/Product);
  - Steam `appmanifest_*.acf`.
- **Dane ręczne** (`curated.rs`): JSON xdg-ninja z przypiętego SHA plus lokalne reguły (Discord,
  Mullvad, katalogi producentów).
- **Normalizacja** (`normalize.rs`):
  - fold (w stylu NFKC, małe litery) i squash (tylko znaki alfanumeryczne);
  - obcięcie rozszerzenia, wersji i przyrostków pakowania/kanału;
  - podział reverse-DNS na producenta + ostatni człon;
  - tokeny i ograniczona odległość edycyjna.
- **Punktacja** (`score.rs`): liczby całkowite w stylu BCU, **tylko najlepszy alias** (punkty się nie sumują).
  - Wagi: pkgname/ID desktop/ID aplikacji +6, binarka/WMClass/Icon +5, nazwa wyświetlana +4.
  - Nazwa + znany przyrostek (`rc`, `-backups`, …) +3, pierwszy token +3, odległość edycyjna ≤1 +2.
  - Weta: należy do zainstalowanego pakietu albo inny zainstalowany pakiet pasuje równie dobrze
    (−4) lub lepiej (weto).
  - „Zapisywany po odinstalowaniu” −3.
  - Poziomy: ≥6 PreChecked, 2–5 Unchecked, 0–1 Hidden, <0 Dropped.
  - Do każdej punktacji dołączone są polskie uzasadnienia.
- **Twarde listy** (`lists.rs`): `SHARED_NEVER` (około 100 nazw), `VENDOR_PARENTS` (tylko ostatni człon),
  `GENERIC_STOPLIST`. Aliasy o długości ≤4 znaków pasują tylko dokładnie; dopasowanie przybliżone wymaga ≥6;
  binarka współdzielona przez ponad 20 pakietów jest ignorowana.
- **Pliki integracji** (`integration.rs`): pliki `.desktop`/autostartu, których Exec wskazuje
  aplikację, jednostki systemd użytkownika i ich linki `.wants`, ikony wg `Icon=`, JSON native-messaging oraz
  edycje `mimeapps.list` na poziomie tokenów.

**Przenośność do `xcleaner-core`** (linie kodu bez testów w pliku):

| Plik | Linie kodu | Zależności | Przenieść? |
|---|---|---|---|
| identity/normalize.rs | 288 | std | tak, bez zmian |
| identity/lists.rs | 285 | normalize | tak + lista NIEBEZPIECZNYCH XC |
| identity/removed.rs | 72 | `valid_package_name` | tak + zachować czasy instalacji |
| identity/score.rs | 549 | identifiers | tak, przestroić poziomy |
| identity/identifiers.rs | 749 | libc, `local_db::Fields` | tak, mały parser |
| identity/curated.rs | 200 | JSON w extract | tak + `xdg-ninja-rules.json` |
| identity/extract.rs | 510 | std | później (asar, product.json) |
| identity/integration.rs | 873 | — | częściowo (martwe `.desktop`, v0.0.2) |
| identity/mod.rs | 592 | wnętrze XU | nie, napisać własny adapter |
| application_history.rs | 739 | ścieżki XU | tylko czytnik `*.record` |
| removed_history.rs | 139 | — | nie (duplikat removed.rs) |

- **Powiązania do przecięcia:**
  - `crate::local_db::Fields`: ścisły tokenizer `%KEY%`. Skopiować około 80 linii albo napisać
    mały własny.
  - `crate::removal::valid_package_name`: około 10 linii.
  - `crate::runner::run_command("pacman-conf", ["LogFile"])`: zastąpić domyślnym
    `/var/log/pacman.log` plus odczytem `LogFile =` z `/etc/pacman.conf`.
  - `crate::application_history`, `Category`, `InventoryError`, `xdg_scan`.
  - Nic z GTK, żadnych nowych crate’ów. `xcleaner-core` już zależy tylko od `libc`.
- **Zmiany potrzebne dla XCleanera:**
  1. `removed.rs` musi zwracać też **czasy instalacji** każdego usuniętego pakietu
     (potrzebne do korelacji z btime). Dziś trzyma tylko ostatnie usunięcie.
  2. Semantyka `IdentifyMode`/`Tier`: XU zaznacza z góry przy ≥6, bo użytkownik właśnie wybrał „usuń
     tę aplikację”. XCleaner skanuje bez docelowej aplikacji, więc domyślne ustawienie musi być surowsze (§6).
  3. XU identyfikuje pozostałości *dla jednego docelowego pakietu*. XCleaner potrzebuje odwrotności: dla każdego
     wpisu w HOME znaleźć *najlepszego właściciela* wśród zainstalowanych i usuniętych tożsamości.
     `IdentityIndex { installed, removed }` i `score_candidate` już to modelują, jak w
     `identify_foreign_in`. Pętlę trzeba tylko uruchamiać dla każdego wpisu zamiast dla każdego celu.
- **Współdzielić zamiast forkować?** Mały wspólny crate (`x-identity`) używany przez oba repozytoria uniknąłby
  dwóch rozjeżdżających się kopii. To decyzja właściciela; kopiowanie jest dopuszczalne na MIT z zachowaną
  linią copyright.

---

## 4. Pomiar na tej maszynie (tylko do odczytu, 2026-11-22)

Dane wejściowe: 1537 zainstalowanych pakietów. 253 pakiety, których ostatnie zdarzenie w logu to `removed` (log od
2026-06-24); 221 z nich rozwiązanych przez bazę plików. Flatpak: `com.hypixel.HytaleLauncher`
(użytkownika). `~/.var/app` nie istnieje. 3 AppImage, 2 skrypty w `~/.local/bin`.

Liczby (218 wpisów: `~/.config` 79, `~/.local/share` 107, `~/.local/state` 11, `~/.*` 21):

| Werdykt | Wpisy | Rozmiar |
|---|---|---|
| zainstalowany właściciel | 67 | 39 GB |
| wspólne/systemowe (lista zakazów) | 66 | 351 GB (Steam 346) |
| dopasowanie do usuniętego pakietu | 23 | 9,5 GB |
| prefiks zainstalowanego pakietu | 13 | 6,3 GB |
| zalążek integracji | 4 | 16 KB |
| nieznane | 45 | 13 GB |

### 4.1 Dopasowania do usuniętych pakietów (kandydaci na pozostałości)

Kolumny: wpis, rozmiar, usunięty pakiet i data, najnowszy mtime, Δ btime po instalacji tego pakietu,
ręczna weryfikacja. `cfg` = `~/.config`, `share` = `~/.local/share`.

| Wpis | Rozmiar | Usunięty pakiet | mtime | Δ btime | Weryfikacja |
|---|---|---|---|---|---|
| share/BSManager | 5,4 GB | bs-manager-git 10-18 | 10-06 | +30 s | prawda, dane gry |
| share/pnpm | 3,0 GB | pnpm 11-18 | 11-12 | przed logiem | prawda, twarde linki |
| cfg/discord | 708 MB | discord 10-21 | 10-20 | +5 s | prawda |
| cfg/chromium | 168 MB | chromium 11-16 | 11-20* | +4 s | prawda, profil |
| cfg/chromium-back-ovfs.base | 174 MB | psd 11-18 | 11-18 | −6 s przed usunięciem psd | prawda, kopia psd |
| share/shadPS4QtLauncher | 65 MB | shadps4-qtlauncher-bin 07-29 | 07-25 | +11 s | prawda |
| share/hhd | 12 MB | hhd 11-18 | 2025-10-14 | przed logiem | prawda |
| share/shadPS4 | 1 MB | shadps4-bin 07-29 | 07-25 | +90 s | prawda, zapisy? |
| share/zed | 1 MB | zed 09-05 | 09-05 | +10 min | prawda |
| share/millennium | 904 KB | millennium 11-18 | 10-29 | +16 s | prawda |
| share/kactivitymanagerd | 420 KB | kactivitymanagerd 09-05 | 06-20 | przed logiem | prawda |
| cfg/millennium | 332 KB | millennium 11-18 | 11-09 | +16 s | prawda |
| share/evolution | 100 KB | evolution-data-server 09-11 | 09-11 | przed logiem | prawda (pierwszy token) |
| cfg/go | 52 KB | go 11-18 | 11-12 | +3 s | prawda |
| cfg/evolution | 12 KB | evolution-data-server 09-11 | 09-04 | przed logiem | prawda (pierwszy token) |
| cfg/dorion | 12 KB | dorion-bin 11-04 | 11-04 | +5 s | prawda |
| share/yelp | 8 KB | yelp 09-05 | 2025-12-08 | przed logiem | prawda |
| cfg/com.spikehd.dorion | 4 KB | dorion-bin 11-04 | 11-04 | +138 s | prawda |
| cfg/psd | 4 KB | profile-sync-daemon 11-18 | 11-18 | +2 d | prawda (baza plików: `psd`) |
| share/glances | 4 KB | glances 09-05 | 09-05 | +28 min | prawda |
| cfg/Discord Key Storage | 4 KB | discord 10-21 | 10-22 | powstał po usunięciu | **fałsz**: discord-canary |
| share/dorion | 0 B | dorion-bin 11-04 | 11-04 | +5 s | prawda |
| share/plasmalogin | 0 B | plasma-login-manager 09-05 | 06-20 | przed logiem | prawda (baza plików) |

\* mtime 11-20 w `cfg/chromium` pochodzi wyłącznie z obcego zapisu do `NativeMessagingHosts/` (Claude
Code). Po wykluczeniu tego poddrzewa najnowszy zapis to 11-18 (odsynchronizowanie psd przy usuwaniu).

- Precyzja przypisania: **22/23**. Pomyłkę łapie weto „powstał po usunięciu” (§5.3).
- „Prawda” nie znaczy „bezpieczne do zaznaczenia z góry”:
  - BSManager trzyma instalacje Beat Saber i piosenki.
  - shadPS4 może trzymać zapisy.
  - chromium i katalogi `*-back-ovfs.base` to pełne profile przeglądarek (ciasteczka, loginy).
  - `store` pnpm jest twardo linkowany do projektów, więc zwolni się może mniej (patrz uwaga o
    `st_nlink` w HOME-JUNK-CATALOG).
- `chromium-back-ovfs.base` i `google-chrome-back-ovfs.base` powstały oba 2026-11-18 08:20:05.
  `profile-sync-daemon` usunięto o 08:20:11. To dowodzi, że to kopie „unsync” psd.
  Ręczna reguła `*-back-ovfs.base` + usunięty psd należy do PRAWIE PEWNYCH.

### 4.2 Prefiks zainstalowanego pakietu (13): to nie pozostałości, poza 3

| Wpis | Rozmiar | Dopasowano | Weryfikacja |
|---|---|---|---|
| share/umu | 3,7 GB | umu-launcher | w użyciu |
| share/osu | 2,4 GB | martwe `osuwinello-*.desktop` | **pozostałość** (osu-winello, ręcznie) |
| cfg/google-chrome-back-ovfs.base | 193 MB | google-chrome | **pozostałość** po psd |
| share/audacious-discord-rpc | 15 MB | audacious | niejasne (wtyczka) |
| cfg/google-chrome-for-testing | 332 KB | google-chrome | w użyciu (narzędzia) |
| cfg/hypr | 28 KB | hyprland (katalog share) | **w użyciu, krytyczne** |
| ~/.vscode-oss(-shared) | 32 KB | vscodium-bin | w użyciu (`dataFolderName`) |
| ~/.android | 12 KB | android-tools | w użyciu |
| cfg/cachyos | 8 KB | cachyos-* | w użyciu |
| share/node-steamuser | 4 KB | nodejs | niejasne |
| cfg/google-chrome-headless | 0 B | google-chrome | w użyciu |
| share/gnome-software | 0 B | gnome-desktop-* | pozostałość, sprzed logu |

- Dopasowania po prefiksie i pierwszym tokenie są **niejednoznaczne w obie strony**. Nigdy nie mogą przejść
  wyżej niż DO PRZEJRZENIA, a gdy wskazują na *zainstalowany* pakiet, domyślnie wetują.
- `hypr` ↔ `hyprland` pokazuje, dlaczego weta muszą być hojne. Przeoczony właściciel oznaczałby
  zaproponowanie konfiguracji kompozytora.

### 4.3 Zalążki integracji (4)

- `~/.config/{BraveSoftware,microsoft-edge,opera,vivaldi}` (po 4 KB, 2026-11-20) zawierają tylko
  `…/NativeMessagingHosts/com.anthropic.claude_code_browser_extension.json`. Żadna z tych
  przeglądarek nigdy nie była zainstalowana. Claude Code zapisuje swój manifest hosta do każdej ścieżki
  rodziny Chromium.
- Reguła: katalog, którego jedyną zawartością jest `NativeMessagingHosts/`, należy do aplikacji wskazanej przez
  manifest, a nie do przeglądarki. Nie pokazywać go jako pozostałości po przeglądarce. Aplikacja-właściciel go odtworzy.

### 4.4 Nieznane (45, 13 GB): nigdy nie proponowane

- Dane z ręcznych instalacji, AppImage i gier, nadal używane:
  - `wineprefixes` 4,9 GB i `osuconfig` 731 MB (osu-winello, foobar2000, l4d);
  - `yawl` 2,7 GB (runtime wrappera Wine);
  - `com.megalithofficial.basalt-launcher` 1,6 GB, `CapoRhythia` 100 MB, `Antra` 81 MB,
    `spotube` 30 MB, `gg.essential.mod` 29 MB, `Vencord` 4 MB (mtime 11-21);
  - `xdns` (projekt właściciela), pliki stanu `ukishima*` i `pill` (mtime 11-18…22).
- Katalogi producentów („Company” w Unity/Qt):
  - `Sandbox Interactive GmbH` (Albion Online), `Ndemic Creations`, `Meltytech` (Shotcut,
    niezainstalowany), `MelonLoader Installer`.
  - Nazwy producentów nigdy nie pasują do nazw pakietów. Potrzebują ręcznych reguł albo ekstraktora
    `app.info` z Unity i usuwania tylko ostatniego członu.
- `com.epicgames.eglv2` 2,5 GB:
  - `epic-games-launcher-bin` zainstalowano 11-12 i usunięto 11-18 (ten sam dzień co ostatni
    zapis). Ale katalog powstał **10-15**, przed tą instalacją, więc stworzyła go wcześniejsza instalacja
    (Flatpak/Heroic?).
  - Token producenta `epicgames` pasuje do usuniętego pakietu. Werdykt: DO PRZEJRZENIA, nie PRAWIE PEWNE.
- Stan bibliotek i narzędzi bez nazwy aplikacji-właściciela:
  - `matplotlib`, `jgit`, `nextjs-nodejs` (env-paths `<nazwa>-nodejs`), `.semgrep`, `rclone`
    (0 B), `.gsd-keyboard.settings-ported` (znacznik gnome-settings-daemon);
  - `resourcefullib`, `rpc-bridge-latest`, `steamforge*`, `simple-update-notifier`,
    `portmaster` + `io.safing.portmaster`, `warp-terminal`, `duckstation`, `PandoraLauncher`,
    `VRSideForge`, `sbox`, `SpotiFLAC-Next`, `stalefiles`, `omniroute-desktop`.
  - Wiele z nich to pewnie pozostałości (portmaster, warp-terminal, duckstation nie są
    zainstalowane). Ale nie ma **żadnego dowodu** poza „brak właściciela”: są starsze niż log albo pochodzą
    z AppImage. Zostają nieznane.

### 4.5 Zaobserwowane klasy fałszywych trafień dla listy zakazów

- Stan toolkitów i pulpitu nazwany od biblioteki, a nie aplikacji: `gtk-3.0`, `gtk-4.0`,
  `glib-2.0`, `dconf`, `ibus`, `goa-1.0`, `procps`, `pulse`, `pipewire`, `wireplumber`,
  `systemd`, `environment.d`, `menus`, `kdeglobals`, `QtProject.conf`, `mimeapps.list`,
  `user-dirs.*`, `gvfs-metadata`, `webkitgtk-4.1`, `gegl-0.4`, `gstreamer-1.0`, `folks`, `icc`,
  `vulkan`.
- Wspólne katalogi zdefiniowane przez specyfikacje: `applications`, `icons`, `fonts`, `mime`, `sounds`,
  `desktop-directories`, `Trash`, `keyrings`, `recently-used.xbel`, `user-places.xbel*`.
- Korzenie runtime’ów: `Steam`, `flatpak`, `containers`, `pki`, `man`, `dbus-1`, `IsolatedStorage`
  (.NET), `Sentry`, `openvr`, `openxr`, `unity3d`, `Valve`, `networkmanagement`, `orca`.
- Pliki kropkowe: `.bashrc`, `.bash_history`, `.ssh`, `.gnupg`, `.pki`, `.java`, `.npm`, `.npmrc`,
  `.cargo`, `.steam` i same korzenie XDG.
- Większość z nich pasuje *też* do zainstalowanego pakietu (`dconf`, `ibus`, `pipewire`), więc samo
  sprawdzenie „ma właściciela” byłoby dziś poprawne. Ale po usunięciu np. `ibus` katalog błędnie
  stałby się „pozostałością po ibus”. To nadal wspólny stan GTK/IM, więc lista zakazów musi wygrywać z
  dowodami z usuniętych pakietów.

---

## 5. Algorytm

### 5.1 Potok (na skan, około 1 s plus `pacman -Fl` około 3,4 s z cache)

1. **Zebrać zainstalowane tożsamości:**
   - lokalna baza (name, base, provides, `usr/bin/*`, `usr/share/applications/*.desktop`,
     metainfo, `opt/<x>/`, `etc/xdg/<x>`, jednostki użytkownika, usługi D-Bus, gschemy);
   - każda binarka w PATH, także bez właściciela;
   - wszystkie pliki `.desktop` (systemowe, użytkownika, eksporty Flatpak): ID, ostatni człon rDNS, argv0 z Exec, WMClass,
     Icon, Name;
   - ID aplikacji Flatpak ze wszystkich instalacji;
   - AppImage, `/proc/*/comm`, manifesty Steam.
   - Zakres: `identifiers.rs` z XU pokrywa część pakietową. Dodać PATH, procesy, Flatpak i
     AppImage.
2. **Zebrać usunięte tożsamości:** pakiety, których ostatnie zdarzenie w `pacman.log` to `removed` i
   które nie są teraz zainstalowane. Zachować każdy czas instalacji i usunięcia. Uzupełnić je z:
   - `pacman -Fl` (binarki, ID desktop, jednostki użytkownika);
   - plików `*.record` XU;
   - archiwów z cache, jeśli są.
3. **Wyliczyć** wpisy na głębokości 1 korzeni, przez `lstat` bez podążania. Dla każdego wpisu
   policzyć: rozmiar, najnowszy mtime bez obcych poddrzew, btime, „zawiera pliki wykonywalne”,
   „zawiera katalogi typu `drive_c`/zapisy” i liczbę plików.
4. **Znormalizować** nazwy: fold, squash, rdzeń (obcięcie rozszerzenia, wersji, przyrostka pakowania,
   `rc`/`-backups`/`-icons`/`-cache`/`-location`), reverse-DNS → (producent, ostatni człon).
   Aliasy ≤4 znaków pasują tylko dokładnie.
5. **Najpierw lista zakazów** (§5.4): taki wpis nigdy nie jest pozostałością.
6. **Weto zainstalowanych:** jeśli jakakolwiek zainstalowana tożsamość równa się wpisowi (squash albo rdzeń, albo ostatni
   człon rDNS = ostatni człon zainstalowanego desktop/Flatpaka), werdykt to `installed`. Koniec.
7. **Punktować względem usuniętych tożsamości** (§5.2), tylko najlepszy alias.
8. **Przydzielić poziom** (§5.3) i dołączyć uzasadnienia.

### 5.2 Dowody i wagi

Dodatnie (wziąć najlepszy sygnał nazwy, potem dodać kontekst):

| Id | Dowód | Waga |
|---|---|---|
| N1 | nazwa = pkgname/base usuniętego pakietu | +6 |
| N2 | nazwa = binarka/ID desktop/ostatni człon rDNS usuniętego pakietu (baza plików) | +6 |
| N3 | nazwa = alias/binarka/ID desktop z rekordu XU | +7 |
| N4 | pierwszy token = usunięty pakiet (≥4 znaki) | +3 |
| N5 | producent z rDNS = pierwszy token usuniętego pakietu | +2 |
| N6 | ręczna reguła (xdg-ninja, lokalna) dla usuniętego pakietu | +3 |
| T1 | btime 0–30 min po instalacji tego pakietu | +4 |
| T2 | btime 30 min–14 d po instalacji | +2 |
| T3 | najnowszy zapis ≤ usunięcie + 1 d | +1 |
| D1 | wskazuje go `.desktop` użytkownika z martwym Exec | +3 |

Ujemne i weta:

| Id | Dowód | Skutek |
|---|---|---|
| V1 | nazwa z listy zakazów | weto |
| V2 | równa zainstalowanej tożsamości | weto (zainstalowane) |
| V3 | **btime po usunięciu** | weto (stworzył to ktoś inny) |
| V4 | działa proces / otwarty fd w środku | zablokowane |
| V5 | zainstalowany pakiet ma ten sam pierwszy token | −3 |
| V6 | zapis > usunięcie + 1 d (bez obcych) | −4 |
| V7 | btime w kohorcie masowego tworzenia | T1/T2 = 0 |
| V8 | tylko `NativeMessagingHosts/` | zalążek, nie pozostałość |
| V9 | symlink, punkt montowania, inny właściciel | zablokowane |

- Kohorta masowego tworzenia to dzień, w którym powstało ponad 30% wpisów danego korzenia. Tutaj te
  dni to 2025-10-13, 2026-09-04 i 2026-09-07 (z XU R7).
- „Przed logiem” (btime wcześniejszy niż pierwsza linia logu) jest neutralne, nie ujemne.

### 5.3 Poziomy

- **PEWNE („pewne”):** wynik ≥ 9 z **N1/N2/N3 + T1**, bez weta, bez V5/V6. Do tego każde
  `~/.var/app/<poprawne-id>` bez aktualnego refa w żadnej instalacji Flatpak (własna reguła Flatpaka).
  - Tutaj: discord, chromium, go, dorion×3, millennium×2, shadPS4×2, BSManager. zed i
    glances dochodzą przez T2, co czyni je PRAWIE PEWNYMI, nie PEWNYMI.
- **PRAWIE PEWNE („prawie pewne”):** wynik 6–8, dowód z nazwy N1–N3, bez weta.
  - Tutaj: pnpm, hhd, kactivitymanagerd, yelp, plasmalogin, psd, zed, glances i para
    `*-back-ovfs.base` (ręczna reguła psd).
- **DO PRZEJRZENIA („do przejrzenia”):** wynik 3–5. Przykłady: tylko pierwszy token (evolution×2), tylko producent
  (com.epicgames.eglv2), tylko martwy launcher (osu, osuconfig).
- **NIEZNANE:** wynik < 3 albo brak dowodów. Nie są wymieniane w grupie pozostałości. Najwyżej na opcjonalnej
  liście diagnostycznej tylko do odczytu („Nieprzypisane katalogi”), bez pola wyboru.
- **Zaznaczanie z góry (decyzja właściciela; rekomendacja):**
  - Nigdy nie zaznaczać z góry DO PRZEJRZENIA ani NIEZNANYCH.
  - PEWNE lub PRAWIE PEWNE można zaznaczyć z góry tylko wtedy, gdy wpis **nie ma znaczników danych osobistych**:
    pliki profilu przeglądarki (`Cookies`, `Login Data`, `places.sqlite`), `drive_c`, zapisy,
    piosenki/instancje, `.git`, ponad N plików użytkownika albo klucze/tokeny (`*Key Storage*`,
    `*.key`). Dzisiejszy AGENTS.md mówi „pozostałe profile nigdy nie są zaznaczane z góry”, co już
    wyklucza chromium, discord, BSManager i parę ovfs.
  - Decyzja właściciela w toku: PEWNE + brak danych osobistych → zaznaczać z góry? Rekomendacja: **nie w
    v0.0.3**. Pokazać jako możliwe do zaznaczenia z pogrubionym chipem „pewne” i wrócić do tego po użyciu w praktyce.
- **Tylko ostatni człon:** katalogi nadrzędne producentów (`unity3d/<Company>`, `~/.config/<Org>`) są proponowane tylko jako
  `<Org>/<App>`. Rodzic znika dopiero, gdy stanie się pusty.

### 5.4 Lista zakazów (wspólne lub systemowe; nigdy nie pozostałość, nawet przy dokładnym dopasowaniu)

- Zacząć od `lists.rs::SHARED_NEVER` z XU plus listy NIEBEZPIECZNYCH z HOME-JUNK-CATALOG. Dodać nazwy
  zmierzone w §4.5:
  - toolkity: `gtk-2.0/3.0/4.0`, `gtkrc*`, `glib-2.0`, `dconf`, `ibus`, `fontconfig`,
    `gstreamer-1.0`, `webkitgtk-*`, `gegl-*`, `QtProject.conf`, `kdeglobals`, `kde.org`,
    `Trolltech.conf`;
  - sesja: `systemd`, `environment.d`, `autostart`, `menus`, `mimeapps.list`,
    `user-dirs.dirs`, `user-dirs.locale`, `pulse`, `pipewire`, `wireplumber`, `goa-1.0`,
    `procps`, `gvfs-metadata`, `folks`, `dbus-1`, `.gsd-*`;
  - katalogi ze specyfikacji: `applications`, `icons`, `fonts`, `mime`, `sounds`, `desktop-directories`,
    `Trash`, `keyrings`, `recently-used.xbel`, `user-places.xbel*`, `icc`, `vulkan`, `man`;
  - korzenie runtime’ów: `flatpak`, `containers`, `Steam`, `.steam`, `pki`, `.pki`,
    `IsolatedStorage`, `Sentry`, `openvr`, `openxr`, `unity3d`, `Valve`, `Epic`, `.mozilla`,
    `networkmanagement`, `orca`;
  - korzenie deweloperskie: `.cargo`, `.rustup`, `.npm`, `go` (jako `~/go`), `.java`, `.mono`, `.dotnet`,
    `.nuget`, `.wine`;
  - sekrety/konfiguracja: `.ssh`, `.gnupg`, `.bashrc`, `.bash_history`, `.profile`, `.npmrc`,
    `.claude*`, `.codex`, `hypr`, `fish`, `kitty`, `quickshell`.
- Dopasowanie po fold i bez rozszerzenia, tak samo jak `is_shared_never`.
- Listę pilnuje test jednostkowy, jak listę „nigdy” z v0.0.2.

### 5.5 Zmienione nazwy aplikacji, wielkość liter i odstępy

- Wielkość liter i odstępy: porównywać formy **po squash** (`Mullvad VPN` ≡ `mullvadvpn` ≡ `mullvad-vpn`,
  `discord-canary` ≡ `discordcanary`, `shadPS4` ≡ `shadps4`).
- Przyrostki pakowania: obcinać `-bin`, `-git`, `-appimage`, `-nightly`, `-beta` i wersje
  (`dorion-bin` → `dorion`, `bs-manager-git` → `bsmanager` ≡ `BSManager`). **Zostawiać słowa kanału**
  (`canary`, `ptb`, `beta` jako część nazwy aplikacji), gdy używa ich strona zainstalowana,
  żeby `discord` ≠ `discordcanary`.
- Zmiany nazw i forki: nazwa używana przez pakiet różni się od nazwy katalogu w kilku częstych
  przypadkach:
  - binarka w bazie plików: `profile-sync-daemon` → `psd`;
  - ID desktop / ostatni człon rDNS: `com.spikehd.dorion` → `dorion`;
  - `productName` Electrona: wymaga ekstraktora, gdy jest zainstalowany, albo rekordu XU, gdy usunięty;
  - provides/replaces: `procps-ng` dostarcza `procps`.
  - Zmiana nazwy pakietu (stara nazwa usunięta, nowa zainstalowana w tej samej minucie, np. `-git` ↔
    `-bin`) widać jako `removed X` + `installed Y` w jednej transakcji. Traktować tożsamości Y jako
    zainstalowane. Jeśli X i Y po squash dają ten sam rdzeń, to jest to **zainstalowany** właściciel (V2), nigdy
    pozostałość.
- Rodziny: najkrótsza zainstalowana nazwa, która jest prefiksem innych, to baza
  (`audacious` / `audacious-plugins`). Katalog nazwany od bazy ma właściciela, dopóki zainstalowany jest którykolwiek członek.

### 5.6 `~/.var/app` (Flatpak)

- PEWNE, gdy spełnione są wszystkie warunki:
  - nazwa to poprawne ID Flatpaka (zasady D-Bus, ≥3 człony);
  - brak refa z tym ID w instalacji użytkownika, systemowej i żadnej instalacji z
    `/etc/flatpak/installations.d/*.conf` (lista przez `flatpak list --app
    --columns=application,installation` albo katalogi `app/` każdej instalacji);
  - aplikacja nie działa (`flatpak ps` albo `/proc/*/root` wewnątrz aplikacji).
- Pokazywać `cache/` osobno (raczej bezpieczne) od `config/` + `data/` (dane użytkownika). Nigdy nie wywoływać
  `flatpak uninstall --delete-data`: resetuje też uprawnienia portali i usuwa bez naszego
  silnika deskryptorów. Czyszczenie magazynu uprawnień to osobna pozycja, tylko raport.
- Ta maszyna nie ma jeszcze `~/.var/app`, więc reguła nie jest tu przetestowana. Przetestować na fixturach.

### 5.7 Prezentacja w UI (po polsku)

- Grupa: **„Pozostałości po odinstalowanych aplikacjach”**, z podgrupami „pewne”,
  „prawie pewne” i „do przejrzenia”. Nieznanych wpisów nie ma w tej grupie.
- Wiersz: ścieżka, rozmiar, ostatni zapis (bez obcych poddrzew), chip poziomu i jednolinijkowe uzasadnienie
  zbudowane z najmocniejszego dowodu, np.:
  - „Usunięto pakiet discord 2026-10-21; katalog powstał 5 s po jego instalacji (2026-10-20);
    brak programu w systemie.”
  - „Usunięto pakiet profile-sync-daemon 2026-11-18; nazwa `psd` to jego program (baza plików
    pacmana).”
  - „Flatpak com.example.App nie jest zainstalowany (ani dla użytkownika, ani systemowo).”
  - „Do przejrzenia: nazwa zaczyna się od `evolution` (usunięty evolution-data-server
    2026-09-11); brak dokładnego dopasowania.”
- Rozwijane „Dlaczego?” wypisuje wszystkie składniki ze znakami, jak już robi `reasons: Vec<String>`
  w XU, łącznie ze sprawdzonymi wetami („żaden zainstalowany
  pakiet nie używa tej nazwy”, „żaden proces nie działa”).
- Chipy ostrzeżeń:
  - „profil przeglądarki: hasła/ciasteczka”;
  - „dane gry / zapisy”;
  - „twarde dowiązania: zwolni mniej niż X”;
  - „zmieniany po odinstalowaniu”.

---

## 6. Testy do napisania (na fixturach, jak `identity/tests.rs::owner_machine` w XU)

- discord usunięty + discord-canary zainstalowany: `discord` jest PEWNE, `discordcanary` jest zainstalowany,
  a `Discord Key Storage` jest zawetowany przez V3.
- `psd` rozwiązuje się przez dowód binarki z bazy plików. `*-back-ovfs.base` dostaje ręczną regułę psd.
- `hypr` nigdy nie jest pozostałością, dopóki zainstalowany jest `hyprland`. `ibus`/`dconf` nigdy nie są pozostałościami
  po usunięciu ich pakietu (lista zakazów wygrywa z dowodami z usuniętych).
- Katalog przeglądarki zawierający tylko NMH to zalążek, nie pozostałość.
- Kohorta masowego tworzenia zeruje T1/T2. btime sprzed logu jest neutralny.
- Binarka w PATH bez właściciela (`/usr/bin/cava`) wetuje `~/.config/cava`.
- Flatpak: osierocone ID w `~/.var/app` z refem w instalacji systemowej nie jest sierotą, a
  niepoprawne ID jest ignorowane.
- Transakcja zmiany nazwy: `foo-git` usunięty i `foo-bin` zainstalowany w tej samej minucie, więc `foo`
  jest zainstalowany.

---

## 7. Ograniczenia i niezweryfikowane założenia

- Liczby precyzji pochodzą z **jednej maszyny**. Sprawdzono je ręcznie po nazwie, osi czasu i
  układzie katalogów, a nie po kodzie źródłowym aplikacji.
- Pakiety AUR nie mają danych w bazie plików. Ich pozostałości opierają się tylko na nazwie i btime. Rekordy XU
  pomagają tylko dla aplikacji usuniętych przez XU.
- Pakiety usunięte przed początkiem `pacman.log` (tu 2026-06-24) nie dają dowodów usunięcia.
  Ich katalogi celowo zostają nieznane.
- btime wymaga systemu plików obsługującego `statx` `STATX_BTIME` (btrfs, ext4 i xfs to mają).
  Kopiowanie/przywracanie go resetuje, i to obsługuje wykrywanie kohort.
- `pacman -Fl` zależy od zsynchronizowanej bazy plików (`pacman -Fy`, root). Tu
  `cachyos-core-v3.files` pochodził z 2026-10-07. Nieaktualna baza tylko zmniejsza czułość.
- Zestawu reguł xdg-unused-data nie oceniono na tym HOME. Dodałby zasięg dla
  znanych aplikacji bez historii pacmana (i wymaga wpisu o autorstwie MIT).
- Twierdzenie, że `com.epicgames.eglv2` pochodzi z wcześniejszej instalacji Epic, wywnioskowano z btime
  10-15 < instalacja w pacmanie 11-12. Nieudowodnione.
- Nie obejmuje (osobne punkty roadmapy): pozostałości w `~/.cache/*` (mniejsza wartość, do odtworzenia),
  martwych plików `.desktop` i symlinków (v0.0.2), osieroconych `compatdata` Steama (katalog §1.3) ani
  czyszczenia tokenów w `mimeapps.list` (`integration.rs` z XU).
