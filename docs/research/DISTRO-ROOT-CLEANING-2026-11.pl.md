# Czyszczenie systemowe (root) na wszystkich dystrybucjach — research na v0.2.0 (2026-11)

[English](DISTRO-ROOT-CLEANING-2026-11.md) | **Polski**

Kontekst: dziś `root.rs` obsługuje tylko Archa (coredumpy, paccache, osierocone
`/usr/lib/modules/<ver>`, porzucone `download-*`, sync DB usuniętych repo). GUI odpala
`sudo xcleaner --root-clean <zadania>` w terminalu, root sam wylicza cele, a każde zadanie
ma probe tylko do odczytu, bez roota.

Oznaczenia: **(źródło)** = sprawdzone w man page, dokumentacji albo kodzie upstreamu (URL na końcu).
**(obs.)** = sprawdzone na maszynie dev (CachyOS, systemd 262, flatpak 1.18.4).
**(niezweryf.)** = wiedza ogólna, sprawdzić w VM w v0.3.0.
Kod GPL (BleachBit, Stacer) nie był czytany ani kopiowany.

## 0. Zasady ogólne (wnioski)

1. **Zawsze wywołuj natywne narzędzie dystrybucji** (`apt-get`, `dnf`, `zypper`, `xbps-remove`,
   `apk`, `eclean`, `flatpak`, `snap`, `journalctl`, `systemd-tmpfiles`). Ręczne `unlink` robimy
   tylko tam, gdzie narzędzia nie ma (coredumpy, crash-reporty, zrotowane logi, osierocone
   katalogi modułów). Narzędzia same biorą swoje blokady, a ich własny dry-run daje
   dokładny probe.
2. **Blokady to w większości locki fcntl/flock, a nie obecność pliku.** Pliki
   `/var/lib/dpkg/lock*`, `/var/db/xbps/lock` i `/lib/apk/db/lock` istnieją zawsze. Sprawdzenie
   w stylu Archa (`db.lck` istnieje → stop) **nie działa** poza pacmanem. Strategia:
   - po stronie roota: niech narzędzie samo spróbuje (bez czekania) i zgłosi błąd;
   - opcjonalnie wcześniej: `fcntl(F_OFD_GETLK)` / `flock(LOCK_EX|LOCK_NB)` na otwartym pliku
     (tylko jako root, bo pliki mają często 0640). Patrz tabela w §9.
3. **Probe bez roota** działa wtedy, gdy katalog ma prawo `x` (stat wpisów) i `r` (listing).
   Większość katalogów cache ma 0755. Wyjątki są opisane niżej. Jeśli probe dostanie `EACCES`,
   GUI pokazuje „rozmiar nieznany (wymaga roota)” zamiast 0.
4. **Nigdy nie automatyzujemy pełnego `autoremove`** (apt/dnf/zypper/xbps `-o`/pamac `-o`).
   Szczegóły w §2.
5. **Systemy niemutowalne** (ostree, SteamOS, NixOS, MicroOS) mają osobny, mocno okrojony
   zestaw zadań (§7). Wykrywanie robimy przed wszystkim innym.

## 1. Cache pakietów

| Menedżer | Komenda (root) | Dry-run / probe | Ścieżki | Lock | Ryzyko |
|---|---|---|---|---|---|
| apt (Debian, Ubuntu, Mint, Pop!_OS, Zorin, MX, elementary) | `apt-get autoclean` (domyślnie); `apt-get clean` (opt-in) | `apt-get -s autoclean` wypisuje `Del <pkg> <ver> [<size>B]` i działa bez roota (symulacja wyłącza locki) **(źródło: kod apt)**. Rozmiar dla `clean` liczymy statem `archives/*.deb` | `/var/cache/apt/archives/*.deb`, `archives/partial/`, `/var/lib/apt/lists/partial/`, `/var/cache/apt/pkgcache.bin`, `srcpkgcache.bin` | `archives/lock` (fcntl, bierze go apt); dpkg `lock-frontend` | niskie |
| dnf4 (Fedora ≤40, RHEL-pochodne) | `dnf clean packages` | stat `/var/cache/dnf/*/packages/*.rpm` | `/var/cache/dnf` | `ProcessLock` (`*_lock.pid` + flock) w katalogu locków dnf **(źródło)** | niskie |
| dnf5 (Fedora 41+) | `dnf5 clean packages` | stat `/var/cache/libdnf5/*/packages/*.rpm` | `system_cachedir=/var/cache/libdnf5` **(źródło)** | dnf5 sam blokuje (ścieżka **niezweryf.**) | niskie |
| zypper (openSUSE) | `zypper --non-interactive clean` (tylko pakiety). Bez `--all`/`--metadata`, bo te usuwają też metadane i następny run musi je pobrać od nowa | stat `/var/cache/zypp/packages/<alias>/**.rpm` | `/var/cache/zypp/packages` (`packagesdir`) **(źródło)** | `/run/zypp.pid` (PID w pliku, a nie samo istnienie) **(źródło)**; exit 7 = ZYPP locked (np. PackageKit) | niskie |
| xbps (Void) | `xbps-remove -O` (nieaktualne); `-OO` (też nieinstalowane, opt-in) | `xbps-remove -On` (`--dry-run`) **(źródło)** | `/var/cache/xbps` | flock `/var/db/xbps/lock`; xbps **czeka** na lock („package database locked, waiting...”) **(źródło: lib/pkgdb.c)**, więc root musi wcześniej sam sprawdzić `LOCK_NB` | niskie |
| apk (Alpine) | `apk cache clean` (pakiety spoza indeksów); `apk cache purge` opt-in | `apk cache clean --simulate` **(źródło)** | `/etc/apk/cache` → zwykle symlink; cache **tylko gdy włączony** | flock `/lib/apk/db/lock`, `LOCK_NB` bez `--wait` → błąd **(źródło: database.c)** | niskie |
| portage (Gentoo) | `eclean distfiles` (domyślny tryb = zostawia pliki ebuildów obecnych w repo); `eclean packages` | `eclean --pretend distfiles` **(źródło)** | `DISTDIR=/var/cache/distfiles`, `PKGDIR=/var/cache/binpkgs` (z `make.conf`) | brak w docs; nie uruchamiać, gdy działa `emerge` (sprawdzić proces) **(niezweryf.)** | niskie; `--deep` = średnie (traci możliwość downgrade), tylko opt-in |
| pacman/pamac (Manjaro) | jak na Archu: `paccache` (pacman-contrib); alternatywa `pamac clean -k N` / `-u` / `-b` | `pamac clean --dry-run` **(źródło: pamac.8)** | `/var/cache/pacman/pkg`; build AUR: `BuildDirectory=/var/tmp` (domyślny pamac.conf) **(źródło)** | `db.lck` jak dziś | niskie |

Uwagi:
- **apt `clean` vs `autoclean`:** `clean` usuwa wszystko z `archives/` i `archives/partial/`
  oprócz locka, czyści `lists/partial/` oraz kasuje `pkgcache.bin` i `srcpkgcache.bin`,
  które apt odtworzy przy następnym uruchomieniu (zweryfikowane w `apt-private/private-download.cc`:
  `CleanDownloadDirectories` → `pkgCacheFile::RemoveCaches()`). `autoclean` usuwa tylko
  .deb, których nie da się już pobrać. **Nie używać `apt-get distclean`**, bo kasuje też
  listy pakietów. Binarka `apt` domyślnie usuwa .deb po udanej instalacji
  (`Binary::apt::APT::Keep-Downloaded-Packages "0"`, od apt 1.2), a `apt-get` ich nie usuwa.
  Cache bywa więc mały, chyba że ktoś używa `apt-get`, Synaptica albo unattended-upgrades.
- **Uprawnienia apt:** `archives/` ma 0755, ale `archives/partial/` należy do `_apt` z 0700
  (sandbox apt) **(niezweryf.)**. Probe liczy wtedy `partial/` jako nieznany.
- **dnf `keepcache`** ma domyślnie `False` (dnf4 i dnf5) **(źródło)**: pakiety są kasowane
  po udanej transakcji. Cache rośnie tylko po przerwanych transakcjach, przy `keepcache=1`
  albo przy `dnf download`/`system-upgrade`. W GUI to zwykle kilka MB, ale zadanie i tak warto mieć.
  Dodatkowo: `dnf system-upgrade clean` (pozostałości po upgrade wersji Fedory,
  `/var/lib/dnf/system-upgrade`) **(niezweryf.)**.
- **zypper:** pakiety trafiają do cache tylko dla repo z `keeppackages=1` (`zypper mr -k`) **(źródło)**.
  Domyślnie jest to wyłączone **(niezweryf.)**.
- **Manjaro:** `pamac clean` działa przez pamac-daemon (polkit), a więc wbrew zasadzie
  „sudo w terminalu, bez polkit”. Zalecenie: na Manjaro używać tej samej ścieżki co na Archu
  (`paccache`). Pakiet `pacman-contrib` może nie być zainstalowany: wtedy zadanie jest
  ukryte z podpowiedzią **(niezweryf. dla domyślnego ISO)**. Osobne zadanie z niskim ryzykiem:
  `/var/tmp/pamac-build-<user>` (pozostałości buildów AUR), tylko gdy pamac nie działa.
- **EndeavourOS i CachyOS** = Arch (bez zmian). EndeavourOS ma własny `paccache` hook i
  narzędzie „eos-…”, ale XCleaner niczego z nich nie potrzebuje.

## 2. Stare kernele

**Rekomendacja ogólna:** XCleaner **nie kasuje plików kerneli ani `/boot` ręcznie**.
Na Debianie/Ubuntu `/boot/vmlinuz-*` i `initrd.img-*` należą do pakietów albo są
generowane przez hooki (`/etc/kernel/postrm.d`, initramfs-tools, update-grub). Ręczne
usunięcie zostawia wpisy w GRUB-ie, rozjeżdża dpkg i przy następnej aktualizacji psuje
generowanie initramfs (niezweryf. w szczegółach, ale wynika z modelu dpkg i hooków).
Kernele usuwamy wyłącznie menedżerem pakietów, z dry-runem pokazanym w GUI.

| Distro | Mechanizm upstream | Co robi XCleaner | Probe | Ryzyko |
|---|---|---|---|---|
| Debian, Ubuntu, Mint, Pop!_OS, Zorin, MX, elementary | `apt autoremove --purge`. Apt chroni kernele: `APT::Protect-Kernels=true`, chroniony jest kernel działający oraz najnowszy, `APT::NeverAutoRemove::KernelCount` = 2 **(źródło: apt.conf(5), test apt)** | **Nie** całe `autoremove`. Zamiast tego: (1) `apt-get -s autoremove --purge` (bez roota) → (2) filtr do `linux-image-*`, `linux-headers-*`, `linux-modules-*`, `linux-modules-extra-*` z wersją ≠ `uname -r` i bez metapakietów (`linux-image-amd64`, `linux-generic` itd.) → (3) root robi `apt-get -s purge <lista>` i **przerywa**, jeśli plan zawiera cokolwiek spoza listy → (4) `apt-get -y purge <lista>` | lista `Remv` z symulacji + `dpkg-query -W -f='${Installed-Size}'` (KiB) | średnie (dotyka bootloadera przez hooki), ale chronione przez apt; nigdy niezaznaczone domyślnie |
| Fedora (dnf4/dnf5), Nobara | `installonly_limit=3` (min. 2), `protect_running_kernel=True` **(źródło)**; `dnf remove --oldinstallonly` usuwa ponad limit, z zachowaniem działającego kernela **(źródło)** | Root: `dnf remove --oldinstallonly --assumeno` (pokaż plan) → gdy plan zawiera tylko pakiety `kernel*` i ich `installonly` → `dnf -y remove --oldinstallonly`. Uwaga: dnf5 ma `clean_requirements_on_remove=True` **(źródło)**, więc plan może zawierać zależności i trzeba go filtrować | bez roota: `rpm -q kernel-core --qf ...` + `uname -r` + `installonly_limit` z `/etc/dnf/dnf.conf` (heurystyka) | niskie/średnie; dnf sam pilnuje limitu, więc nadmiar pojawia się rzadko (zwykle po podniesieniu limitu) |
| openSUSE Tumbleweed/Leap | `zypper purge-kernels` wg `multiversion.kernels` (vendor: `latest,latest-1,running`) **(źródło: zypp.conf(5))**; jest też `purge-kernels.service` przy starcie **(niezweryf.)** | Root: `zypper purge-kernels --dry-run` → `zypper -n purge-kernels` | dry-run wymaga roota (lock zypp) **(niezweryf.)**; bez roota: `rpm -q kernel-default` | niskie (zgodnie z polityką distro) |

**Czy `apt autoremove` jest bezpieczne do automatyzacji? Nie.** Usuwa *wszystkie*
pakiety z flagą „auto”, których nic nie wymaga. Po `apt remove` metapakietu pulpitu
(np. `ubuntu-desktop`) albo po błędnym oznaczeniu z apt-mark może zdjąć połowę systemu
(znany wzorzec w zgłoszeniach, niezweryf. konkretnym linkiem). To samo dotyczy `dnf autoremove`, `xbps-remove -o`
i `pamac remove -o`. XCleaner może to najwyżej **pokazać** (lista z symulacji, „Do przejrzenia”)
i nigdy nie powinien tego wykonywać zbiorczo.

**Osierocone katalogi modułów** (odpowiednik dzisiejszego zadania z Archa):
- Debian/Ubuntu: `/usr/lib/modules/<ver>` (na usrmerge `/lib` → `/usr/lib`). **Pułapka:** dpkg
  nie zna aliasów usrmerge. Trzeba pytać o obie ścieżki: `dpkg-query -S /lib/modules/<ver>` i
  `dpkg-query -S /usr/lib/modules/<ver>` (pakiety jądra wciąż często deklarują `/lib/...`) **(niezweryf.)**.
  Typowe źródło sierot: DKMS i `linux-modules-extra` po purge.
- Fedora/openSUSE: `rpm -qf /usr/lib/modules/<ver>` → „not owned”. Bez roota, bo rpmdb jest czytelna.
- Warunki jak dziś: brak właściciela, ≠ `uname -r`, brak `vmlinuz`/`modules.dep` należącego do pakietu.
  Ryzyko niskie, zostawić jako „Do przejrzenia”, gdy w katalogu są pliki `updates/dkms`.

## 3. Zrzuty awaryjne

| Źródło | Ścieżka | Sprzątanie upstream | XCleaner | Ryzyko |
|---|---|---|---|---|
| systemd-coredump (Arch, Fedora, openSUSE, Debian z pakietem) | `/var/lib/systemd/coredump/core.*` | tmpfiles `d /var/lib/systemd/coredump 0755 root root 2w` (upstream 2 tyg.; CachyOS nadpisuje na 3d) **(obs.)**; limity `MaxUse=`/`KeepFree=` w coredump.conf | to samo co dziś; probe: stat (katalog 0755) **(obs.)**. `coredumpctl` nie ma polecenia usuwania **(źródło)** | niskie |
| apport (Ubuntu, Mint, Pop, Zorin, elementary) | `/var/crash/*.crash`, `*.upload`, `*.uploaded` | `/etc/cron.daily/apport`: usuwa puste pliki oraz pliki >7 dni, a także katalogi `/var/crash/YYYYMMDDHHMM` >7 dni **(źródło: canonical/apport)** | regularne pliki `*.crash`/`*.upload*` dowolnego wieku; probe stat. Pomijamy pliki otwarte (whoopsie/apport w trakcie pisania) | niskie |
| kdump-tools (Debian/Ubuntu) | `/var/crash/<YYYYMMDDHHMM>/dump.*`, `dmesg.*` | jak wyżej (cron apport) | „Do przejrzenia”, nigdy domyślnie: vmcore to dowód przy debugowaniu paniki | średnie |
| kdump (Fedora/RHEL) | `/var/crash/<ip>-<data>/vmcore` (domyślny `path` w kdump.conf) **(niezweryf.)** | brak automatu | „Do przejrzenia” | średnie |
| ABRT (Fedora, starsze/opcjonalne) | `DumpLocation=/var/spool/abrt` (domyślnie), limit `MaxCrashReportsSize=5000` MiB **(źródło: abrt.conf.txt)**; `/var/tmp/abrt` (niezweryf.) | ABRT sam przycina do limitu | preferować `abrt remove`/`abrt-cli rm <dir>`; fallback: katalogi problemów w spool, gdy `abrtd` nie działa | niskie |

## 4. journald i /var/log

- `journalctl --vacuum-time=` / `--vacuum-size=` / `--vacuum-files=` dotyczą **tylko plików
  zarchiwizowanych**. Aktywne pliki zostają, chyba że wcześniej podano `--rotate` **(źródło)**.
  **Dry-run nie istnieje.** Probe: `journalctl --disk-usage` (archiwalne + aktywne).
- Kto może czytać: użytkownik widzi swoje journale, a grupy `systemd-journal`, `adm` i `wheel`
  widzą wszystko **(źródło)**. Na dev maszynie `--disk-usage` działa bez roota (użytkownik w wheel) **(obs.)**.
  Fallback bez grup: stat plików `/var/log/journal/<machine-id>/*.journal`. Katalog ma 2755
  oraz ACL `wheel`/`adm` r-x **(obs.)**, więc stat zwykle się udaje (niezweryf. na Debianie, gdzie
  użytkownik nie jest w `adm`). Do oszacowania vacuum: suma plików `*@*.journal` (archiwalne)
  starszych niż próg (mtime).
- Domyślne: `Storage=` ma wartość kompilacyjną (strona systemd 262 podaje `persistent`).
  W trybie `auto` journal jest trwały tylko, gdy istnieje `/var/log/journal`. `SystemMaxUse`
  domyślnie 10% FS (max 4G), `MaxRetentionSec=0` (wyłączone) **(źródło)**. Dystrybucje
  nadpisują (CachyOS: `SystemMaxUse=50M` w `journald.conf.d` **(obs.)**). Przy volatile
  (`/run/log/journal`) zadanie jest ukryte.
- **Zadanie:** `journalctl --vacuum-time=<N>` (domyślnie 4 tygodnie, do wyboru w GUI), bez
  `--rotate`. Nigdy `--vacuum-size=1` ani „usuń wszystko” (zgodnie z ROADMAP). Ryzyko: niskie
  (tracimy historię logów, co może przeszkodzić w diagnozie).

**Zrotowane logi w /var/log:**
- logrotate kasuje stare wersje sam (`rotate N`, `maxage`). Po ręcznym usunięciu starych
  plików logowanie trwa dalej poprawnie **(źródło: logrotate(8), opcja -f)**. Usunięcie
  *zrotowanych* plików jest więc bezpieczne; **aktywnych nie ruszamy nigdy**.
- Dopasowanie (tylko regularne pliki, `nlink==1`, bez symlinków, nie w `/var/log/journal`):
  `*.[0-9]`, `*.[0-9].gz|.xz|.zst|.bz2`, `*-YYYYMMDD` / `*-YYYYMMDD.gz` (`dateext`, domyślne
  na Fedorze/openSUSE **(niezweryf.)**), `*.old`. Nie dopasowujemy samego `*.gz` bez numeru/daty,
  bo to może być aktywny log aplikacji.
- Wykluczenia / „Do przejrzenia”: `/var/log/audit/` (rotuje auditd, wymagania audytu),
  `wtmp.1`/`btmp.1` (historia logowań; `btmp` to dowód prób włamań), `installer/` (Debian/Ubuntu,
  logi instalatora), `anaconda/` (Fedora).
- Probe: katalog `/var/log` ma 0755, pliki często 0600, ale **stat działa** (rozmiar bez czytania
  treści) **(obs.: `boot.log.N` 0600 root)**. Ryzyko: niskie.

## 5. /tmp i /var/tmp

- Upstream `tmp.conf`: `q /tmp 1777 root root 10d`, `q /var/tmp 1777 root root 30d` **(obs.)**.
  Wiek liczony z atime, mtime, ctime i btime (domyślnie `abcmABM`), a nowszy dowolny znacznik
  blokuje usunięcie. Wpisy z istniejącym flock są pomijane razem z poddrzewem. `x`/`X` wykluczają
  ścieżki **(źródło: tmpfiles.d(5))**. Codziennie uruchamia to `systemd-tmpfiles-clean.timer`.
- `systemd-tmpfiles --clean --dry-run` istnieje od v256, a `-n` od v262 **(źródło)**. To daje dokładny probe
  (ale tylko roota, bo musi przejść całe drzewa innych użytkowników).
- **Rekomendacja:** zadanie root `tmpfiles-clean` = `systemd-tmpfiles --clean` (dokładnie to,
  co robi timer, z tymi samymi wykluczeniami i lockami). Probe: `--dry-run`, jeśli systemd ≥256,
  w przeciwnym razie rozmiar nieznany. **Własnego kasowania /var/tmp jako root nie robić**: są tam
  `systemd-private-*` (PrivateTmp działających usług), sockety, pliki otwarte, katalogi buildów.
  Zysk jest mały, skoro timer i tak działa codziennie. Większy zysk to pliki **użytkownika** w `/var/tmp`
  (np. `pamac-build-<user>`, `flatpak-cache-*`) w fazie HOME (uid = właściciel, wiek > 30 d, bez flock).
- `/tmp` to zwykle tmpfs (Fedora, Arch, Debian 13+, niezweryf. dla wszystkich). Nie ruszamy.
- Na systemach bez systemd (Void z runit, Alpine z OpenRC, Gentoo z OpenRC) czyszczenie `/tmp`
  zależy od init/cron. Pomijamy.

## 6. Flatpak i Snap

**Flatpak**
- `flatpak uninstall --unused` usuwa nieużywane refy (runtime’y/rozszerzenia bez aplikacji).
  Przypięte (`flatpak pin`) zostają (niezweryf. w docs, opis pin: „prevent automatic removal”).
  Opcje `-y`/`--noninteractive` istnieją, a `--dry-run` **nie istnieje** (man 1.18) **(obs.)**.
- **Probe bez roota:** `flatpak uninstall --unused --system` wypisuje listę i pyta `[Y/n]`, więc
  podanie `n` na stdin anuluje operację **(obs.: 5 nieużywanych refów, np. org.gnome.Platform 51)**.
  Wystarczy parsować to wyjście (`LC_ALL=C`, bo wyjście jest lokalizowane).
  Alternatywa bez interakcji: API `flatpak_installation_list_unused_refs()` z libflatpak
  (niezweryf. wersja dodania). Rozmiar: `flatpak info --show-size <ref>` albo stat katalogu
  `/var/lib/flatpak/runtime/<id>/<arch>/<branch>` (to współdzielony repo ostree, więc rozmiar
  jest **przybliżony**, bo obiekty mogą być współdzielone).
- Wykonanie: system = root `flatpak uninstall --unused --system -y --noninteractive`;
  user = bez roota `flatpak uninstall --unused --user -y` (faza HOME). Ryzyko: niskie.
  Lock: flatpak sam blokuje repo, więc nie trzeba sprawdzać ręcznie.
- `flatpak repair` (`--dry-run` istnieje) przycina obiekty bez refów i ponownie instaluje
  uszkodzone **(obs.: man)**. Nie dawać go jako „czyszczenia”, bo może pobierać z sieci. Najwyżej
  jako akcja naprawcza, poza v0.2.
- `/var/lib/flatpak/.removed/`: tu trafiają deploye odinstalowane w trakcie działania aplikacji.
  `flatpak_dir_cleanup_removed()` kasuje je przy kolejnych operacjach, ale tylko gdy deploy
  nie jest zablokowany (`deploy_dir_is_locked`) **(źródło: flatpak-dir.c)**. **(obs.: 142 MB po epicgames
  launcherze.)** Zalecenie: nie kasować ręcznie. Wystarczy wywołać operację flatpaka (np. zadanie
  `--unused`) przy zamkniętej aplikacji. Probe: stat `.removed/*` (0755) **(obs.)**.

**Snap (Ubuntu)**
- `refresh.retain`: domyślnie **2 na classic**, 3 na Ubuntu Core, zakres 2–20 **(źródło: snapd
  overlord/snapstate/snap.go, configcore/refresh.go)**.
- Wyłączone rewizje: `snap list --all` (kolumna Notes = `disabled`), usuwanie przez
  `snap remove <name> --revision=<rev>`. Rozmiar: stat `/var/lib/snapd/snaps/<name>_<rev>.snap`
  (bez roota, niezweryf. prawa). Ryzyko niskie: tracimy możliwość `snap revert`. Lock: snapd sam
  serializuje zmiany, a przy konflikcie komenda zwraca błąd. Trzeba pomijać snapy z aktywną zmianą
  (`snap changes`).
- `/var/lib/snapd/cache/`: pliki są **hardlinkami** do snapów w `snaps/` (cache po treści). Plik
  z `nlink>1` nie zwalnia miejsca. snapd sam usuwa „unreferenced” (nlink==1), maks. 5 sztuk,
  starsze niż 30 dni **(źródło: snapd store/cache.go)**. **Nie ruszać.** Najwyżej pokazać sumę plików
  z nlink==1 jako informację.

## 7. Systemy niemutowalne

| System | Wykrywanie | Co robić | Czego nie robić |
|---|---|---|---|
| Fedora Silverblue/Kinoite, Bazzite, Aurora, Bluefin (rpm-ostree/bootc) | `/run/ostree-booted` istnieje | `rpm-ostree cleanup -b` (przerwane operacje, „use this first”) i `-m` (cache repomd) **(źródło: rpm-ostree(1))**; flatpak `--unused` | `-r` (rollback) i `-p` (pending) tylko jako opt-in z ostrzeżeniem (traci się powrót); nie dotykać `/usr`, `/sysroot`, ani cache pakietów |
| SteamOS 3 | `ID=steamos` w os-release; root tylko do odczytu (A/B) **(niezweryf.)** | flatpak `--unused`, journald vacuum, coredumpy (`/var` jest zapisywalne) **(niezweryf.)** | żadnych pacmanowych zadań (paccache, moduły, sync DB): rootfs jest read-only i nadpisywany przy aktualizacji |
| NixOS | `/etc/NIXOS` lub `ID=nixos` | najwyżej informacja: `nix-collect-garbage --delete-older-than 30d` (opt-in). Docs: „Deleting previous configurations makes rollbacks to them impossible” **(źródło)**. Probe: `nix-store --gc --print-dead` (niezweryf. koszt) | `-d` domyślnie; ręczne kasowanie w `/nix/store`; wpisy bootloadera znikają dopiero po `nixos-rebuild boot/switch` **(niezweryf.)** |
| openSUSE MicroOS/Aeon/Kalpa (transactional-update) | `/usr/sbin/transactional-update` + root btrfs ro | flatpak, journald | `zypper` na żywym systemie; usuwanie snapshotów snappera (to mechanizm rollbacku; snapper ma własny `cleanup` wg `NUMBER_LIMIT`) |
| openSUSE TW/Leap ze snapperem | `/.snapshots` + `snapper` | pokazać informację „snapshoty snappera zajmują X” (`snapper list` wymaga roota albo ALLOW_USERS) | kasować snapshoty |

## 8. Eskalacja uprawnień bez polkit

| Mechanizm | Gdzie domyślnie | Uwagi |
|---|---|---|
| `sudo` | Arch-pochodne (wheel), Ubuntu i pochodne (grupa `sudo`), Fedora (wheel), openSUSE (sudo pyta o hasło **roota**, `Defaults targetpw`, niezweryf.), Void (zainstalowany, ale reguła `%wheel` zakomentowana **(źródło)**) | dziś używane; ustawia `SUDO_UID` |
| `su -c` | Debian, gdy podczas instalacji ustawiono hasło roota: wtedy „sudo does not get installed” i użytkownik nie trafia do grupy `sudo` **(źródło: wiki.debian.org/sudo)** | `su` przekazuje polecenie do powłoki roota jako **jeden string**, więc potrzebne jest bezpieczne cytowanie (ścieżka AppImage może mieć spacje). Nie ustawia `SUDO_UID` |
| `doas` | Alpine (zamiennik sudo od 3.15/3.16; `setup-user -a` dodaje do wheel i instaluje doas; reguły w `/etc/doas.d/*.conf`) (niezweryf.: wiki Alpine zwraca 403, wiedza z wyników wyszukiwania); opcjonalnie Void/Gentoo/Arch | OpenDoas ustawia `DOAS_USER` (nazwa, a nie UID) **(niezweryf.)** |
| `run0` | systemd ≥256 **(źródło)** | **Wymaga polkit** (uwierzytelnienie przez polkit; bez polkitd PID1 odrzuci nieuprzywilejowanego, niezweryf.), więc łamie zasadę „bez polkit”. Najwyżej ostatni fallback, gdy polkitd działa. Własny pty. Nie ustawia `SUDO_UID` (brak w man 262) **(obs.)** |
| `pkexec` | prawie wszędzie z polkitem | polkit, więc odpada (ma tekstowego agenta jako fallback **(źródło)**, ale to wciąż polkit) |

**Wykrywanie (propozycja):**
1. `geteuid()==0` → odmowa GUI jak dziś.
2. `sudo` w `PATH` (zaufane: `/usr/bin`, `/bin`) **i** użytkownik w grupie `wheel`/`sudo`/`admin`
   (getgroups). `sudo -n true` bez hasła nic nie mówi, a `sudo -l` pyta o hasło, więc decydują grupy.
   Na openSUSE i Void sudo może mimo to pytać o hasło roota albo odmówić; komunikat o błędzie trzeba pokazać.
3. `doas` w PATH i istnieje `/etc/doas.conf` lub `/etc/doas.d/` (pliki często 0600, więc nie parsować).
4. W przeciwnym razie `su -c` (hasło roota).
5. `run0` tylko po jawnym wyborze w ustawieniach.
6. Zawsze: przełącznik w ustawieniach (sudo/doas/su/run0) jako ręczne nadpisanie.

**Zmiana wymagana w kodzie:** `run_root()` wymaga dziś `SUDO_UID`. Dla doas/su/run0 przekazać
UID jawnie (np. `--uid <n>` dodawany przez GUI). To bezpieczne, bo proces jest już rootem po
uwierzytelnieniu, a uid służy tylko do sprawdzenia właściciela pliku raportu. Kolejność:
`SUDO_UID` → `PKEXEC_UID` → `--uid` → `DOAS_USER` (getpwnam).

## 9. Locki: skrót

| Narzędzie | Plik | Typ | Wykrywanie przed startem (root) |
|---|---|---|---|
| pacman | `/var/lib/pacman/db.lck` | istnienie pliku | jak dziś |
| apt/dpkg | `/var/lib/dpkg/lock-frontend`, `/var/lib/dpkg/lock`, `/var/cache/apt/archives/lock`, `/var/lib/apt/lists/lock` | fcntl (cały plik) **(źródło: dpkg frontend.txt, apt)** | `F_OFD_GETLK`/`F_GETLK` na `lock-frontend`; apt-get i tak zakończy się błędem (exit 100) |
| dnf4 | `*_lock.pid` w katalogu locków dnf | PID + flock **(źródło: dnf/lock.py)** | zostawić dnf (exit 200 = problem z lockiem) **(źródło)** |
| dnf5, rpm | (niezweryf.) / `.rpm.lock` w rpmdb (niezweryf.) | — | zostawić narzędziu |
| zypper | `/run/zypp.pid` | PID w pliku **(źródło)** | odczytaj PID, sprawdź `/proc/<pid>`; exit 7 = locked |
| xbps | `/var/db/xbps/lock` | flock, **czeka** **(źródło)** | `flock(LOCK_EX|LOCK_NB)` przed uruchomieniem, inaczej terminal zawiśnie |
| apk | `/lib/apk/db/lock` | flock, bez czekania **(źródło)** | zostawić apk |
| PackageKit | trzyma locki powyższych | — | dodatkowo komunikat „Zamknij Menedżera oprogramowania / Discover” |

## 10. Źródła

- apt-get(8): https://manpages.debian.org/trixie/apt/apt-get.8.en.html
- apt.conf(5): https://manpages.debian.org/trixie/apt/apt.conf.5.en.html
- apt kod `CleanDownloadDirectories`: https://salsa.debian.org/apt-team/apt (mirror https://github.com/Debian/apt, `apt-private/private-download.cc`)
- apt test ochrony kerneli: https://github.com/Debian/apt/blob/main/test/integration/test-kernel-helper-autoremove
- Keep-Downloaded-Packages: https://bugs.launchpad.net/bugs/1970110 , https://lists.debian.org/deity/2020/01/msg00046.html
- dpkg frontend locking: https://sources.debian.org/src/dpkg/1.17.12/doc/frontend.txt , https://bugs.launchpad.net/bugs/1781169
- dnf5 clean: https://dnf5.readthedocs.io/en/latest/commands/clean.8.html ; dnf5.conf: https://dnf5.readthedocs.io/en/latest/dnf5.conf.5.html ; dnf5 remove: https://dnf5.readthedocs.io/en/latest/commands/remove.8.html
- dnf4 conf/command ref: https://dnf.readthedocs.io/en/latest/conf_ref.html , https://dnf.readthedocs.io/en/latest/command_ref.html ; lock: https://github.com/rpm-software-management/dnf/blob/master/dnf/lock.py
- zypper(8): https://man.archlinux.org/man/zypper.8.en ; zypp.conf(5): https://github.com/openSUSE/libzypp/blob/master/zypp/doc/zypp.conf.5.txt ; lock: https://github.com/openSUSE/libzypp/tree/master/maintainer-docs
- xbps-remove(1): https://man.voidlinux.org/xbps-remove.1 ; lock: https://github.com/void-linux/xbps/blob/master/lib/pkgdb.c
- apk-cache(8): https://man.archlinux.org/man/apk-cache.8.en ; lock: https://gitlab.alpinelinux.org/alpine/apk-tools/-/blob/master/src/database.c
- eclean: https://wiki.gentoo.org/wiki/Eclean
- pamac(8), pamac.conf(5): https://github.com/manjaro/pamac-cli/tree/master/data/doc ; domyślny pamac.conf: https://github.com/manjaro/libpamac/blob/master/data/config/pamac.conf
- journalctl(1): https://www.freedesktop.org/software/systemd/man/latest/journalctl.html ; journald.conf(5): https://www.freedesktop.org/software/systemd/man/latest/journald.conf.html
- tmpfiles.d(5): https://www.freedesktop.org/software/systemd/man/latest/tmpfiles.d.html ; systemd-tmpfiles(8): https://www.freedesktop.org/software/systemd/man/latest/systemd-tmpfiles.html
- coredumpctl(1): https://www.freedesktop.org/software/systemd/man/latest/coredumpctl.html
- run0(1): https://www.freedesktop.org/software/systemd/man/latest/run0.html ; pkexec(1): https://man.archlinux.org/man/pkexec.1.en
- apport cron: https://github.com/canonical/apport/blob/main/etc/cron.daily/apport
- ABRT abrt.conf: https://github.com/abrt/abrt/blob/master/doc/abrt.conf.txt
- logrotate(8): https://manpages.debian.org/trixie/logrotate/logrotate.8.en.html
- flatpak: https://docs.flatpak.org/en/latest/flatpak-command-reference.html ; `.removed`: https://github.com/flatpak/flatpak/blob/main/common/flatpak-dir.c
- snapd retain/cache: https://github.com/canonical/snapd/blob/master/overlord/snapstate/snap.go , https://github.com/canonical/snapd/blob/master/overlord/configstate/configcore/refresh.go , https://github.com/canonical/snapd/blob/master/store/cache.go
- rpm-ostree(1): https://github.com/coreos/rpm-ostree/blob/main/man/rpm-ostree.xml
- nix-collect-garbage: https://nix.dev/manual/nix/2.28/command-ref/nix-collect-garbage
- Debian sudo: https://wiki.debian.org/sudo ; Void: https://docs.voidlinux.org/config/users-and-groups.html ; Alpine doas (403 przy pobieraniu, cytowane z wyników wyszukiwania): https://wiki.alpinelinux.org/wiki/Setting_up_a_new_user
- Niedostępne podczas researchu (403/404/login): wiki Alpine, en.opensuse.org SDB, Fedora docs (Anubis), dokumentacja snap (logowanie readthedocs), wiki Manjaro. Wnioski z nich są oznaczone (niezweryf.) albo oparte na kodzie źródłowym.
