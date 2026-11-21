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

## v0.0.2 — małe reguły (z researchu)
- [ ] Steam: `logs`, `dumps`, `appcache/httpcache`, `steamapps/shadercache/<id>`
  (niskie ryzyko, guard: steam). Według researchu CS2 ma tu 11 GB, a Deadlock 6.6 GB.
- [ ] Cache launcherów: `PrismLauncher/cache` (2.8 GB), `bedrock-on-linux/cache`.
- [ ] Discord: stare `app-1.0.*` po aktualizacji i `discord_asset_cache`.
- [ ] Logi Electron (`logs/`) i Crashpad: osobna pozycja „logi”.
- [ ] Twarda lista „nigdy”, wymuszona testem: `.ssh`, `.gnupg`, keyringi,
  katalogi XDG użytkownika, Steam `userdata`/`config`, `.local/bin`, `.claude`.
- [ ] Osierocone pliki `.desktop` i zepsute symlinki w `~/.local/share/applications`
  i `~/.local/bin`.

## v0.0.3 — średnie
- [ ] Okno szczegółów pozycji: lista ścieżek z rozmiarami, otwórz w menedżerze plików.
- [ ] Dziennik „Zdarzenia”: co i ile usunięto, błędy, eksport .md (jak w XU).
- [ ] Flatpak: `~/.var/app/*/cache`.
- [ ] Pozostałości po odinstalowanych aplikacjach (profil `discord`, `chromium`,
  `*-back-ovfs.base`): pozycje „do przejrzenia”, nigdy nie zaznaczane z góry.
- [ ] Lista wyjątków użytkownika („nie ruszaj tego”).

## v0.0.4 — duże
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
