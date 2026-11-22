# XCleaner: distributions and packaging (research for v0.2.0, 2026-11-22)

**English** | [Polski](DISTRO-PACKAGING-2026-11.pl.md)

Markers: **[measured]** is a measurement on this machine, **[source]** comes from a primary source listed at the end, and **[unverified]** is general knowledge I did not confirm.

## 0. Key points
- The current `XCleaner-0.1.0-x86_64.AppImage` needs **glibc ≥ 2.44** **[measured]**. So it runs only on Arch, CachyOS, Manjaro, EndeavourOS, Tumbleweed and Fedora Rawhide. It will not start on Ubuntu, Debian, Mint, Fedora ≤ 44 or Leap.
- The `xcleaner` binary itself needs only **glibc 2.34** (hard versions). The high version is forced by libraries built on CachyOS and bundled by the GTK plugin: `libsqlite3` (2.44), `libglib`, `libgtk-4`, `libgnutls` and `liborc` (2.43) and ~35 others (2.38) **[measured]**.
- Recommendation for v0.2.0: an AppImage built in an **Ubuntu 24.04** container (glibc 2.39, GTK 4.14, libadwaita 1.5) with the **`gtk4/v4_10` + `adw/v1_5`** features. Only `adw::Spinner` must be replaced with `gtk::Spinner`, which I checked by compiling **[measured]**. Plus `.deb` and `.rpm` with the same binary and the pacman package. No Flatpak, Snap later.

## 1. glibc: measurement and distributions

### Measurement (AppImage 0.1.0, 120 ELF files)
Commands:
```sh
./XCleaner-0.1.0-x86_64.AppImage --appimage-extract
for f in $(find squashfs-root -type f -name '*.so*') squashfs-root/usr/bin/xcleaner; do
  readelf -V "$f" | grep -oE 'Name: GLIBC_[0-9.]+  Flags: none'; done | sort -Vu | tail -3
objdump -T squashfs-root/usr/bin/xcleaner | grep -E 'GLIBC_2\.(39|4)'
```
| File | Hard glibc minimum | Symbols |
|---|---|---|
| `usr/lib/libsqlite3.so.0` (tinysparql/GTK dependency) | **2.44** | `cosh`, `sinh` |
| `libglib-2.0.so.0` | 2.43 | `free_sized`, `free_aligned_sized` |
| `libgtk-4.so.1`, `liborc-0.4.so.0` | 2.43 | `acosf`, `atan2f`, `sqrtf` |
| `libgnutls.so.30` | 2.43 | `__memset_explicit_chk` |
| ~35 libraries (adwaita, gio, pango, cairo, dbus, xkbcommon…) | 2.38 | C23 `__isoc23_*` |
| `usr/bin/xcleaner` | **2.34** | `pidfd_spawnp` and `pidfd_getpid` (2.39) are **weak** (`w`, `Flags: WEAK`), so their absence does not block startup |

Other host requirements (libraries outside the bundle, linuxdeploy excludelist): `libEGL`, `libGLX`, `libgbm`, `libdrm`, `libwayland-client`, `libX11`, `libxcb`, `libfontconfig`, `libfreetype`, `libharfbuzz`, `libfribidi`, `libgmp`, `libexpat`, `libz`, `libstdc++`, `libgcc_s`. ICU needs `GLIBCXX_3.4.30` (GCC 12). The `libopenraw_pixbuf` loader needs `GLIBCXX_3.4.32` (GCC 13), but it loads only for RAW files. Mesa and GL come from the host, and that should stay so. The extracted AppImage is 144 MB, the file 48 MB.

Extra risk **[measured/unverified]**: the bundled `gdk-pixbuf 2.44` links `libglycin-2`. Glycin runs external loaders from the host (`glycin-loaders`, through `bwrap`). On distributions without glycin 2 (Ubuntu 24.04, Debian) PNG/JPEG through gdk-pixbuf may not work. A build on Ubuntu 24.04 (gdk-pixbuf 2.42, no glycin) removes the problem.

### glibc, GTK4 and libadwaita in distributions (Repology API, 2026-11-22)
| Distribution | glibc | GTK4 | libadwaita | AppImage 0.1.0 | AppImage from Ubuntu 24.04 | Native (v4_10/v1_5) |
|---|---|---|---|---|---|---|
| Debian 12 / MX 23 | 2.36 | 4.8.3 | 1.2.2 | no | no | no |
| Debian 13 / MX 25* | 2.41 | 4.18.6 | 1.7.6 | no | yes | yes |
| Ubuntu 22.04 / Zorin 17* | 2.35 | 4.6.9 | 1.1.7 | no | no | no |
| Ubuntu 24.04, Mint 22.x*, Pop!_OS 24.04*, elementary 8*, Zorin 18* | 2.39 | 4.14.5 | 1.5.0 | no | **yes** | yes |
| Ubuntu 25.04 / 25.10 | 2.41 / 2.42 | 4.18 / 4.20 | 1.7 / 1.8 | no | yes | yes |
| Ubuntu 26.04 LTS | 2.43 | 4.22.4 | 1.9.1 | no | yes | yes |
| Fedora 41 / 42 / 43 / 44 | 2.40 / 2.41 / 2.42 / 2.43 | 4.16 / 4.18 / 4.20 / 4.22 | 1.6 / 1.7 / 1.8 / 1.9 | no | yes | yes |
| Bazzite (Fedora 43)* | 2.42 | 4.20 | 1.8 | no | yes | rpm-ostree, better AppImage |
| openSUSE Leap 15.6 | 2.38 | 4.12.4 | 1.4.2 | no | no | no (needs adw 1.5) |
| openSUSE Leap 16.0 | 2.40 [unverified] | 4.18 (GNOME 48)* | 1.7* | no | yes* | yes* |
| Tumbleweed | 2.44 | 4.24.1 | 1.10.0 | yes | yes | yes |
| Arch / Manjaro / EndeavourOS | 2.44 | 4.24.1 (Manjaro 4.22.5) | 1.10 (Manjaro 1.9.4) | yes | yes | yes (pkg) |
| SteamOS 3.7+ | ~2.41 [unverified] | from the Arch image | — | ? | yes* | no (read-only `/usr`) |

\* = version assumed from the distribution's base (Mint/Pop/elementary/Zorin 18 = Ubuntu 24.04, MX 25 = Debian 13, Leap 16 = GNOME 48). Repology does not give these values directly.

### How to build portably
| Method | Verdict |
|---|---|
| **Container with an old base** (`ubuntu:24.04` in podman/docker, linuxdeploy + GTK plugin) | **Recommended.** glibc and GTK from one consistent base. The minimum is glibc 2.39, which covers every listed distribution except Debian 12, Ubuntu 22.04/Zorin 17 and Leap 15.6. Needs going down to `v4_10`/`v1_5`. |
| `debian:13` container | GTK 4.18 and adw 1.7, code unchanged, but the glibc 2.41 minimum cuts off Ubuntu 24.04, Mint 22 and Pop. Worse. |
| `cargo-zigbuild --target x86_64-unknown-linux-gnu.2.28` | Lowers the requirements of **our binary** only. It needs 2.34 anyway, and the bundled GTK and glib are what block. Useless for the AppImage. |
| static musl | GTK4, GL and Mesa cannot sensibly be linked statically. Out. |
| sharun / "Anylinux AppImages" (pkgforge-dev) | Bundle everything including libc and do not depend on the host libc, so they also start on Debian 12 and 22.04. The cost is a bigger file and its own Mesa. Candidate for v0.3 if Debian 12 and 22.04 are needed. |
| Building GTK 4.14 and adw 1.5 from source on Debian 12 | A lot of work (meson, the whole stack). Not worth it. |

Rust and GTK4 in practice: most gtk-rs and libadwaita apps (Fractal, Loupe, Mission Center) publish mainly a Flatpak on Flathub. AppImage projects build in a container of the oldest supported base or in the sharun style **[unverified as a statistic]**.

## 2. v1_6 and v4_12 features in the code
Test on a copy of the repo in a temporary directory (repo untouched), `cargo check --offline` **[measured]**:
| Features | Result | What blocks |
|---|---|---|
| `v4_10` + `v1_5` | 1 error | `adw::Spinner` (1.6), ui.rs:165 |
| `v4_10` + `v1_5` + `gtk::Spinner::builder().spinning(true)` | **compiles** | — |
| `v4_10` + `v1_4` | 4 errors | `adw::AlertDialog` (1.5) ui.rs:380, `adw::Dialog` (1.5) ui.rs:527/601/652 |
| `v4_8` + `v1_5` | 3 errors | `gtk::FileLauncher` (4.10) ui.rs:510/770, `gtk::FileDialog` (4.10) ui.rs:614 |
| `v4_6` + `v1_2` (Debian 12) | additionally `adw::ToolbarView` (1.4) | ToolbarView, Dialog, AlertDialog, FileDialog, FileLauncher |

Conclusions:
- `v4_12` is not needed for anything; `v4_10` is enough.
- `v1_6` serves only `adw::Spinner`. Replacing it with `gtk::Spinner` gives Ubuntu 24.04+.
- Debian 12 and 22.04 would need ToolbarView replaced (Box + HeaderBar), adw::Dialog (adw::Window) and AlertDialog (MessageDialog 1.2), plus cfg-gating of FileDialog/FileLauncher (old `gtk::FileChooserNative`, `gio::AppInfo::launch_default_for_uri`). Not worth it: those systems have GTK 4.6–4.8.
- Gating through `cfg(feature)` works only at compile time. `-sys` checks the version through pkg-config: `gtk4-sys` (base 4.0.0, `v4_10` → 4.10) and `libadwaita-sys` (base 1, `v1_5` → 1.5). `adw::minor_version()` serves runtime choice, but the symbols must exist at link time anyway, so it needs one binary per level.
- A missing symbol (e.g. `adw_spinner_new` on adw 1.5) ends startup with a loader error **[unverified: binding mode, but the effect is fatal either way]**.

## 3. AppImage runtime, FUSE and AppImageLauncher
- The current AppImage **already** uses the static `type2-runtime` (static-pie, 0 NEEDED, commit `fedfad5`) **[measured]**. The `appimage` plugin of linuxdeploy (continuous) adds it. `libfuse2` is not needed **[source: type2-runtime README]**.
- The runtime looks on `$PATH` for any setuid-root `fusermount*` file, so it also works with `fusermount3`. It can be overridden with `FUSERMOUNT_PROG` (`runtime.c` `find_fusermount`) **[source]**. Ubuntu 22.04+ has `fuse3` with `fusermount3` by default **[unverified: default seed]**, so the old "install libfuse2" advice does not apply to us.
- Fallback without FUSE (containers, no module): `./XCleaner.AppImage --appimage-extract-and-run` or `APPIMAGE_EXTRACT_AND_RUN=1`. Worth describing in the Install section.
- Note: `sudo` resets the env, so the root step starts `$APPIMAGE` normally through FUSE. If the user has no FUSE, the root step fails. Suggestion: when the GUI process has `APPIMAGE_EXTRACT_AND_RUN`, pass it in the `env` prefix in `sudo_args`.
- With appimagetool: `appimagetool --runtime-file runtime-x86_64 AppDir` (from the type2-runtime releases) **[unverified: flag name]**. With linuxdeploy nothing is needed, since `--output appimage` already gives type2.
- AppImageLauncher: intercepts the start through binfmt and may move the file to `~/Applications`. We read `APPIMAGE` at runtime, so the new path works. `APPIMAGELAUNCHER_DISABLE=1` is already in the root step. The build script already bypasses binfmt (it zeroes the magic bytes of the linuxdeploy copy).
- Ubuntu 24.04 restricts unprivileged userns (AppArmor). This may affect `bwrap` in glycin. Building on 24.04 removes glycin **[unverified]**.

## 4. Native packaging
| Format | Tool | Pros | Cons | When |
|---|---|---|---|---|
| `.deb` | `cargo-deb` (`depends = "$auto"` → dpkg-shlibdeps) | one package for Ubuntu 24.04+, Debian 13, Mint 22, Pop, elementary, Zorin 18 and MX 25 | must be built in `ubuntu:24.04` so the dependencies come out as `>= 4.14/1.5` | **v0.2.0** |
| `.rpm` | `cargo-generate-rpm` (`auto-req`, dependencies by sonames `libgtk-4.so.1()(64bit)`) | one package for Fedora and openSUSE (different package names, shared sonames) | Leap 15.6 is out (adw 1.4); unsigned | **v0.2.0** |
| `.pkg.tar.zst` | `makepkg` | already exists | — | v0.2.0 |
| AUR | PKGBUILD | `yay -S xcleaner` | needs a public repo | once the repo is public |
| Fedora COPR | `.spec` | `dnf copr enable`, auto-update | public repo, maintaining a spec | v0.3 |
| openSUSE OBS | `.spec`/`.dsc` | builds deb and rpm for many distributions at once | learning OBS | v0.3 (best gain for the work) |
| Ubuntu PPA | Launchpad | `apt` with updates | builds from source without network, so crates must be vendored with `cargo vendor` | v0.3+ |
| Snap (classic) | snapcraft | full host access | classic needs manual Snap Store approval; snapd only on Ubuntu | v0.3, optional |
| Flatpak | flatpak-builder | Flathub and SteamOS | **poor fit**, see below | no |

Flatpak and XCleaner **[source: docs.flatpak.org]**:
- `--filesystem=host` does **not** give `/usr`, `/etc`, `/proc`, `/lib`. `/var` and `/tmp` must be requested separately.
- `/proc` in the sandbox is its own PID namespace, so process detection (`procs.rs`) would not work.
- `/var/lib/pacman` needs `--filesystem=/var`. `host-os` mounts `/usr` at `/run/host`, so paths would have to be rewritten.
- sudo in a terminal needs `flatpak-spawn --host`, i.e. `--talk-name=org.freedesktop.Flatpak`. That is practically a full sandbox escape, which Flathub usually rejects **[unverified: Flathub policy]**.
- Conclusion: Flatpak would be a reduced "HOME only" mode. Not for v0.2.0.

On SteamOS and Bazzite (read-only `/usr`) the AppImage remains.

## 5. Environment passed to host processes (sudo in a terminal and others)
The GTK plugin hook exports: `APPDIR`, `GTK_DATA_PREFIX`, `XDG_DATA_DIRS` (with `$APPDIR/usr/share` appended), `GSETTINGS_SCHEMA_DIR`, `GI_TYPELIB_PATH`, `GTK_EXE_PREFIX`, `GTK_PATH` and `GDK_PIXBUF_MODULE_FILE`. The runtime adds `APPIMAGE`, `APPDIR`, `ARGV0`, `OWD` and `FUSERMOUNT_PROG`. `LD_LIBRARY_PATH` is **not** set, because libraries load through `RUNPATH $ORIGIN/../lib` **[measured]**.

`FOREIGN_ENV` (root.rs:734) removes when starting the terminal: LD_*, GTK_PATH, GTK_EXE_PREFIX, GTK_DATA_PREFIX, GIO_MODULE_DIR, GIO_EXTRA_MODULES, GDK_PIXBUF_*, GSETTINGS_SCHEMA_DIR, PYTHON*, PERLLIB and QT_PLUGIN_PATH.

What still leaks:
| Variable | Where | Effect | Suggestion |
|---|---|---|---|
| `GI_TYPELIB_PATH` | the terminal and its children | Python-GI or GJS tools started in that terminal load typelibs from the AppImage, risking an ABI mismatch | add to `FOREIGN_ENV` |
| `XDG_DATA_DIRS` | terminal | points to a mount point that disappears when XCleaner closes; icons and schemas from the AppImage are visible on the host | restore the original (save it before the hook, e.g. `XCLEANER_ORIG_XDG_DATA_DIRS`) or remove the `$APPDIR` prefix |
| `APPDIR`, `APPIMAGE`, `ARGV0`, `OWD`, `FUSERMOUNT_PROG` | terminal | some programs (other AppImages, scripts) treat `APPDIR`/`APPIMAGE` as "I am in an AppImage" | remove (the GUI reads `APPIMAGE` before spawning) |
| `GIO_LAUNCHED_DESKTOP_FILE*` | — | GIO sets it only when launching through GAppInfo | no action |
| **all of the above + GTK_PATH/GDK_PIXBUF/GSETTINGS** | `gtk::FileLauncher` ("Open in the file manager") | without the GTK portal it starts the manager through GIO with the **full** env. Nautilus is DBusActivatable, so safe. Thunar, Nemo and PCManFM get our `GDK_PIXBUF_MODULE_FILE` and `GTK_PATH`, risking loader crashes **[unverified in practice]** | instead of `FileLauncher` use `gio::AppInfo::launch_default_for_uri` with an own `gdk::AppLaunchContext` (`display.app_launch_context()` + `unsetenv(...)` for these variables) **[unverified: whether FileLauncher allows injecting a context; it seems not]** |
| `pacman`, `paccache`, `pacman-conf` | GUI children | C tools without GTK, so no effect | no action |

Root step: `sudo` (env_reset) clears the env, and we pass only `APPIMAGELAUNCHER_DISABLE=1`. The AppRun hook still runs as root too and calls `dbus-send` and `gsettings` with root rights. This is harmless (at worst it fails) but unnecessary. It can be solved for v0.3, e.g. with an own AppRun that skips the hook for `--root`.

## 6. Plan for v0.2.0
1. `Cargo.toml`: `gtk4 features = ["v4_10"]`, `adw features = ["v1_5"]`; `adw::Spinner` → `gtk::Spinner` (checked by compiling).
2. `packaging/build-appimage.sh` run in `podman run --rm -v "$PWD":/src ubuntu:24.04`. Inside: `apt install build-essential curl git file python3 libgtk-4-dev libadwaita-1-dev librsvg2-dev` and rustup (rust-version 1.85). After the build check: `readelf -V … | grep 'GLIBC_2\.\(4[0-9]\)'` must give an empty result.
3. In the same container `cargo deb` and `cargo generate-rpm`, both with automatic dependencies.
4. v0.2.0 release assets:
   - `XCleaner-0.2.0-x86_64.AppImage` + `.sha256` (glibc ≥ 2.39);
   - `xcleaner_0.2.0-1_amd64.deb`;
   - `xcleaner-0.2.0-1.x86_64.rpm`;
   - `xcleaner-0.2.0-1-x86_64.pkg.tar.zst`;
   - a shared `SHA256SUMS`.
5. Add the variables from section 5 to `FOREIGN_ENV` and describe `--appimage-extract-and-run` in Install.
6. For v0.3: virtual machines (Ubuntu 24.04, Debian 13, Fedora 43, Leap 16, Tumbleweed), OBS/COPR, a decision on sharun for Debian 12 and Ubuntu 22.04.

## Sources
- Repology API (glibc, gtk, libadwaita, libadwaita-1), fetched 2026-11-22: https://repology.org/api/v1/project/glibc , https://repology.org/api/v1/project/gtk , https://repology.org/api/v1/project/libadwaita , https://repology.org/api/v1/project/libadwaita-1
- type2-runtime: https://github.com/AppImage/type2-runtime (README: "libfuse2 is no longer required"), `src/runtime/runtime.c` (`find_fusermount`, `FUSERMOUNT_PROG`, `APPIMAGE_EXTRACT_AND_RUN`, `setenv APPIMAGE/APPDIR/ARGV0/OWD`)
- linuxdeploy-plugin-gtk (commit 7a3fbc31): https://github.com/linuxdeploy/linuxdeploy-plugin-gtk
- Flatpak sandbox permissions: https://docs.flatpak.org/en/latest/sandbox-permissions.html
- Anylinux AppImages (sharun): https://github.com/pkgforge-dev/Anylinux-AppImages
- openSUSE Leap 16.0 (GNOME 48): https://itsfoss.com/news/opensuse-leap-16-release , https://doc.opensuse.org/release-notes/x86_64/openSUSE/Leap/16.0/
- gtk4-sys 0.11.5 / libadwaita-sys 0.9.2 `Cargo.toml` (`package.metadata.system-deps`) in `~/.cargo/registry`
- cargo-deb: https://github.com/kornelski/cargo-deb ; cargo-generate-rpm: https://github.com/cat-in-136/cargo-generate-rpm ; cargo-zigbuild: https://github.com/rust-cross/cargo-zigbuild (knowledge of the flags **[unverified in this session]**)
