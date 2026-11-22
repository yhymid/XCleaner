# Changelog

## 0.1.0 - First Sweep

Pierwsze wydanie. XCleaner szuka śmieci w katalogu domowym i w systemie,
pokazuje dowód dla każdej pozycji i usuwa tylko to, co zaznaczysz.

### Katalog domowy
- Pamięć podręczna `~/.cache`. Każdy program jest osobną pozycją. Katalogi z muzyką
  lub filmami są rozpoznawane i nigdy nie są zaznaczane z góry.
- Przeglądarki (Chrome, Chromium, Firefox, Zen) i aplikacje Electron (Discord, VSCodium…):
  tylko cache. Logowanie, wiadomości i ustawienia zostają.
- Steam: shadery każdej gry osobno z nazwą gry, osierocone shadery po odinstalowanych
  grach, logi i zrzuty, cache sklepu. Launchery: Prism, Bedrock.
- Programowanie: yay/paru (bez `vcs.json`), cargo, npm, pnpm, pip/uv, go,
  artefakty budowania projektów (`target/`, `node_modules`… — ok. 30 typów projektów).
- Stare wersje Discorda po auto-aktualizacji, martwe skróty w menu, cache Flatpaka.
- Kosz i lista ostatnio otwieranych plików (zawsze do przejrzenia).

### Pozostałości po odinstalowanych aplikacjach
- Katalogi w `~/.config`, `~/.local/share`, `~/.local/state`, `~/.var/app` po pakietach usuniętych
  według `pacman.log`. Muszą zgadzać się 3 fakty: nazwa, brak zainstalowanego właściciela
  i data utworzenia przed usunięciem pakietu.
- Poziomy „pewne / prawie pewne / możliwe” z uzasadnieniem.
- Ostrzeżenie, gdy w środku są Twoje dane (profile przeglądarek, zapisy gier, zrzuty ekranu).

### System (hasło sudo w terminalu)
- Zrzuty awarii (coredumpy), moduły po usuniętych kernelach (nigdy działającego),
  cache pacmana: `paccache -rk2`, a pełne czyszczenie tylko na Twoje wyraźne zaznaczenie.

### Okno
- Kategorie, rozmiary, szczegóły każdej pozycji, otwieranie w menedżerze plików.
- „Nie ruszaj tego” (trwała lista wyjątków), dziennik „Zdarzenia” (kopiuj / zapisz .md),
  historia czyszczeń w `~/.local/state/xcleaner/history.log`.
- „Co zajmuje miejsce” — przeglądanie rozmiarów katalogów jak w ncdu.
- Tryb terminalowy: `xcleaner --scan [--details]`, `xcleaner --usage [katalog]`.

### Bezpieczeństwo
- Usuwanie przez deskryptory (bez podążania za symlinkami, bez wychodzenia poza katalog,
  bez innych systemów plików, bez plików innych użytkowników).
- Twarda lista „nigdy”: klucze, keyringi, konfiguracja pulpitu, dane gier Steam, `~/.local/bin`…
- Pozycje działających programów są blokowane.
- Usunięcie jest trwałe — program mówi to przed każdym czyszczeniem.
