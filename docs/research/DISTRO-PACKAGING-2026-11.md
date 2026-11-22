# XCleaner: dystrybucje i pakowanie (research na v0.2.0, 2026-11-22)

Oznaczenia: **[zmierzone]** to pomiar na tej maszynie, **[źródło]** pochodzi z pierwotnego źródła z listy na końcu, a **[niezweryfikowane]** to wiedza ogólna, której nie potwierdziłem.

## 0. Najważniejsze
- Obecny `XCleaner-0.1.0-x86_64.AppImage` wymaga **glibc ≥ 2.44** **[zmierzone]**. Działa więc tylko na Arch, CachyOS, Manjaro, EndeavourOS, Tumbleweed i Fedora Rawhide. Nie uruchomi się na Ubuntu, Debianie, Mincie, Fedorze ≤ 44 ani Leap.
- Sama binarka `xcleaner` wymaga tylko **glibc 2.34** (twarde wersje). Wysoką wersję wymuszają biblioteki zbudowane na CachyOS i dołączone przez wtyczkę GTK: `libsqlite3` (2.44), `libglib`, `libgtk-4`, `libgnutls` i `liborc` (2.43) oraz ~35 innych (2.38) **[zmierzone]**.
- Rekomendacja na v0.2.0: AppImage budowany w kontenerze **Ubuntu 24.04** (glibc 2.39, GTK 4.14, libadwaita 1.5) z cechami **`gtk4/v4_10` + `adw/v1_5`**. Trzeba tylko zamienić `adw::Spinner` na `gtk::Spinner`, co sprawdziłem kompilacją **[zmierzone]**. Do tego `.deb` i `.rpm` z tą samą binarką oraz paczka pacmana. Flatpak nie, Snap później.

## 1. glibc: pomiar i dystrybucje

### Pomiar (AppImage 0.1.0, 120 plików ELF)
Polecenia:
```sh
./XCleaner-0.1.0-x86_64.AppImage --appimage-extract
for f in $(find squashfs-root -type f -name '*.so*') squashfs-root/usr/bin/xcleaner; do
  readelf -V "$f" | grep -oE 'Name: GLIBC_[0-9.]+  Flags: none'; done | sort -Vu | tail -3
objdump -T squashfs-root/usr/bin/xcleaner | grep -E 'GLIBC_2\.(39|4)'
```
| Plik | Twarde minimum glibc | Symbole |
|---|---|---|
| `usr/lib/libsqlite3.so.0` (zależność tinysparql/GTK) | **2.44** | `cosh`, `sinh` |
| `libglib-2.0.so.0` | 2.43 | `free_sized`, `free_aligned_sized` |
| `libgtk-4.so.1`, `liborc-0.4.so.0` | 2.43 | `acosf`, `atan2f`, `sqrtf` |
| `libgnutls.so.30` | 2.43 | `__memset_explicit_chk` |
| ~35 bibliotek (adwaita, gio, pango, cairo, dbus, xkbcommon…) | 2.38 | C23 `__isoc23_*` |
| `usr/bin/xcleaner` | **2.34** | `pidfd_spawnp` i `pidfd_getpid` (2.39) są **słabe** (`w`, `Flags: WEAK`), więc ich brak nie blokuje startu |

Inne wymagania hosta (biblioteki spoza paczki, linuxdeploy excludelist): `libEGL`, `libGLX`, `libgbm`, `libdrm`, `libwayland-client`, `libX11`, `libxcb`, `libfontconfig`, `libfreetype`, `libharfbuzz`, `libfribidi`, `libgmp`, `libexpat`, `libz`, `libstdc++`, `libgcc_s`. ICU wymaga `GLIBCXX_3.4.30` (GCC 12). Loader `libopenraw_pixbuf` wymaga `GLIBCXX_3.4.32` (GCC 13), ale ładuje się tylko dla plików RAW. Mesa i GL pochodzą z hosta, i tak ma zostać. Rozpakowany AppImage ma 144 MB, plik 48 MB.

Dodatkowe ryzyko **[zmierzone/niezweryfikowane]**: dołączony `gdk-pixbuf 2.44` linkuje `libglycin-2`. Glycin uruchamia zewnętrzne loadery z hosta (`glycin-loaders`, przez `bwrap`). Na dystrybucjach bez glycin 2 (Ubuntu 24.04, Debian) PNG/JPEG przez gdk-pixbuf mogą nie działać. Build na Ubuntu 24.04 (gdk-pixbuf 2.42, bez glycin) usuwa ten problem.

### glibc, GTK4 i libadwaita w dystrybucjach (Repology API, 2026-11-22)
| Dystrybucja | glibc | GTK4 | libadwaita | AppImage 0.1.0 | AppImage z Ubuntu 24.04 | Natywnie (v4_10/v1_5) |
|---|---|---|---|---|---|---|
| Debian 12 / MX 23 | 2.36 | 4.8.3 | 1.2.2 | nie | nie | nie |
| Debian 13 / MX 25* | 2.41 | 4.18.6 | 1.7.6 | nie | tak | tak |
| Ubuntu 22.04 / Zorin 17* | 2.35 | 4.6.9 | 1.1.7 | nie | nie | nie |
| Ubuntu 24.04, Mint 22.x*, Pop!_OS 24.04*, elementary 8*, Zorin 18* | 2.39 | 4.14.5 | 1.5.0 | nie | **tak** | tak |
| Ubuntu 25.04 / 25.10 | 2.41 / 2.42 | 4.18 / 4.20 | 1.7 / 1.8 | nie | tak | tak |
| Ubuntu 26.04 LTS | 2.43 | 4.22.4 | 1.9.1 | nie | tak | tak |
| Fedora 41 / 42 / 43 / 44 | 2.40 / 2.41 / 2.42 / 2.43 | 4.16 / 4.18 / 4.20 / 4.22 | 1.6 / 1.7 / 1.8 / 1.9 | nie | tak | tak |
| Bazzite (Fedora 43)* | 2.42 | 4.20 | 1.8 | nie | tak | rpm-ostree, raczej AppImage |
| openSUSE Leap 15.6 | 2.38 | 4.12.4 | 1.4.2 | nie | nie | nie (wymaga adw 1.5) |
| openSUSE Leap 16.0 | 2.40 [niezweryfikowane] | 4.18 (GNOME 48)* | 1.7* | nie | tak* | tak* |
| Tumbleweed | 2.44 | 4.24.1 | 1.10.0 | tak | tak | tak |
| Arch / Manjaro / EndeavourOS | 2.44 | 4.24.1 (Manjaro 4.22.5) | 1.10 (Manjaro 1.9.4) | tak | tak | tak (pkg) |
| SteamOS 3.7+ | ~2.41 [niezweryfikowane] | z obrazu Arch | — | ? | tak* | nie (read-only `/usr`) |

\* = wersja przyjęta po bazie dystrybucji (Mint/Pop/elementary/Zorin 18 = Ubuntu 24.04, MX 25 = Debian 13, Leap 16 = GNOME 48). Tych wartości Repology nie podaje bezpośrednio.

### Jak budować przenośnie
| Metoda | Ocena |
|---|---|
| **Kontener ze starą bazą** (`ubuntu:24.04` w podman/docker, linuxdeploy + wtyczka GTK) | **Zalecane.** glibc i GTK z jednej, zgodnej bazy. Minimum to glibc 2.39, co obejmuje wszystkie wymienione dystrybucje poza Debianem 12, Ubuntu 22.04/Zorin 17 i Leap 15.6. Wymaga zejścia do `v4_10`/`v1_5`. |
| Kontener `debian:13` | GTK 4.18 i adw 1.7, kod bez zmian, ale minimum glibc 2.41 odcina Ubuntu 24.04, Minta 22 i Pop. Gorzej. |
| `cargo-zigbuild --target x86_64-unknown-linux-gnu.2.28` | Obniża wymagania tylko **naszej binarki**. Ta i tak potrzebuje 2.34, a blokują dołączone GTK i glib. Do AppImage bezużyteczne. |
| static musl | GTK4, GL i Mesa nie dadzą się sensownie zlinkować statycznie. Odpada. |
| sharun / „Anylinux AppImages” (pkgforge-dev) | Dołączają wszystko łącznie z libc i nie zależą od libc hosta, więc ruszą też na Debianie 12 i 22.04. Kosztem są większy plik i własny Mesa. Kandydat na v0.3, jeśli Debian 12 i 22.04 będą potrzebne. |
| Budowa GTK 4.14 i adw 1.5 ze źródeł na Debianie 12 | Duży nakład (meson, cały stos). Nie warto. |

Rust i GTK4 w praktyce: większość aplikacji gtk-rs i libadwaita (Fractal, Loupe, Mission Center) publikuje głównie Flatpak na Flathub. Projekty AppImage budują w kontenerze najstarszej obsługiwanej bazy albo w stylu sharun **[niezweryfikowane jako statystyka]**.

## 2. Funkcje v1_6 i v4_12 w kodzie
Test na kopii repo w katalogu tymczasowym (repo nietknięte), `cargo check --offline` **[zmierzone]**:
| Cechy | Wynik | Co blokuje |
|---|---|---|
| `v4_10` + `v1_5` | 1 błąd | `adw::Spinner` (1.6), ui.rs:165 |
| `v4_10` + `v1_5` + `gtk::Spinner::builder().spinning(true)` | **kompiluje się** | — |
| `v4_10` + `v1_4` | 4 błędy | `adw::AlertDialog` (1.5) ui.rs:380, `adw::Dialog` (1.5) ui.rs:527/601/652 |
| `v4_8` + `v1_5` | 3 błędy | `gtk::FileLauncher` (4.10) ui.rs:510/770, `gtk::FileDialog` (4.10) ui.rs:614 |
| `v4_6` + `v1_2` (Debian 12) | dodatkowo `adw::ToolbarView` (1.4) | ToolbarView, Dialog, AlertDialog, FileDialog, FileLauncher |

Wnioski:
- `v4_12` nie jest do niczego potrzebne, wystarczy `v4_10`.
- `v1_6` służy tylko do `adw::Spinner`. Zamiana na `gtk::Spinner` daje Ubuntu 24.04+.
- Debian 12 i 22.04 wymagałyby zastąpienia ToolbarView (Box + HeaderBar), adw::Dialog (adw::Window) i AlertDialog (MessageDialog 1.2) oraz cfg-gatingu FileDialog/FileLauncher (stare `gtk::FileChooserNative`, `gio::AppInfo::launch_default_for_uri`). Nie warto: te systemy mają GTK 4.6–4.8.
- Gating przez `cfg(feature)` działa tylko w czasie kompilacji. `-sys` sprawdza wersję przez pkg-config: `gtk4-sys` (bazowo 4.0.0, `v4_10` → 4.10) i `libadwaita-sys` (bazowo 1, `v1_5` → 1.5). Do wyboru w runtime służy `adw::minor_version()`, ale same symbole i tak muszą istnieć przy linkowaniu, więc to wymaga jednej binarki na poziom.
- Brak symbolu (np. `adw_spinner_new` na adw 1.5) kończy start błędem loadera **[niezweryfikowane: tryb wiązania, ale efekt i tak fatalny]**.

## 3. Runtime AppImage, FUSE i AppImageLauncher
- Obecny AppImage **już** używa statycznego `type2-runtime` (static-pie, 0 NEEDED, commit `fedfad5`) **[zmierzone]**. Dodaje go wtyczka `appimage` w linuxdeploy (continuous). Nie trzeba `libfuse2` **[źródło: README type2-runtime]**.
- Runtime szuka na `$PATH` dowolnego pliku `fusermount*` z setuid root, więc działa też z `fusermount3`. Można go nadpisać przez `FUSERMOUNT_PROG` (`runtime.c` `find_fusermount`) **[źródło]**. Ubuntu 22.04+ ma domyślnie `fuse3` z `fusermount3` **[niezweryfikowane: domyślny seed]**, więc stare porady „zainstaluj libfuse2” nas nie dotyczą.
- Fallback bez FUSE (kontenery, brak modułu): `./XCleaner.AppImage --appimage-extract-and-run` albo `APPIMAGE_EXTRACT_AND_RUN=1`. Warto to opisać w sekcji Install.
- Uwaga: `sudo` resetuje env, więc krok root uruchamia `$APPIMAGE` normalnie przez FUSE. Jeśli użytkownik nie ma FUSE, krok root się nie uda. Sugestia: gdy w procesie GUI jest `APPIMAGE_EXTRACT_AND_RUN`, przekazać go w prefiksie `env` w `sudo_args`.
- Przy appimagetool: `appimagetool --runtime-file runtime-x86_64 AppDir` (z releases type2-runtime) **[niezweryfikowane: nazwa flagi]**. Z linuxdeploy nic nie trzeba, bo `--output appimage` już daje type2.
- AppImageLauncher: przechwytuje uruchomienie przez binfmt i może przenieść plik do `~/Applications`. `APPIMAGE` czytamy w runtime, więc nowa ścieżka działa. `APPIMAGELAUNCHER_DISABLE=1` w kroku root już jest. Skrypt budujący już omija binfmt (zeruje bajty magic kopii linuxdeploy).
- Ubuntu 24.04 ogranicza nieuprzywilejowane userns (AppArmor). Może to dotyczyć `bwrap` w glycin. Build na 24.04 eliminuje glycin **[niezweryfikowane]**.

## 4. Pakowanie natywne
| Format | Narzędzie | Plusy | Minusy | Kiedy |
|---|---|---|---|---|
| `.deb` | `cargo-deb` (`depends = "$auto"` → dpkg-shlibdeps) | jedna paczka dla Ubuntu 24.04+, Debiana 13, Minta 22, Pop, elementary, Zorina 18 i MX 25 | trzeba budować w `ubuntu:24.04`, żeby zależności wyszły `>= 4.14/1.5` | **v0.2.0** |
| `.rpm` | `cargo-generate-rpm` (`auto-req`, zależności po sonamach `libgtk-4.so.1()(64bit)`) | jedna paczka dla Fedory i openSUSE (różne nazwy pakietów, wspólne sonamy) | Leap 15.6 odpada (adw 1.4); nie podpisane | **v0.2.0** |
| `.pkg.tar.zst` | `makepkg` | już jest | — | v0.2.0 |
| AUR | PKGBUILD | `yay -S xcleaner` | wymaga publicznego repo | gdy repo publiczne |
| Fedora COPR | `.spec` | `dnf copr enable`, auto-update | publiczne repo, utrzymanie spec | v0.3 |
| openSUSE OBS | `.spec`/`.dsc` | buduje deb i rpm na wiele dystrybucji naraz | nauka OBS | v0.3 (najlepszy stosunek zysku do pracy) |
| Ubuntu PPA | Launchpad | `apt` z aktualizacjami | build ze źródeł bez sieci, czyli trzeba vendorować crate’y przez `cargo vendor` | v0.3+ |
| Snap (classic) | snapcraft | pełny dostęp do hosta | classic wymaga ręcznej zgody Snap Store; snapd tylko na Ubuntu | v0.3, opcjonalnie |
| Flatpak | flatpak-builder | Flathub i SteamOS | **słabo pasuje**, patrz niżej | nie |

Flatpak a XCleaner **[źródło: docs.flatpak.org]**:
- `--filesystem=host` **nie** daje `/usr`, `/etc`, `/proc`, `/lib`. `/var` i `/tmp` trzeba żądać osobno.
- `/proc` w sandboksie to własna przestrzeń PID, więc wykrywanie procesów (`procs.rs`) nie zadziała.
- `/var/lib/pacman` wymaga `--filesystem=/var`. `host-os` montuje `/usr` w `/run/host`, więc ścieżki trzeba by przepisać.
- Sudo w terminalu wymaga `flatpak-spawn --host`, czyli `--talk-name=org.freedesktop.Flatpak`. To praktycznie pełna ucieczka z sandboksu, którą Flathub zwykle odrzuca **[niezweryfikowane: polityka Flathub]**.
- Wniosek: Flatpak byłby zubożonym trybem „tylko HOME”. Nie na v0.2.0.

Na SteamOS i Bazzite (read-only `/usr`) zostaje AppImage.

## 5. Środowisko przekazywane procesom hosta (sudo w terminalu i inne)
Hook wtyczki GTK eksportuje: `APPDIR`, `GTK_DATA_PREFIX`, `XDG_DATA_DIRS` (z doklejonym `$APPDIR/usr/share`), `GSETTINGS_SCHEMA_DIR`, `GI_TYPELIB_PATH`, `GTK_EXE_PREFIX`, `GTK_PATH` i `GDK_PIXBUF_MODULE_FILE`. Runtime dodaje `APPIMAGE`, `APPDIR`, `ARGV0`, `OWD` i `FUSERMOUNT_PROG`. `LD_LIBRARY_PATH` **nie** jest ustawiany, bo biblioteki ładują się przez `RUNPATH $ORIGIN/../lib` **[zmierzone]**.

`FOREIGN_ENV` (root.rs:734) usuwa przy uruchamianiu terminala: LD_*, GTK_PATH, GTK_EXE_PREFIX, GTK_DATA_PREFIX, GIO_MODULE_DIR, GIO_EXTRA_MODULES, GDK_PIXBUF_*, GSETTINGS_SCHEMA_DIR, PYTHON*, PERLLIB i QT_PLUGIN_PATH.

Co nadal przecieka:
| Zmienna | Gdzie | Skutek | Sugestia |
|---|---|---|---|
| `GI_TYPELIB_PATH` | terminal i jego dzieci | narzędzia Python-GI lub GJS uruchomione w tym terminalu ładują typelibs z AppImage, co grozi niezgodnością ABI | dodać do `FOREIGN_ENV` |
| `XDG_DATA_DIRS` | terminal | wskazuje na punkt montowania, który zniknie po zamknięciu XCleanera; ikony i schematy z AppImage widać na hoście | przywrócić oryginał (zapisać przed hookiem, np. `XCLEANER_ORIG_XDG_DATA_DIRS`) albo usunąć prefiks `$APPDIR` |
| `APPDIR`, `APPIMAGE`, `ARGV0`, `OWD`, `FUSERMOUNT_PROG` | terminal | niektóre programy (inne AppImage, skrypty) traktują `APPDIR`/`APPIMAGE` jako „jestem w AppImage” | usunąć (`APPIMAGE` czytamy w GUI przed spawnem) |
| `GIO_LAUNCHED_DESKTOP_FILE*` | — | ustawia GIO tylko przy uruchamianiu przez GAppInfo | brak akcji |
| **wszystkie powyższe + GTK_PATH/GDK_PIXBUF/GSETTINGS** | `gtk::FileLauncher` („Otwórz w menedżerze plików”) | bez portalu GTK uruchamia menedżer przez GIO z **pełnym** env. Nautilus jest DBusActivatable, więc bezpieczny. Thunar, Nemo i PCManFM dostaną nasze `GDK_PIXBUF_MODULE_FILE` i `GTK_PATH`, co grozi crashem loaderów **[niezweryfikowane w praktyce]** | zamiast `FileLauncher` użyć `gio::AppInfo::launch_default_for_uri` z własnym `gdk::AppLaunchContext` (`display.app_launch_context()` + `unsetenv(...)` dla tych zmiennych) **[niezweryfikowane: czy FileLauncher pozwala wstrzyknąć kontekst, wygląda że nie]** |
| `pacman`, `paccache`, `pacman-conf` | dzieci GUI | narzędzia w C bez GTK, więc bez skutku | brak akcji |

Krok root: `sudo` (env_reset) czyści env, a my przekazujemy tylko `APPIMAGELAUNCHER_DISABLE=1`. Hook AppRun uruchamia się jednak także jako root i wywołuje `dbus-send` oraz `gsettings` z uprawnieniami root. To nieszkodliwe (najwyżej się nie powiedzie), ale zbędne. Można to rozwiązać na v0.3, np. własnym AppRun, który przy `--root` pomija hook.

## 6. Plan na v0.2.0
1. `Cargo.toml`: `gtk4 features = ["v4_10"]`, `adw features = ["v1_5"]`; `adw::Spinner` → `gtk::Spinner` (sprawdzone kompilacją).
2. `packaging/build-appimage.sh` uruchamiany w `podman run --rm -v "$PWD":/src ubuntu:24.04`. Wewnątrz `apt install build-essential curl git file python3 libgtk-4-dev libadwaita-1-dev librsvg2-dev` i rustup (rust-version 1.85). Po buildzie sprawdzić: `readelf -V … | grep 'GLIBC_2\.\(4[0-9]\)'` ma dać pusty wynik.
3. W tym samym kontenerze `cargo deb` i `cargo generate-rpm`, oba z automatycznymi zależnościami.
4. Assety release v0.2.0:
   - `XCleaner-0.2.0-x86_64.AppImage` + `.sha256` (glibc ≥ 2.39);
   - `xcleaner_0.2.0-1_amd64.deb`;
   - `xcleaner-0.2.0-1.x86_64.rpm`;
   - `xcleaner-0.2.0-1-x86_64.pkg.tar.zst`;
   - wspólny `SHA256SUMS`.
5. Dopisać zmienne z sekcji 5 do `FOREIGN_ENV` i opisać `--appimage-extract-and-run` w Install.
6. Na v0.3: maszyny wirtualne (Ubuntu 24.04, Debian 13, Fedora 43, Leap 16, Tumbleweed), OBS/COPR, decyzja o sharun dla Debiana 12 i Ubuntu 22.04.

## Źródła
- Repology API (glibc, gtk, libadwaita, libadwaita-1), pobrane 2026-11-22: https://repology.org/api/v1/project/glibc , https://repology.org/api/v1/project/gtk , https://repology.org/api/v1/project/libadwaita , https://repology.org/api/v1/project/libadwaita-1
- type2-runtime: https://github.com/AppImage/type2-runtime (README: „libfuse2 is no longer required”), `src/runtime/runtime.c` (`find_fusermount`, `FUSERMOUNT_PROG`, `APPIMAGE_EXTRACT_AND_RUN`, `setenv APPIMAGE/APPDIR/ARGV0/OWD`)
- linuxdeploy-plugin-gtk (commit 7a3fbc31): https://github.com/linuxdeploy/linuxdeploy-plugin-gtk
- Flatpak sandbox permissions: https://docs.flatpak.org/en/latest/sandbox-permissions.html
- Anylinux AppImages (sharun): https://github.com/pkgforge-dev/Anylinux-AppImages
- openSUSE Leap 16.0 (GNOME 48): https://itsfoss.com/news/opensuse-leap-16-release , https://doc.opensuse.org/release-notes/x86_64/openSUSE/Leap/16.0/
- gtk4-sys 0.11.5 / libadwaita-sys 0.9.2 `Cargo.toml` (`package.metadata.system-deps`) w `~/.cargo/registry`
- cargo-deb: https://github.com/kornelski/cargo-deb ; cargo-generate-rpm: https://github.com/cat-in-136/cargo-generate-rpm ; cargo-zigbuild: https://github.com/rust-cross/cargo-zigbuild (wiedza o flagach **[niezweryfikowane w tej sesji]**)
