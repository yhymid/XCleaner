# Pozostałości po aplikacjach na innych dystrybucjach: menedżery pakietów (2026-11-22)

[English](DISTRO-PACKAGES-2026-11.md) | **Polski**

Cel v0.2.0: rozszerzyć detekcję z `LEFTOVERS-DETECTION-2026-11.md` (dziś tylko pacman) na
popularne dystrybucje. Cała detekcja ma działać **bez roota** i **bez procesu na pakiet**.
Model trzech faktów zostaje bez zmian:

1. **removed**: nazwa pasuje do pakietu usuniętego według dziennika menedżera pakietów,
2. **brak zainstalowanej tożsamości**: lokalna baza pakietów (nazwy, provides, pliki), PATH,
   `.desktop`, Flatpak, Snap, AppImage, `/proc`,
3. **btime katalogu nie późniejszy niż usunięcie**.

Do tego **aliasy usuniętego pakietu**, czyli nazwy jego binarek i plików `.desktop`. Na Arch
dają je `pacman -Fl`.

Oznaczenia: **[Ź]** = sprawdzone w źródle lub w oficjalnej dokumentacji (link w §9).
**[NZ]** = niezweryfikowane, z pamięci lub z drugiej ręki; sprawdzić na prawdziwej maszynie
lub w VM. Kodu GPL nie kopiowano; opisane są wyłącznie formaty.

---

## 0. TL;DR: zalecany projekt

| Rodzina | Fakt 1: usunięcia | Fakt 2: zainstalowane | Aliasy usuniętych | Blokada (menedżer działa) |
|---|---|---|---|---|
| pacman (Arch, CachyOS, Manjaro, EndeavourOS, SteamOS) | `/var/log/pacman.log` (jest) | `/var/lib/pacman/local` (jest) | `pacman -Fl` (jest) | `/var/lib/pacman/db.lck` istnieje |
| dpkg/apt (Debian, Ubuntu, Mint, Pop!_OS, Zorin, elementary, MX) | `/var/log/dpkg.log` + `dpkg.log.N[.gz]` (status `config-files`/`not-installed`); dodatkowo `apt/history.log*` | `/var/lib/dpkg/status` + `info/*.list` | 1) `info/<pkg>.list` pakietów `rc` (pliki pozostałe po usunięciu); 2) `Contents-*.lz4` z `/var/lib/apt/lists`, jeśli są; 3) `.deb` w `/var/cache/apt/archives` | fcntl na `/var/lib/dpkg/lock-frontend` / `lock` → `/proc/locks` |
| rpm + dnf4 (RHEL/Alma/Rocky 8–9, starsza Fedora) | `/var/lib/dnf/history.sqlite` (`trans_item.action=8`) | `rpmdb.sqlite` (indeksy `Name`, `Providename` + nagłówki) albo jeden `rpm -qa --qf` | `filelists.xml.gz` w cache dnf, jeśli jest; zwykle go nie ma | `/var/lib/rpm/.rpm.lock` (fcntl), `/run/dnf/rpmtransaction.lock` |
| rpm + dnf5 (Fedora ≥ 41, Nobara) | `/usr/lib/sysimage/libdnf5/transaction_history.sqlite` (`action_id=5`) | jw. | jw. (dnf5 domyślnie nie pobiera filelists) | jw. |
| zypper (openSUSE TW/Leap, Aeon) | `/var/log/zypp/history` (`remove` w CSV z `\|`), prawa dostępu [NZ] | rpmdb ndb (`/usr/lib/sysimage/rpm/Packages.db`) → jeden `rpm -qa --qf` | `filelists.xml.gz` w `/var/cache/zypp/raw/…` [NZ] | `/run/zypp.pid` (PID w pliku) |
| xbps (Void) | brak dziennika → tylko „nieznane” | `/var/db/xbps/pkgdb-0.38.plist` + `.<pkg>-files.plist` | brak | flock `/var/db/xbps/lock` |
| apk (Alpine) | brak dziennika | `/lib/apk/db/installed` | brak | `/lib/apk/db/lock` |
| portage (Gentoo) | `/var/log/emerge.log` (0660 portage → zwykle nieczytelny) | `/var/db/pkg/*/*/CONTENTS`, `PROVIDES` | brak (CONTENTS znika z pakietem) | `/var/db/.pkg.portage_lockfile` |
| eopkg (Solus) | `/var/lib/eopkg/history/*.xml` | `/var/lib/eopkg/package/*/{metadata,files}.xml` [NZ] | brak | [NZ] |
| Nix (NixOS) | **pominąć**: brak „usunięć” jako zdarzeń | profile (`/run/current-system/sw`, `~/.nix-profile`, `/etc/profiles/per-user/$USER`) jako fakt 2 | — | — |
| Flatpak | journal (`MESSAGE_ID=c7b39b1e…`), opcjonalnie | `app/<id>/current` w instalacjach | nazwa = ID | — |
| Snap | brak dziennika plikowego | `/snap/<name>`, `/var/lib/snapd/snaps/<name>_*.snap` | — | — |

Zasady projektu:
- Wprowadzić trait `PackageBackend { removed_events(), installed_identities(),
  removed_aliases(), is_busy() }`. Backend wybierać po **istnieniu baz** (`/var/lib/pacman/local`,
  `/var/lib/dpkg/status`, `rpmdb.sqlite`/`Packages.db`, `/var/db/xbps`, `/lib/apk/db/installed`,
  `/var/db/pkg`, `/var/lib/eopkg`), a nie po `/etc/os-release`. Na jednym systemie może działać
  kilka backendów naraz, np. Debian z pakietem `rpm` (ma własną, pustą bazę).
- Backend bez dziennika usunięć (xbps, apk, Nix, Gentoo bez dostępu do `emerge.log`) **nie
  produkuje faktu 1**. Jego katalogi trafiają do „nieznane” i nigdy nie są proponowane. Zgodne z
  zasadą „precise over aggressive”.
- Wszystkie odczyty są plikowe. Wyjątki, z przełącznikiem jak `--no-files-db`:
  - jeden `rpm -qa --qf …` na rodzinie rpm, kiedy parsowanie blobów nagłówków rpm nie jest
    jeszcze gotowe albo baza jest w formacie ndb;
  - opcjonalnie jeden `journalctl` dla historii Flatpaka.
- Gdy blokada menedżera jest aktywna, skan pomija sekcję pozostałości i pokazuje komunikat
  „menedżer pakietów pracuje”. Baza może być wtedy w połowie zapisu.

---

## 1. dpkg / apt (Debian i pochodne)

### 1.1 `/var/log/dpkg.log` (+ `dpkg.log.1`, `dpkg.log.N.gz`): główne źródło faktu 1

- **Format** [Ź dpkg(1) `--log`]. Cztery typy linii:
  ```
  YYYY-MM-DD HH:MM:SS startup archives|packages <polecenie>
  YYYY-MM-DD HH:MM:SS status <stan> <pkg>[:arch] <wersja-zainstalowana>
  YYYY-MM-DD HH:MM:SS <akcja> <pkg>[:arch] <wersja-zainst> <wersja-dostępna>   # install|upgrade|remove|purge|configure|trigproc
  YYYY-MM-DD HH:MM:SS conffile <plik> install|keep
  ```
  Przykład [NZ, kształt zgodny z man]:
  ```
  2026-10-21 18:02:11 remove discord:amd64 0.0.70 <none>
  2026-10-21 18:02:11 status half-configured discord:amd64 0.0.70
  2026-10-21 18:02:12 status config-files discord:amd64 0.0.70
  2026-10-21 18:02:12 status not-installed discord:amd64 <none>
  ```
- **Semantyka usunięcia** [Ź `src/main/remove.c`]:
  - `dpkg -r` loguje akcję `remove` i kończy na statusie `config-files` (to „rc”);
  - `--purge` loguje `purge` i kończy na `not-installed`;
  - pakiet bez conffiles i bez postrm przechodzi od razu do purge (`not-installed`).
- **Reguła parsera**: ostatni `status installed` dla danego pkg to instalacja, a ostatni
  `status config-files|not-installed` to usunięcie. Dowolny późniejszy `status installed`
  kasuje usunięcie, tak jak `installed`/`upgraded` w pacmanie. Nazwy: obciąć `:arch`.
  Uwaga: przy upgrade też pojawia się `status half-installed`/`unpacked`. Nie należy ich
  traktować jako usunięcia.
- **Rotacja** [Ź `debian/dpkg.logrotate`]: `monthly`, `rotate 12`, `compress`,
  `delaycompress`, `create 644 root root`. Historia sięga więc najwyżej około 12 miesięcy;
  `dpkg.log.1` jest jawnym tekstem, starsze to `.gz` (potrzebny `flate2`).
- **Wiarygodność**: wysoka. Zapisuje każde wywołanie dpkg, także spod apt, aptitude,
  PackageKit, unattended-upgrades i Synaptic.
- **Koszt**: około 1–5 MB tekstu po rozpakowaniu, liniowe parsowanie, poniżej 100 ms [NZ].
- **Root**: nie (0644).

### 1.2 `/var/log/apt/history.log` (+ `.N.gz`): uzupełnienie

- **Format** [Ź `apt-pkg/deb/dpkgpm.cc`]. Bloki rozdzielone pustą linią:
  ```
  Start-Date: 2026-10-21  18:02:05
  Commandline: apt remove discord
  Requested-By: anon (1000)
  Remove: discord:amd64 (0.0.70), libfoo:amd64 (1.2-3)
  End-Date: 2026-10-21  18:02:14
  ```
  Klucze: `Install`, `Reinstall`, `Upgrade`, `Downgrade`, `Remove`, `Purge`, opcjonalnie
  `Error`. Lista ma postać `name:arch (wersja[, automatic])`, a elementy oddziela `), `.
- **Prawa**: apt robi `chmod 0644` na history.log [Ź]; `term.log` ma 0640. Rotacja
  `monthly`, `rotate 12` [Ź `debian/apt.logrotate`].
- **Wartość**: daje pole `Requested-By` i rozróżnia usunięcie przez użytkownika od
  autoremove (`Commandline: apt autoremove`). Nie widzi operacji wykonanych samym `dpkg -r`.
  Rola: opcjonalna, do UI („usunięte przez apt remove”).

### 1.3 `/var/lib/dpkg/status`: fakt 2 i lista „rc”

- **Format**: deb822, stanzy rozdzielone pustą linią; pola `Package`, `Status`,
  `Architecture`, `Provides`, `Version`, `Conffiles` (wcięte linie `/etc/x md5`) itd.
- `Status: <want> <flag> <state>`, np. `install ok installed` albo `deinstall ok config-files`.
  Tu `state` przyjmuje wartości: not-installed, config-files, half-installed, unpacked,
  half-configured, triggers-awaited, triggers-pending, installed [Ź dpkg(1)].
- **Zainstalowane** to stany od `unpacked` w górę, a ostrożniej wszystko poza `not-installed`
  i `config-files`. Pakiety `config-files` (`rc` w `dpkg -l`) są **dowodem usunięcia bez
  daty**. Datę bierze się z dpkg.log, a jeśli log jest za krótki, to fakt 1 bez daty (tier
  niższy, bez reguły btime).
- `Provides: foo (= 1.0), bar`: rozbić po przecinkach i obciąć wersję w nawiasie.
- **Koszt**: plik około 2–5 MB przy około 2–3 tys. pakietów, jeden odczyt [NZ]. **Root**: nie
  (0644 [NZ, standard]).

### 1.4 `/var/lib/dpkg/info/<pkg>[:arch].list`: pliki

- Jedna ścieżka absolutna na linię, łącznie z katalogami. Dla pakietów `Multi-Arch: same`
  nazwa pliku ma sufiks `:arch`, np. `libc6:amd64.list` [NZ, znane zachowanie].
- Dla zainstalowanych daje to samo co `local/*/files` w pacmanie: `usr/bin/*`,
  `usr/share/applications/*.desktop`, `opt/*`, `etc/xdg/*`.
- **Kluczowe dla aliasów** [Ź `remove.c`]: przy `remove` (bez purge) plik `.list` **nie jest
  kasowany**. dpkg zapisuje go ponownie, zostawiając tylko to, czego nie usunął: conffiles i
  niepuste lub współdzielone katalogi. Przy `purge` `.list` jest kasowany. Dla pakietów `rc`
  daje to aliasy typu `/etc/<app>/…`, `/etc/xdg/autostart/<app>.desktop`. Binarek już tam nie
  ma.
- **Koszt**: tysiące małych plików, ale czytamy je i tak dla zainstalowanych, jak `files` w
  pacmanie. Czas równoległy rzędu 100–300 ms na SSD [NZ].

### 1.5 Pliki USUNIĘTEGO pakietu (odpowiednik `pacman -Fl`)

Kolejność według wartości:

1. **`.list` pakietów rc** (§1.4): zawsze dostępne, ale tylko conffiles i katalogi.
2. **`Contents-<arch>` w `/var/lib/apt/lists/`** [Ź apt-file(1), Debian repo format]:
   - Pobierane tylko, gdy zainstalowany jest `apt-file`. Ten pakiet dodaje cele
     `Acquire::IndexTargets::deb::Contents-deb` w `/etc/apt/apt.conf.d/50apt-file.conf`.
     Na świeżym Ubuntu, Debianie i Mincie domyślnie **brak** [NZ co do domyślnych instalacji].
   - Nazwa na dysku ma postać
     `<host>_<ścieżka>_dists_<suite>_<comp>_Contents-<arch>.lz4`. Apt przechowuje listy
     przekompresowane do lz4 [NZ: rozszerzenie sprawdzić na maszynie; może też być `.gz`].
   - Format [Ź wiki.debian.org/DebianRepository/Format]: opcjonalny nagłówek, potem
     `ścieżka-bez-wiodącego-/␣+[area/]section/name[,…]`. Ścieżka może zawierać spacje, więc
     dzielić po **ostatnim** ciągu spacji, bo nazwy pakietów spacji nie mają.
     ```
     usr/bin/discord                                     net/discord
     usr/share/applications/discord.desktop              net/discord
     ```
   - **Koszt**: dziesiątki MB po rozpakowaniu na suite. Przy strumieniowym czytaniu wystarczy
     zachować linie z prefiksami `usr/bin/`, `usr/share/applications/`, `opt/` i `etc/xdg/`
     dla nazw z zbioru removed. Wymaga crate `lz4_flex` (MIT) [NZ wybór crate]. Odczyt bez
     roota (0644). Cache klucza po mtime plików Contents, jak w pacmanie.
   - **Ograniczenie**: Contents opisuje wersje dostępne w repo. Pakiety spoza repo (`.deb` z
     sieci: Discord, Chrome, Steam) i PPA bez Contents nie mają wpisów.
3. **`/var/cache/apt/archives/*.deb`**: archiwum usuniętego pakietu, jeśli cache nie był
   czyszczony. Polecenie `apt` domyślnie usuwa pobrane `.deb` po instalacji
   (`APT::Keep-Downloaded-Packages "false"` dla binarki `apt`) [NZ]. Czytanie `.deb` oznacza
   kontener ar, a w nim `data.tar.{xz,zst,gz}`. Niski zysk, można pominąć w v0.2.
4. Pomijamy `apt-file list`: to proces na pakiet i wymaga Contents.

Rekomendacja: v0.2 = 1 + 2 (jeśli pliki istnieją). Bez Contents działają nazwa pakietu, btime i
rekordy XU, tak samo jak dla AUR.

### 1.6 Blokady (apt/dpkg działa)

- `/var/lib/dpkg/lock-frontend` (apt i frontendy, dpkg ≥ 1.19.1, zmienna
  `DPKG_FRONTEND_LOCKED` [Ź dpkg(1)]), `/var/lib/dpkg/lock` (dpkg), `/var/lib/apt/lists/lock`,
  `/var/cache/apt/archives/lock`. Wszystkie to **blokady fcntl** na stale istniejących
  plikach, więc samo istnienie pliku nic nie znaczy [NZ: ścieżki zgodnie z praktyką; man dpkg
  ich nie wymienia].
- Pliki mają zwykle 0640 root, więc zwykły użytkownik nie zrobi `F_GETLK` (wymaga
  otwarcia). **Metoda bez roota**:
  - `stat()` pliku (wymaga tylko prawa przeszukania katalogu) daje `st_dev`/`st_ino`;
  - szukamy linii w `/proc/locks` (0444, sprawdzone tu: `-r--r--r--`) z polem
    `MAJ:MIN:INODE` (MAJ:MIN szesnastkowo) równym temu plikowi;
  - ta sama metoda działa dla rpm (`.rpm.lock`), xbps (flock) i dnf5.

---

## 2. rpm (Fedora, RHEL/Alma/Rocky, Nobara, openSUSE)

### 2.1 Baza rpm

- **Ścieżka**: `%_dbpath = %{_var}/lib/rpm` w upstreamie [Ź `macros.in`]. Fedora ≥ 36
  przeniosła ją do `/usr/lib/sysimage/rpm` i zostawiła dowiązanie `/var/lib/rpm` [NZ, Fedora
  change]. openSUSE używa `/usr/lib/sysimage/rpm` (łatka `usr-lib-sysimage-rpm.patch` w
  pakiecie rpm openSUSE, widoczna w liście plików pakietu). rpm-ostree trzyma bazę w
  `/usr/share/rpm` z dowiązaniem `L /var/lib/rpm → ../../usr/share/rpm` [Ź
  `rpm-ostree-0-integration.conf`].
  Zalecenie: sprawdzać po kolei `/usr/lib/sysimage/rpm`, `/var/lib/rpm` i `/usr/share/rpm`.
- **Backendy** [Ź `lib/backend/`]: `sqlite` (`rpmdb.sqlite`), `ndb` (`Packages.db`,
  `Index.db`), `bdb_ro` (stary `Packages` z BerkeleyDB, tylko odczyt).
  - Fedora ≥ 33 i RHEL 9: sqlite [NZ: daty z changelogów];
  - RHEL 8: bdb;
  - openSUSE: ndb [NZ, nie udało się pobrać spec z src.opensuse.org z powodu wyzwania PoW].
- **Schemat sqlite** [Ź `lib/backend/sqlite.cc`]:
  - tabela główna `Packages(hnum INTEGER PRIMARY KEY AUTOINCREMENT, blob BLOB NOT NULL)`, w
    której blob to nagłówek rpm;
  - tabele indeksów nazwane po tagu, np. `Name`, `Providename`, `Basenames`, `Dirnames`,
    `Installtid`… Każda ma kolumny `(key TEXT|BLOB, hnum INTEGER → Packages, idx INTEGER)` i
    indeksy `<tabela>_key_idx`, `<tabela>_hnum_idx`.
  - Przy otwarciu RW rpm ustawia `journal_mode = WAL` i `SQLITE_FCNTL_PERSIST_WAL`, z
    komentarzem w źródle, że jest to potrzebne do obsługi nieuprzywilejowanych czytelników.
    Pliki `-wal`/`-shm` zostają więc na dysku i czytelnik bez roota otwiera bazę
    `SQLITE_OPEN_READONLY`.
  - `busy_timeout` 10 s.
- **Odczyt bez librpm**:
  - Nazwy: `SELECT key FROM Name`. Provides: `SELECT key FROM Providename`. Wystarczy to dla
    rdzenia faktu 2.
  - Pliki: `Basenames` ma tylko nazwy bazowe bez katalogu, co jest za mało precyzyjne. Pełne
    ścieżki dają tagi z bloba: `DIRNAMES` (1118), `BASENAMES` (1117), `DIRINDEXES` (1116),
    `NAME` (1000), `PROVIDENAME` (1047), `INSTALLTIME` (1008) [NZ: numery tagów z
    `rpmtag.h`, z pamięci].
  - Format nagłówka [Ź rpm.org format_header]: big-endian, `il` (liczba wpisów), `dl`
    (rozmiar danych), potem `il` × `{tag u32, type u32, offset i32, count u32}`, potem dane.
    Typy: 6 = STRING, 8 = STRING_ARRAY, 4 = INT32 itd. W bazie blob zaczyna się od `il`/`dl`
    **bez** 8-bajtowej magii [NZ, sprawdzić na fiksturze]. Dokumentacja zastrzega, że
    „właściwym” dostępem jest biblioteka, więc to rozwiązanie jest kruche, ale stabilne od
    lat.
- **Zależność**: `rusqlite` z `bundled` (MIT, SQLite to domena publiczna) dodaje około 1 MB do
  binarki. Alternatywa: **jeden** proces
  `rpm -qa --qf '[%{NAME}\t%{FILENAMES}\n]'` plus `--qf '[%{NAME}\t%{PROVIDES}\n]'`.
  Koszt: około 1–3 s dla około 2–3 tys. pakietów z listą plików [NZ, nie zmierzono, brak rpm
  na tej maszynie]. Wynik cache'ować z kluczem po mtime `rpmdb.sqlite`/`Packages.db`.
- **ndb (openSUSE)**: własny format binarny (`rpmpkg.c`, `rpmidx.c`, `rpmxdb.c`), więc tu
  zalecany jest jeden `rpm -qa --qf` (rpm jest zawsze obecny, bo menedżer z niego korzysta).
- **rpm nie prowadzi dziennika usunięć**. `INSTALLTIME` istnieje tylko dla zainstalowanych.
  Fakt 1 pochodzi z dnf albo zypp (§2.2, §3).
- **Blokada**: `%_rpmlock_path = %{_dbpath}/.rpm.lock`, blokada fcntl transakcji [Ź
  `macros.in`]. Sprawdzać przez `/proc/locks`.

### 2.2 Historia dnf4: `/var/lib/dnf/history.sqlite`

- `persistdir` domyślnie `/var/lib/dnf` [Ź dnf `const.py.in`, conf_ref], plik
  `history.sqlite` (libdnf swdb).
- **Schemat** [Ź libdnf `transaction/sql/create_tables.sql`]:
  - `trans(id, dt_begin unix, dt_end, rpmdb_version_begin/end, releasever, user_id, cmdline, state)`;
  - `item(id, item_type)`;
  - `trans_item(id, trans_id, item_id, repo_id, action, reason, state)`;
  - `rpm(item_id, name, epoch, version, release, arch)`;
  - `item_replaced_by`, `repo`, `console_output`, tabele comps.
- **Akcje** [Ź libdnf `Types.hpp`]: 1 INSTALL, 2 DOWNGRADE, 3 DOWNGRADED, 4 OBSOLETE,
  5 OBSOLETED, 6 UPGRADE, 7 UPGRADED, **8 REMOVE**, 9 REINSTALL, 10 REINSTALLED,
  11 REASON_CHANGE.
- **Zapytanie** (szkic): `SELECT r.name, MAX(t.dt_begin) … WHERE ti.action=8` oraz ostatni
  `INSTALL`/`UPGRADE`/`REINSTALL` dla tej samej nazwy. Usunięcie jest ważne, gdy nie było
  późniejszej instalacji i nazwy nie ma w rpmdb. Status `trans_item.state` powinien być OK
  (wartość enum [NZ]).
- **Uwaga**: OBSOLETED (5) to zastąpienie przez inny pakiet. Należy je traktować jak „rename”,
  nie jak usunięcie.
- **Prawa**: `dnf history` działa jako zwykły użytkownik, więc plik jest najpewniej 0644 [NZ].
- **Dziennik tekstowy**: `/var/log/dnf.rpm.log` [Ź `LOG_RPM`]. Linie
  `%Y-%m-%dT%H:%M:%S%z SUBDEBUG <Akcja>: <nevra>`, gdzie dla usunięcia akcja to `Erase` [Ź
  `dnf/transaction.py`]. Rotacja odbywa się **po rozmiarze** (`log_size`/`log_rotate`), więc
  log jest krótki. Tylko zapas.

### 2.3 Historia dnf5: `/usr/lib/sysimage/libdnf5/transaction_history.sqlite`

- `transaction_history_dir` = `system_state_dir` = `/usr/lib/sysimage/libdnf5`, plik
  `transaction_history.sqlite` [Ź dnf5 `const.hpp`, `db.cpp`, dnf5.conf(5)].
- **Schemat** [Ź dnf5 `create_tables.sql`]:
  - `trans(id, dt_begin, dt_end, …, description, …)`;
  - `trans_item(id, trans_id, item_id, repo_id, action_id, reason_id, state_id)`;
  - `rpm(item_id, name_id → pkg_name.id, epoch, version, release, arch_id → arch.id)`;
  - `pkg_name(id, name)`.
  - Słowniki zapisane w samej bazie: `trans_item_action` = 1 Install, 2 Upgrade,
    3 Downgrade, 4 Reinstall, **5 Remove**, 6 Replaced, 7 Reason Change;
    `trans_item_state` = 1 Started, 2 Ok, 3 Error.
  - **Czytać nazwy akcji z tabeli `trans_item_action`, a nie stałe**, bo to odporne na zmiany.
- Fedora 41+ (dnf5 domyślny) może mieć **obie** bazy. Stara `history.sqlite` zostaje po
  migracji z dnf4 [NZ], więc należy scalić obie.
- **Blokada dnf5**: `/run/dnf/rpmtransaction.lock` [Ź `const.hpp`]. dnf4 trzyma
  `rpmdb_lock.pid`, `metadata_lock.pid` i `download_lock.pid` w katalogach `locks/` [Ź
  `dnf/lock.py`].
- Inne frontendy: PackageKit (GNOME Software/Discover) na Fedorze używa libdnf (dnf4) lub
  libdnf5, więc trafia do tej samej historii [NZ dla wersji dnf5].

### 2.4 Pliki usuniętego pakietu (rpm-md)

- Repo rpm-md ma `repodata/*-filelists.xml.gz` (albo `.zst`), z wpisami
  `<package pkgid name arch><version/><file>/usr/bin/x</file>…`.
- dnf5: `optional_metadata_types` domyślnie `comps,updateinfo`, więc **filelists nie są
  pobierane** [Ź dnf5.conf(5)]. dnf4 (nowsze wersje): domyślnie pusta lista dodatkowych typów,
  czyli też bez filelists, chyba że polecenie ich zażąda, np. `dnf provides /ścieżka` [Ź dnf
  conf_ref].
- Cache: `/var/cache/libdnf5/<repo>-<hash>/repodata/` (dnf5, `system_cachedir`),
  `/var/cache/dnf/<repo>-<hash>/repodata/` (dnf4). Dla zwykłego użytkownika dnf5 używa
  `~/.cache/libdnf5` [Ź]. Pliki `.solv`/`.solvx` są w binarnym formacie libsolv, więc pomijamy.
- Wniosek: filelists **czytać, jeśli istnieją** (strumień gzip/zstd XML, filtr prefiksów).
  W typowej Fedorze ich nie będzie. Wtedy zostają nazwa pakietu, btime i rekordy XU.

---

## 3. zypper (openSUSE Tumbleweed / Leap / Aeon)

- **Historia**: `/var/log/zypp/history`, zmienialna przez `history.logfile` w
  `/etc/zypp/zypp.conf` [Ź openSUSE wiki Libzypp/Package_History].
  - Format: CSV z `|`, jedno zdarzenie na linię; linie zaczynające się od `#` to komentarze.
  - Akcje [Ź libzypp `HistoryLogData.cc`]: `install`, `remove`, `radd`, `rremove`, `ralias`,
    `rurl`, `command`, `patch`.
  - Pola [Ź `HistoryLogData.h`]:
    ```
    install: date|install|name|edition|arch|reqby|repoalias|checksum|userdata
    remove : date|remove |name|edition|arch|reqby|userdata
    ```
    W pliku nazwa akcji jest wyrównywana spacjami do 7 znaków (`"remove "`), więc
    przycinać pole. Data ma format `YYYY-MM-DD HH:MM:SS` [NZ format daty].
  - `install` obejmuje też upgrade, bo zypp loguje nową wersję jako install [NZ]. Reguła:
    ostatnie zdarzenie per nazwa wygrywa, tak jak w pacmanie.
- **Prawa**: [NZ]. Raport błędu openSUSE (bsc#1225962, wątek o rootless) podaje, że
  `/var/log/zypper.log` nie jest czytelny dla użytkownika. Prawa `history` trzeba sprawdzić na
  TW. Jeśli plik jest nieczytelny, fakt 1 nie istnieje i katalogi trafiają do „nieznane”,
  z komunikatem w UI.
- **Zainstalowane**: rpmdb ndb (§2.1), więc jeden `rpm -qa --qf`.
- **Filelists**: `/var/cache/zypp/raw/<alias>/repodata/*filelists.xml.*` [NZ, czy zypp
  domyślnie je trzyma]. Cache solv: `/var/cache/zypp/solv/<alias>/solv` (libsolv, pomijamy).
- **Blokada** [Ź `ZYppFactory.cc`]: plik `zypp.pid` w `lockfileDir()`, czyli `/run/zypp.pid`
  (dawniej `/var/run`, [NZ] dokładny katalog), oraz `zypp-rpm.pid`. Zawierają PID; blokada
  jest aktywna, gdy `/proc/<pid>` istnieje.
- **Aeon/MicroOS** (transactional-update): zmiany pakietów idą do nowego snapshotu, ale `/var`
  jest wspólny, więc `history` jest pisany normalnie [NZ]. Pakiet „usunięty” działa dopiero po
  restarcie, więc fakt 2 musi brać rpmdb **bieżącego** systemu.

---

## 4. Pozostałe menedżery (krótko)

| PM | Zainstalowane (plik, format) | Dziennik usunięć | Pliki | Blokada | Root |
|---|---|---|---|---|---|
| **xbps** (Void) | `/var/db/xbps/pkgdb-0.38.plist` (XML plist, słownik `pkgname → {pkgver, provides, …}`) [Ź `xbps.h.in`] | brak domyślnego pliku; xbps pisze do sysloga [NZ] | `/var/db/xbps/.<pkg>-files.plist` [NZ nazwa] | `flock` na `/var/db/xbps/lock` [Ź `pkgdb.c`] | nie |
| **apk** (Alpine) | `/lib/apk/db/installed`: bloki `P:name V:ver p:provides F:dir R:file`, pusta linia między pakietami [Ź `database.c` dla ścieżki; litery pól NZ] + `/etc/apk/world` | brak | `F:`/`R:` w `installed` | `/lib/apk/db/lock` [Ź] | nie (0644 [Ź]) |
| **portage** (Gentoo) | `/var/db/pkg/<cat>/<pf>/` z `CONTENTS` (`obj /usr/bin/x <md5> <mtime>`, `dir`, `sym a -> b <mtime>`), `PROVIDES`, `PF` [NZ format CONTENTS] | `/var/log/emerge.log`: `<epoch>:  >>> unmerge success: cat/pkg-ver` [NZ tekst]. **Tworzony 0660 portage:portage** [Ź `emergelog.py`], więc czytelny tylko dla grupy portage | brak po unmerge | `lockdir(/var/db/pkg)` → `/var/db/.pkg.portage_lockfile` [Ź `vartree.py`, `locks.py`] | log wymaga grupy portage |
| **eopkg** (Solus) | `/var/lib/eopkg/package/<name>-<ver>-<rel>/{metadata.xml,files.xml}` [NZ układ] | `/var/lib/eopkg/history/NNN_<op>.xml` z `<Package operation="remove|install|upgrade…"><Name>…</Name><Before/><After/>` [Ź `history_dir`, `history.py`; nazwy plików NZ] | brak | `lock_dir = /var/lock/subsys` [Ź], nazwa pliku [NZ] | nie [NZ] |
| **Nix** | `/run/current-system/sw/{bin,share/applications}`, `/etc/profiles/per-user/$USER`, `~/.nix-profile`, `~/.local/state/nix/profiles` | generacje profili, a nie zdarzenia usunięć | `/nix/store/*` (wszystko zostaje do GC) | — | — |

Wniosek:
- xbps, apk i Nix dostarczają tylko fakt 2. Bez faktu 1 ich użytkownicy dostaną pozostałości
  wyłącznie z Flatpaka, Snapa, rekordów XU i (dla Nix) z niczego. To akceptowalne: wolimy
  brak wyników niż zgadywanie.
- Gentoo: jeśli `emerge.log` jest czytelny (użytkownik w grupie portage), traktować jak
  pacman.log.

---

## 5. Systemy niezmienne

| System | Pakiety hosta | Fakt 1 | Uwagi |
|---|---|---|---|
| Fedora Silverblue/Kinoite, Bazzite, Aurora, Bluefin (rpm-ostree / bootc) | rpmdb w `/usr/share/rpm` obrazu (dowiązanie `/var/lib/rpm`) [Ź] | brak dziennika usunięć. Warstwy: plik origin wdrożenia, klucze `requested-packages`, `requested-base-removals` [Ź rpm-ostree administrator-handbook]. Różnica origin poprzedniego i bieżącego wdrożenia daje tylko ostatnią zmianę [NZ: czytelność `/ostree/deploy/*/deploy/*.origin` bez roota] | aplikacje użytkownika to głównie Flatpak, Distrobox/toolbox i Homebrew (`/home/linuxbrew/.linuxbrew/Cellar`, Bazzite/Bluefin) → fakt 2. Zmiany obrazu (rebase) nie przechodzą przez dnf, więc historia dnf jest pusta lub myląca |
| SteamOS 3 | pacman (`/var/lib/pacman/local` na read-only rootfs) | aktualizacje A/B podmieniają obraz **bez** pacmana, więc pacman.log nie widzi zmian obrazu [NZ] | traktować pacman jako fakt 2; fakt 1 z pacman.log tylko przy odblokowanym trybie deweloperskim. Główne źródło: Flatpak (Discover) |
| openSUSE Aeon/Kalpa | zypp + transactional-update | `/var/log/zypp/history` (§3) | jw. |
| NixOS | §4 | brak | pominąć |

Wykrywanie: `/run/ostree-booted` istnieje dla rpm-ostree/bootc [NZ];
`/etc/os-release` `ID=steamos`; `VARIANT_ID` na Fedora Atomic. Służy do komunikatów UI, nie do
wyboru backendu.

---

## 6. Flatpak

- **Instalacje** [Ź flatpak(1)]: użytkownika `~/.local/share/flatpak/`, systemowa
  `/var/lib/flatpak/`, dodatkowe w `/etc/flatpak/installations.d/*.conf`. Aplikacja jest
  zainstalowana, gdy istnieje `<inst>/app/<id>/current` (dowiązanie do aktywnej gałęzi)
  [NZ, układ katalogu, zgodny z implementacją z leftovers.rs].
- **Dane po odinstalowaniu**: `flatpak uninstall` **zostawia** `~/.var/app/<id>`. Usuwa je
  tylko `--delete-data` („Remove app data in ~/.var/app and in the permission store”) [Ź].
  To główne źródło pozostałości na Mint, SteamOS i Bazzite.
- **Dowód, że aplikacji nie ma**: brak `app/<id>` we wszystkich instalacjach i brak procesu
  (`/proc/*/root` lub `flatpak ps`). Zaimplementowane już w `flatpak_orphan`.
- **Historia usunięć**: `flatpak history` czyta **systemd journal** [Ź]. Filtruje po
  `MESSAGE_ID=c7b39b1e006b464599465e105b361485` [Ź `flatpak-utils-private.h`] i pola
  `OPERATION`, `REF`, `INSTALLATION`, `REMOTE`, `COMMIT`, `_SOURCE_REALTIME_TIMESTAMP`
  [Ź `flatpak-builtins-history.c`].
  - Instalacje `--user` logują do journala użytkownika (czytelny bez uprawnień [NZ]);
    systemowe przez flatpak-system-helper do journala systemowego (wymaga grupy
    `systemd-journal`/`wheel`/`adm` [NZ: zależy od dystrybucji]).
  - Format plików journala jest złożony, więc jeśli w ogóle, to jeden proces
    `journalctl -o json MESSAGE_ID=… OPERATION=uninstall`. Daje to datę usunięcia, czyli fakt 3
    (btime ≤ usunięcie). **Opcjonalnie**: fakt 1 dla Flatpaka już jest, bo ID jest dokładne.
- Uprawnienia portali: `~/.local/share/flatpak/db/` (permission store), osobna pozycja
  tylko do raportu (jak w LEFTOVERS §5.6).

## 7. Snap

- **Zachowanie `snap remove`** [Ź snapd `overlord/snapstate/backend/snapdata.go`, snapcraft
  docs]:
  - snapd usuwa dane rewizji oraz `common` w `~/snap/<name>` **dla wszystkich użytkowników**,
    których katalog jest dostępny, do tego `/root/snap/<name>` i `/var/snap/<name>`; na
    końcu usuwa sam katalog bazowy;
  - pomija użytkowników, których katalog snap jest niedostępny (np. NFS root_squash; [NZ]
    zaszyfrowane, niezamontowane HOME);
  - przed usunięciem robi **automatyczny snapshot** (bez `--purge`), trzymany 31 dni w
    `/var/lib/snapd/snapshots`.
  - Wniosek: `~/snap/<name>` po normalnym usunięciu **zwykle znika**. Jeśli został, to albo
    snap nadal jest zainstalowany, albo HOME był niedostępny podczas usuwania, albo
    `os.Remove` katalogu bazowego zawiódł, bo nie był pusty (snapd loguje błąd).
- **Dowód braku**: brak `/snap/<name>/` (punkty montowania, 0755) i brak
  `/var/lib/snapd/snaps/<name>_*.snap`. `state.json` jest 0600 root [NZ], więc go nie
  czytamy. Klasyczne snapy (`--classic`) mogą pisać do `~/.config/<app>`. Nazwę snapa
  dodawać do faktu 2 (zainstalowane), także z `/var/lib/snapd/desktop/applications/*.desktop`.
- **Dziennik usunięć**: brak pliku czytelnego dla użytkownika. Zmiany są w
  `/var/lib/snapd/state.json` (root). Fakt 1 dla snapów: `~/snap/<name>` bez
  `/snap/<name>` to sierota pewna z samej struktury (analogicznie do Flatpaka).
- Ubuntu instaluje Firefoxa i Thunderbirda jako snap. Należy uwzględnić `~/snap/firefox`
  jako zainstalowany profil, żeby `~/.mozilla` z czasów deb nie był brany za pozostałość bez
  sprawdzenia [NZ, ścieżka profilu po migracji].

---

## 8. Ograniczenia i niezweryfikowane założenia

- Żadna z tych dystrybucji nie była uruchomiona. Wszystko oznaczone [NZ] trzeba sprawdzić w VM
  lub kontenerze: Debian 13, Ubuntu 24.04/26.04, Fedora 43 (dnf5), openSUSE TW, Silverblue.
  Należy zebrać fikstury: `dpkg.log`, `status`, `*.list`, `history.sqlite`,
  `transaction_history.sqlite`, `rpmdb.sqlite`, `zypp/history`.
- Zwłaszcza do sprawdzenia:
  - prawa `/var/log/zypp/history`;
  - brak magii w blobie rpmdb;
  - rozszerzenie Contents w apt lists;
  - czy dnf5 zostawia starą `history.sqlite`;
  - backend rpm openSUSE (ndb);
  - czytelność origin w ostree.
- Kontenery (Distrobox/toolbox) mają własne bazy pakietów, ale **wspólny HOME**. Aplikacja
  zainstalowana w kontenerze i eksportowana (`distrobox-export`) zostawia `.desktop` w
  `~/.local/share/applications` (fakt 2 już to łapie). Nieeksportowana aplikacja z kontenera
  jest dla hosta „nieznana”, co jest bezpieczne, bo bez faktu 1 nic nie jest proponowane.
  Bazy kontenerów można dołożyć później (`~/.local/share/containers/storage/…` → rootfs
  kontenera → jego `/var/lib/dpkg/status` itd.) [NZ].
- Rotacja dzienników ogranicza historię: dpkg/apt około 12 miesięcy, dnf.rpm.log kilka MB.
  Usunięcia starsze niż log są niewidoczne, tak jak przy pacmanie. Bazy sqlite dnf mają pełną
  historię.

## 9. Źródła

- dpkg(1), format logu, stany i `DPKG_FRONTEND_LOCKED`: https://man7.org/linux/man-pages/man1/dpkg.1.html
- dpkg `src/main/remove.c` (`.list` po remove/purge): https://salsa.debian.org/dpkg-team/dpkg/-/blob/main/src/main/remove.c (mirror https://github.com/guillemj/dpkg)
- dpkg logrotate: https://salsa.debian.org/dpkg-team/dpkg/-/blob/main/debian/dpkg.logrotate
- apt history.log (`apt-pkg/deb/dpkgpm.cc`): https://salsa.debian.org/apt-team/apt/-/blob/main/apt-pkg/deb/dpkgpm.cc
- apt logrotate: https://salsa.debian.org/apt-team/apt/-/blob/main/debian/apt.logrotate
- apt-file(1): https://manpages.debian.org/unstable/apt-file/apt-file.1.en.html
- Format Contents: https://wiki.debian.org/DebianRepository/Format#A.22Contents.22_indices
- rpm sqlite backend: https://github.com/rpm-software-management/rpm/blob/master/lib/backend/sqlite.cc
- rpm backendy: https://github.com/rpm-software-management/rpm/tree/master/lib/backend
- rpm `macros.in` (`_dbpath`, `_rpmlock_path`): https://github.com/rpm-software-management/rpm/blob/master/macros.in
- Format nagłówka rpm: https://rpm-software-management.github.io/rpm/manual/format_header.html (źródło `docs/manual/format_header.md`)
- libdnf (dnf4) schemat swdb: https://github.com/rpm-software-management/libdnf/blob/dnf-4-master/libdnf/transaction/sql/create_tables.sql
- libdnf akcje: https://github.com/rpm-software-management/libdnf/blob/dnf-4-master/libdnf/transaction/Types.hpp
- dnf4 `const.py.in`, `logging.py`, `lock.py`, `transaction.py`: https://github.com/rpm-software-management/dnf/tree/master/dnf
- dnf4 conf_ref (`optional_metadata_types`, `persistdir`): https://github.com/rpm-software-management/dnf/blob/master/doc/conf_ref.rst
- dnf5 schemat historii: https://github.com/rpm-software-management/dnf5/blob/main/libdnf5/transaction/db/sql/create_tables.sql
- dnf5 `db.cpp`: https://github.com/rpm-software-management/dnf5/blob/main/libdnf5/transaction/db/db.cpp
- dnf5 `const.hpp` (ścieżki, blokada): https://github.com/rpm-software-management/dnf5/blob/main/include/libdnf5/conf/const.hpp
- dnf5.conf(5): https://github.com/rpm-software-management/dnf5/blob/main/doc/dnf5.conf.5.rst
- libzypp historia: https://github.com/openSUSE/libzypp/blob/master/zypp/zypp/HistoryLogData.h oraz `HistoryLogData.cc`
- libzypp blokada: https://github.com/openSUSE/libzypp/blob/master/zypp/zypp/ZYppFactory.cc
- openSUSE wiki: https://en.opensuse.org/Libzypp/Package_History
- Raport o prawach logów zypper: https://lists.opensuse.org/archives/list/bugs@lists.opensuse.org/thread/7237YRGFL7GO4ZZWHWQ2IYIHZQWQ6O3L
- rpm-ostree tmpfiles: https://github.com/coreos/rpm-ostree/blob/main/src/app/rpm-ostree-0-integration.conf
- rpm-ostree origin: https://github.com/coreos/rpm-ostree/blob/main/docs/administrator-handbook.md
- xbps: https://github.com/void-linux/xbps/blob/master/include/xbps.h.in, https://github.com/void-linux/xbps/blob/master/lib/pkgdb.c
- apk-tools: https://gitlab.alpinelinux.org/alpine/apk-tools/-/blob/master/src/database.c
- portage: https://github.com/gentoo/portage/blob/master/lib/_emerge/emergelog.py, https://github.com/gentoo/portage/blob/master/lib/portage/dbapi/vartree.py, https://github.com/gentoo/portage/blob/master/lib/portage/locks.py
- eopkg: https://github.com/getsolus/eopkg/blob/master/pisi/configfile.py, https://github.com/getsolus/eopkg/blob/master/pisi/history.py
- Flatpak: https://docs.flatpak.org/en/latest/flatpak-command-reference.html, https://github.com/flatpak/flatpak/blob/main/common/flatpak-utils-private.h, https://github.com/flatpak/flatpak/blob/main/app/flatpak-builtins-history.c
- Snap: https://snapcraft.io/docs/how-to-guides/manage-snaps/create-data-snapshots, https://github.com/canonical/snapd/blob/master/overlord/snapstate/backend/snapdata.go
- `/proc/locks`: proc(5) https://man7.org/linux/man-pages/man5/proc_locks.5.html (prawa 0444 sprawdzone na tej maszynie)
