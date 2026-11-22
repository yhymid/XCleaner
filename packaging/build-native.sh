#!/usr/bin/env bash
# Build native packages in containers (podman or docker):
#   dist/xcleaner_<ver>-1_amd64.deb   built on Ubuntu 24.04 (Ubuntu 24.04+, Debian 13, Mint 22, Pop!_OS)
#   dist/xcleaner-<ver>-1.x86_64.rpm  built on Fedora 41 (Fedora 41+, openSUSE Tumbleweed)
# Same files as the Arch package: binary, desktop entry, icon, license.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
engine="$(command -v podman || command -v docker || true)"
[[ -n "$engine" ]] || { echo "Need podman or docker" >&2; exit 1; }
ver="$(sed -n 's/^version = "\(.*\)"/\1/p' "$root/Cargo.toml" | head -1)"
cache="$root/build/container-cache"
mkdir -p "$cache/rustup" "$cache/cargo" "$cache/apt" "$cache/dnf" "$root/dist"
user_args=()
[[ "$(basename "$engine")" == podman ]] && user_args=(--userns=keep-id)

# Shared: rustup into the cache, release build into a per-distro target dir.
rust_build='
  if [[ ! -x /opt/cargo/bin/cargo ]]; then
    curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs \
      | sh -s -- -y --profile minimal --default-toolchain stable --no-modify-path >/dev/null
  fi
  export PATH=/opt/cargo/bin:$PATH HOME=/tmp/home
  mkdir -p "$HOME"
  cargo build --locked --release -p xcleaner \
    --config "build.rustflags=[\"--remap-path-prefix=/src=/usr/src/xcleaner\"]"
'
stage='
  st=/tmp/stage; rm -rf $st
  install -Dm755 "$CARGO_TARGET_DIR/release/xcleaner" $st/usr/bin/xcleaner
  install -Dm644 packaging/io.github.yhymid.XCleaner.desktop $st/usr/share/applications/io.github.yhymid.XCleaner.desktop
  install -Dm644 packaging/io.github.yhymid.XCleaner.svg $st/usr/share/icons/hicolor/scalable/apps/io.github.yhymid.XCleaner.svg
  install -Dm644 LICENSE $st/usr/share/licenses/xcleaner/LICENSE
'
run() {
  local image="$1" pkgcache="$2" script="$3"
  "$engine" run --rm "${user_args[@]}" --user 0 \
    -v "$root:/src:Z" -v "$cache/rustup:/opt/rustup:Z" -v "$cache/cargo:/opt/cargo:Z" \
    -v "$cache/$pkgcache" \
    -e RUSTUP_HOME=/opt/rustup -e CARGO_HOME=/opt/cargo -e VER="$ver" \
    -w /src "$image" bash -euo pipefail -c "$script
    chown -R $(id -u):$(id -g) /src/build /src/dist /opt/rustup /opt/cargo"
}

echo "== .deb (ubuntu:24.04)"
run docker.io/library/ubuntu:24.04 "apt:/var/cache/apt/archives:Z" '
  export DEBIAN_FRONTEND=noninteractive
  rm -f /etc/apt/apt.conf.d/docker-clean
  apt-get update -qq
  apt-get install -y -qq --no-install-recommends build-essential pkg-config ca-certificates curl git \
    libgtk-4-dev libadwaita-1-dev >/dev/null
  export CARGO_TARGET_DIR=/src/build/target-ubuntu24
  '"$rust_build$stage"'
  mkdir -p $st/DEBIAN
  size=$(du -sk --exclude=DEBIAN $st | cut -f1)
  cat > $st/DEBIAN/control <<EOF
Package: xcleaner
Version: $VER-1
Architecture: amd64
Maintainer: yhymid <yhymid@users.noreply.github.com>
Installed-Size: $size
Depends: libc6 (>= 2.39), libgtk-4-1 (>= 4.14), libadwaita-1-0 (>= 1.5), libglib2.0-0t64 | libglib2.0-0, libgcc-s1
Recommends: sudo
Section: utils
Priority: optional
Homepage: https://github.com/yhymid/XCleaner
Description: Fast, precise disk cleaner (GTK4)
 Caches, logs, build artifacts and leftovers of uninstalled apps, with
 proof for every item. System cleaning asks for the password in a terminal.
EOF
  dpkg-deb --root-owner-group -Zxz --build $st "dist/xcleaner_${VER}-1_amd64.deb" >/dev/null
  dpkg-deb --info "dist/xcleaner_${VER}-1_amd64.deb" | sed -n "/Package/,/Depends/p"
'

echo "== .rpm (fedora:41)"
run registry.fedoraproject.org/fedora:41 "dnf:/var/cache/libdnf5:Z" '
  dnf -y -q --setopt=keepcache=1 install gcc pkgconf-pkg-config curl git rpm-build \
    gtk4-devel libadwaita-devel >/dev/null
  export CARGO_TARGET_DIR=/src/build/target-fedora41
  '"$rust_build$stage"'
  top=/tmp/rpmbuild; rm -rf $top; mkdir -p $top/SPECS
  cat > $top/SPECS/xcleaner.spec <<EOF
Name:           xcleaner
Version:        $VER
Release:        1
Summary:        Fast, precise disk cleaner (GTK4)
License:        MIT
URL:            https://github.com/yhymid/XCleaner
Recommends:     sudo
%global debug_package %{nil}
%global __strip /bin/true

%description
Caches, logs, build artifacts and leftovers of uninstalled apps, with
proof for every item. System cleaning asks for the password in a terminal.

%install
cp -a $st/. %{buildroot}/

%files
/usr/bin/xcleaner
/usr/share/applications/io.github.yhymid.XCleaner.desktop
/usr/share/icons/hicolor/scalable/apps/io.github.yhymid.XCleaner.svg
%license /usr/share/licenses/xcleaner/LICENSE
EOF
  rpmbuild -bb --define "_topdir $top" $top/SPECS/xcleaner.spec >/dev/null
  cp $top/RPMS/x86_64/xcleaner-$VER-1.x86_64.rpm dist/
  rpm -qp --requires dist/xcleaner-$VER-1.x86_64.rpm | grep -v rpmlib | tr "\n" " "; echo
'
ls -la "$root"/dist/xcleaner_"$ver"-1_amd64.deb "$root"/dist/xcleaner-"$ver"-1.x86_64.rpm
