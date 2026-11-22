# Changelog

## Unreleased

### Język / Language
- Program po angielsku (domyślnie) i po polsku. Przełącznik **EN / PL** jest w pasku okna;
  wybór zapisuje się w `~/.config/xcleaner/settings.txt`. W terminalu: `--lang en|pl`.
- Terminal z hasłem i pomocnik roota mówią w języku okna.
- Rozmiary po angielsku z kropką (`1.5 GiB`), po polsku z przecinkiem (`1,5 GiB`).

## 0.2.0 - Open Road

XCleaner wychodzi poza Archa: działa na Ubuntu, Debianie, Mincie, Fedorze, openSUSE
i pochodnych. Każdy nowy element ma źródło w kodzie lub dokumentacji danego systemu,
a podejrzane rzeczy są tylko do przejrzenia.

### Na każdej dystrybucji
- Paczki `.deb` (Ubuntu 24.04+, Debian 13, Mint 22, Pop!_OS) i `.rpm` (Fedora 41+,
  openSUSE Tumbleweed) obok AppImage i paczki pacmana. AppImage wymaga teraz glibc 2.39
  zamiast 2.44, więc działa także poza Archem.
- Pozostałości po odinstalowanych aplikacjach na Debianie i Ubuntu (`dpkg.log`, także
  stare spakowane logi, i pakiety „rc”). Na Fedorze i openSUSE historia pakietów nie jest
  czytelna bez roota, więc program niczego tam nie zgaduje.
- Terminal do hasła dobierany według pulpitu (Konsole, Ptyxis, GNOME Terminal, xfce4-terminal…),
  z poprawnymi argumentami każdego. Program wie, kiedy okno zamknięto albo hasło było złe.
- Hasło przez `sudo`, `doas` albo `su` (Debian z hasłem roota). Nadal bez polkit.

### System
- Cache paczek przez ich własne komendy: `apt-get autoclean` (pełne `apt-get clean` tylko
  do przejrzenia), `dnf clean packages`, `zypper clean`, `xbps-remove -O`, `apk cache clean`,
  `eclean distfiles`.
- Stare wpisy dziennika systemd (`journalctl --vacuum-time=4weeks`), zrotowane logi
  w `/var/log` (nigdy aktywne, audit ani historia logowań).
- Nieużywane runtime’y Flatpak, wyłączone rewizje Snap, `rpm-ostree cleanup`.
- Raporty awarii apport (Ubuntu) i ABRT (Fedora).
- Stare kernele — wyłącznie menedżerem pakietów, tylko do przejrzenia; działający zostaje.
- Moduły po usuniętych kernelach także na Debianie i Fedorze.
- Na systemach niezmiennych (SteamOS, Silverblue/Bazzite, NixOS, MicroOS) program nie
  dotyka obrazu systemu.
- Pacman: puste i porzucone katalogi `download-*` (na nich wywalał się `pacman -Sc`),
  bazy repozytoriów usuniętych z `pacman.conf`.
- Każde zadanie czeka, gdy działa menedżer pakietów (apt, dnf, zypper, xbps, apk, pacman).

### Katalog domowy
- Steam z pakietu deb, Flatpaka i Snapa: ochrona danych gier i shadery jak w zwykłej instalacji.
- Cache aplikacji Snap i dane starych rewizji Snap; Flatpak i Snap są blokowane, gdy aplikacja działa.
- Kosz: także `directorysizes`.

### Bezpieczeństwo
- Nigdy z góry: katalogi w `~/.cache`, które trzymają Twoje dane (sesje XFCE, historia schowka,
  pliki z Bluetooth, słowniki ibus, Local History JetBrains, kopie poczty Evolution, indeks tracker3).
- Nowe blokady: gwiazdki Nautilusa, metadane GVfs, Akonadi, ulubione KDE, dodatki Cinnamona,
  poczta lokalna Evolution, `data` i `config` aplikacji Flatpak, profile Firefoksa w Snapie.
- Programy uruchamiane z AppImage (terminal, menedżer plików, pacman) nie dostają jego
  bibliotek ani zmiennych.

### Poprawki
- Historia czyszczeń: każde zadanie systemowe ma osobną linię z prawdziwym zwolnionym miejscem.

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

### Wydajność
- Skan wszystkiego ok. 1 s; „Co zajmuje miejsce” dla całego HOME (440 GB) ok. 3 s.
- Duże katalogi (np. shadery gier) są usuwane wielowątkowo.
