# XCleaner — roadmapa

Zadania są ułożone od najmniejszych do największych. Każdy etap kończy się
działającym buildem, który można przetestować na Decku.

Research: [docs/research/](research/). HOME-JUNK-CATALOG zawiera pomiary z tej
maszyny, a UPSTREAM-CLEANERS opisuje BleachBit, czkawka, kondo, CCleaner i inne.

## v0.0.1 — szkielet (zrobione)
- [x] Workspace Rust: `xcleaner-core` (bez GTK) + `xcleaner` (GTK4/libadwaita).
- [x] Bezpieczne usuwanie na deskryptorach:
  - openat2 BENEATH/NO_SYMLINKS/NO_XDEV;
  - bez podążania za symlinkami, bez przechodzenia na inne systemy plików;
  - pomija pliki innych właścicieli.
- [x] 16 reguł: miniatury, fontconfig, GStreamer, shadery Mesa, cache przeglądarek,
  yay/paru (bez `vcs.json`), cargo, npm, pnpm, pip/uv, go, kosz, recently-used.
- [x] Automatyczne wykrywanie aplikacji Electron/Chromium w `~/.config` (tylko katalogi cache).
- [x] Katalogi w `~/.cache` z `CACHEDIR.TAG`.
- [x] Blokada, gdy program działa: /proc oraz SingletonLock z PID.
- [x] GUI: skan → lista z kategoriami → jedno potwierdzenie → czyszczenie.
- [x] `xcleaner --scan`: raport w terminalu, nic nie usuwa.

## v0.0.2 — małe reguły (zrobione)
- [x] Steam:
  - shadery per gra, z nazwą z appmanifest (CS2 10,9 GB, Deadlock 6,5 GB);
  - osierocone shadery po odinstalowanych grach (zaznaczone);
  - logi i zrzuty, cache sklepu.
- [x] Cache launcherów: `PrismLauncher/cache` (2,7 GB), `bedrock-on-linux/cache` (843 MB).
- [x] Discord: stare `app-1.0.*` i `1.0.*` po auto-aktualizacji (zostaje najnowsza).
- [x] Logi Electron (`logs/`), Crashpad, `Service Worker/CacheStorage`.
- [x] Reszta `~/.cache`: każda pozycja osobno i zaznaczona.
  - Katalogi z muzyką lub filmami są automatycznie oznaczane „do przejrzenia” (`music-opus`).
- [x] Twarda lista „nigdy” (`protect.rs`), sprawdzana przy wykrywaniu i przed usunięciem,
  pilnowana testami (`.ssh`, keyringi, Steam `userdata`/`compatdata`, `.local/bin`,
  katalogi najwyższego poziomu HOME).
- [x] Martwe skróty `.desktop`: brak programu, brak Flatpaka, brak prefiksu Wine,
  zepsuty link. Na tej maszynie: 7, m.in. osu-wine, BeatSaberModManager, stare XUninstaller.
- [x] `xcleaner --scan --details`: ścieżki i uzasadnienia.

## v0.0.3 — pozostałości po odinstalowanych aplikacjach (zrobione)
- [x] `leftovers.rs`: `~/.config`, `~/.local/share`, `~/.local/state`, `~/.var/app`.
  Pozycja trafia na listę tylko wtedy, gdy zgadzają się 3 fakty:
  - nazwa odpowiada pakietowi usuniętemu według pacman.log;
  - nic zainstalowanego nie używa tej nazwy;
  - katalog nie powstał po usunięciu pakietu.
- [x] Dowody: baza pacmana (nazwy, provides, binarki, .desktop, /opt, /etc/xdg), PATH,
  pliki .desktop (z pominięciem martwych), Flatpak, procesy.
- [x] Poziomy: „pewne” (katalog utworzony 0–30 min po instalacji) i „prawie pewne”.
  Nigdy nie są zaznaczane z góry. Ostrzeżenie, gdy w środku są dane osobiste.
- [x] Lista wspólnych nazw (gtk, dconf, pulse, systemd…) zawsze wygrywa. Kopie psd `*-back-ovfs.base`.
- [x] Na tej maszynie: 20 pozycji, ok. 9,8 GB. Zgodne z ręczną analizą, a fałszywy
  alarm „Discord Key Storage” jest odrzucany.
- [x] Aliasy z `pacman -Fl <usunięte>` (binarki, .desktop, /opt, /etc/xdg), z cache
  w `~/.cache/xcleaner` (pierwszy skan 3,9 s, kolejne 0,5 s). Powrót: `--no-files-db`.
  - Na tej maszynie dało 0 nowych trafień, ale łapie przemianowania na innych systemach.
- [x] Decyzja właściciela: resztki bez danych osobistych są zaznaczone z góry („niskie ryzyko”).
  - Znaczniki danych: profile przeglądarek, zapisy, zrzuty ekranu, instancje gier,
    pliki .exe/.sav, historia rozmów AI.
  - Na tej maszynie odznaczone: BSManager (gra i mapy), shadPS4, zed, profile przeglądarek.
- [x] Twarde linki: rozmiar współdzielony jest raportowany (realnie zwolni się mniej).
- [ ] Później:
  - dopasowania po pierwszym członie nazwy (evolution-data-server → evolution);
  - rekordy XUninstaller, dotdiry `~/.<nazwa>`.

## v0.0.4 — średnie (zrobione)
- [x] Okno szczegółów (⋮): każda ścieżka z rozmiarem, otwieranie w menedżerze plików,
  przycisk „Nie ruszaj tego — ukryj na stałe”.
- [x] Lista wyjątków `~/.config/xcleaner/keep.txt` (edytowalna ręcznie), przycisk
  „Przywróć ukryte (N)”.
- [x] Dziennik „Zdarzenia”: skany, czyszczenia, błędy, kopiowanie i zapis .md
  (ścieżka HOME zamieniana na `~`). Trwała historia w `~/.local/state/xcleaner/history.log`.
- [x] Flatpak: `~/.var/app/<id>/cache` zainstalowanych aplikacji.
- [x] Resztki „możliwe” (pierwszy człon nazwy); AppImage’y liczone jako zainstalowane.

## v0.0.5 — duże (zrobione)
- [x] Artefakty budowania projektów (`artifacts.rs`), w stylu npkill:
  - katalog jest artefaktem tylko obok pliku projektu (Cargo.toml→target,
    package.json→node_modules, CMake `build` tylko z CMakeCache.txt, ok. 30 typów);
  - szukanie w całym HOME bez ukrytych katalogów, symlinków i innych systemów plików;
  - zaznaczone, jeśli projekt nie był budowany od 30 dni. Na tej maszynie: 12 GB w 4 projektach.
- [x] „Co zajmuje miejsce” (ikona dysku, `xcleaner --usage [katalog]`):
  - wchodzenie w głąb, paski udziału, otwieranie w menedżerze plików;
  - całe HOME (442 GB) w 3,0 s, `du` ok. 3,7 s.
- [x] Poprawka: pomiar zawieszał się na kolejkach FIFO (`~/.steam/steam.pipe`).
- [x] Poprawka: `| head` w CLI nie robi już panic ani coredumpa.
- [ ] Przeniesione: reguły w osadzonym TOML. Wymaga nowej zależności (`toml`);
  zrobić razem z regułami użytkownika.

## v0.1.0 — wydanie
- [ ] AppImage (linuxdeploy + plugin GTK, jak w XUninstaller).
- [ ] Ikona, plik .desktop, README PL/EN, testy na świeżym Arch/CachyOS.

## Faza 2 — system (`/`, przez sudo)
- [ ] Broker roota z XUninstaller: GUI nigdy nie działa jako root; dozwolona jest tylko
  zatwierdzona lista komend.
- [ ] Cache pacmana przez `paccache -rk2`. Nie `-rk0`, bo to blokuje downgrade.
- [ ] journald: `--vacuum-time`, nigdy „usuń wszystko”. Tylko zrotowane logi w `/var/log`.
- [ ] Coredumpy (`/var/lib/systemd/coredump`, research: 1–4 GB, głównie CS2).
- [ ] Moduły po usuniętych kernelach, `/var/tmp`, nieużywane runtime’y Flatpak
  (`flatpak uninstall --unused`).

## Nigdy
- Wymazywanie RAM, swapu i wolnego miejsca, „shred”.
- Menedżer autostartu.
- Odinstalowywanie aplikacji: to robi XUninstaller.
