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
- [ ] Później:
  - dopasowania po pierwszym członie nazwy (evolution-data-server → evolution);
  - aliasy z `pacman -Fl`, rekordy XUninstaller, dotdiry `~/.<nazwa>`;
  - decyzja właściciela o zaznaczaniu pozycji „pewnych” bez danych osobistych.

## v0.0.4 — średnie
- [ ] Okno szczegółów pozycji: lista ścieżek z rozmiarami, otwórz w menedżerze plików.
- [ ] Dziennik „Zdarzenia”: co i ile usunięto, błędy, eksport .md (jak w XU).
- [ ] Flatpak: `~/.var/app/*/cache`.
- [ ] Lista wyjątków użytkownika („nie ruszaj tego”).

## v0.0.5 — duże
- [ ] Artefakty budowania projektów w stylu kondo/npkill:
  - `target/` (CACHEDIR.TAG), `node_modules`, `.venv`, `build/`;
  - filtr wieku („nieużywane od 30 dni”);
  - wyszukiwanie w `~/Projects`, `~/Pobrane` i katalogach wskazanych przez użytkownika.
- [ ] Reguły w osadzonym TOML (`include_str!`) zamiast w kodzie Rust.
- [ ] Widok „co zajmuje miejsce” (jak ncdu) dla całego `$HOME`: równoległy skan.

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
