# XCleaner

[English](README.md) | **Polski**

Szybki i precyzyjny program do czyszczenia dysku dla Linuksa, napisany w Rust + GTK4/libadwaita.

XCleaner znajduje pamięć podręczną, stare logi, artefakty budowania i pozostałości po
odinstalowanych aplikacjach, pokazuje, **dlaczego** każda pozycja jest śmieciem, i usuwa tylko
to, co zaznaczysz. Twoja konfiguracja i dane osobiste nigdy nie są zaznaczane za Ciebie.

[![Najnowsze wydanie](https://img.shields.io/github/v/release/yhymid/XCleaner)](https://github.com/yhymid/XCleaner/releases/latest)
![Licencja: MIT](https://img.shields.io/badge/license-MIT-blue)

> Interfejs po angielsku i po polsku — przełącznik **EN / PL** jest w pasku okna.
> Usunięcie jest trwałe — nie ma kwarantanny, a program mówi to przed każdym czyszczeniem.

## Pobieranie

Pliki są w [najnowszym wydaniu](https://github.com/yhymid/XCleaner/releases/latest):

| Plik | Dla |
|---|---|
| `XCleaner-<wersja>-x86_64.AppImage` | każda dystrybucja z glibc 2.39+, bez instalacji |
| `xcleaner_<wersja>-1_amd64.deb` | Ubuntu 24.04+, Debian 13, Linux Mint 22, Pop!_OS |
| `xcleaner-<wersja>-1.x86_64.rpm` | Fedora 41+, openSUSE Tumbleweed |
| `xcleaner-<wersja>-1-x86_64.pkg.tar.zst` | Arch, CachyOS, EndeavourOS, Manjaro |
| `SHA256SUMS` | sumy kontrolne wszystkich plików |

```sh
sha256sum -c --ignore-missing SHA256SUMS

sudo apt install ./xcleaner_<wersja>-1_amd64.deb        # Debian / Ubuntu / Mint
sudo dnf install ./xcleaner-<wersja>-1.x86_64.rpm       # Fedora
sudo zypper install --allow-unsigned-rpm ./xcleaner-<wersja>-1.x86_64.rpm   # openSUSE
sudo pacman -U xcleaner-<wersja>-1-x86_64.pkg.tar.zst   # Arch / CachyOS
chmod +x XCleaner-<wersja>-x86_64.AppImage              # AppImage
```

Debian 12 i Ubuntu 22.04 są za stare (glibc i GTK).

## Użycie

```sh
xcleaner                      # okno (z menu: XCleaner)
xcleaner --scan [--details]   # raport w terminalu, nic nie usuwa
xcleaner --usage [katalog]    # co zajmuje miejsce w HOME, jak ncdu (tylko odczyt)
xcleaner --lang pl --scan     # język jednego uruchomienia: en | pl
```

Uruchamiaj **bez sudo**. Czyszczenie systemu otwiera terminal i tam pyta o hasło
(`sudo`, `doas` albo `su`); nie ma agenta polkit ani usługi w tle.

## Co czyści

**Katalog domowy**
- `~/.cache`, każdy program jako osobna pozycja; katalogi z Twoimi danymi (muzyka, stan
  sesji, historia schowka…) nigdy nie są zaznaczane z góry.
- Przeglądarki i aplikacje Electron: tylko cache — logowanie, wiadomości i ustawienia zostają.
- Steam (zwykły, deb, Flatpak, Snap): shadery każdej gry, osierocone shadery
  odinstalowanych gier, logi, zrzuty. Dane gier są chronione.
- Cache programistyczne (cargo, npm, pnpm, pip/uv, go, yay/paru) i artefakty budowania
  (`target/`, `node_modules`… ok. 30 typów projektów).
- Cache aplikacji Flatpak i Snap, stare rewizje Snap, kosz, lista ostatnich plików.

**Pozostałości po odinstalowanych aplikacjach** w `~/.config`, `~/.local/share`, `~/.local/state`
i `~/.var/app`, według historii pakietów pacmana (Arch) i dpkg (Debian/Ubuntu).
Muszą zgadzać się 3 fakty: nazwa, brak zainstalowanego właściciela i katalog istniejący
przed usunięciem pakietu. Każda pozycja ma poziom *pewne / prawie pewne / możliwe*,
z ostrzeżeniem, jeśli w środku są dane osobiste.

**System** (hasło w terminalu)
- Cache paczek przez własną komendę menedżera: `paccache`, `apt-get autoclean`,
  `dnf clean packages`, `zypper clean`, `xbps-remove -O`, `apk cache clean`, `eclean`.
- Stare wpisy dziennika systemd, zrotowane logi w `/var/log`, coredumpy, raporty awarii
  apport/ABRT.
- Nieużywane runtime’y Flatpak, wyłączone rewizje Snap, `rpm-ostree cleanup`.
- Moduły po usuniętych kernelach i stare kernele (tylko menedżerem pakietów, nigdy działający).
- Nic nie rusza, gdy działa menedżer pakietów; na systemach niezmiennych (SteamOS,
  Silverblue/Bazzite, NixOS, MicroOS) obraz systemu zostaje nietknięty.

## Bezpieczeństwo

- Usuwanie przez deskryptory plików (`openat2` z `RESOLVE_BENEATH`): bez podążania za
  symlinkami, bez wychodzenia poza katalog, bez innych systemów plików, bez plików innych użytkowników.
- Twarda lista „nigdy”: klucze, keyringi, konfiguracja pulpitu, dane gier Steam, `~/.local/bin`…
- Pozycje działających programów są blokowane.
- Okno nigdy nie działa jako root; tylko wybrane zadania systemowe, w terminalu.
- Historia każdego czyszczenia: `~/.local/state/xcleaner/history.log`.

## Budowanie

Potrzebne: Rust 1.85+ oraz pliki deweloperskie GTK 4.10+ i libadwaita 1.5+.

```sh
cargo build --release
./target/release/xcleaner
```

Pliki wydania (Podman albo Docker):

```sh
./packaging/build-in-container.sh   # AppImage, budowany na Ubuntu 24.04 (glibc 2.39)
./packaging/build-native.sh         # .deb (Ubuntu 24.04) i .rpm (Fedora 41)
./packaging/build-pkg.sh            # paczka Archa (makepkg)
```

## Układ projektu

- `crates/xcleaner-core` – reguły, równoległy skaner, bezpieczne usuwanie, zadania systemowe (bez GTK).
- `crates/xcleaner` – okno GTK4/libadwaita i tryb terminalowy.
- `docs/ROADMAP.pl.md` – plan; `docs/research/` – raporty badawcze, na których opierają się reguły.
- `CHANGELOG.pl.md` – zmiany w każdym wydaniu.
- Każdy dokument ma wersję angielską (`X.md`) i polską (`X.pl.md`).

## Licencja

MIT — zobacz [LICENSE](LICENSE).
