#!/usr/bin/env bash
# Build dist/xcleaner-<ver>-1-x86_64.pkg.tar.zst from the current checkout
# (works while the GitHub repo is private: the source tarball is made locally
# with `git archive` under the name the PKGBUILD expects).
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"
ver="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
work="$root/build/pkg"
rm -rf "$work"; mkdir -p "$work" "$root/dist"
git archive --format=tar.gz --prefix="XCleaner-$ver/" -o "$work/xcleaner-$ver.tar.gz" HEAD
sum="$(sha256sum "$work/xcleaner-$ver.tar.gz" | cut -d' ' -f1)"
sed -e "s/^pkgver=.*/pkgver=$ver/" -e "s/^sha256sums=.*/sha256sums=('$sum')/" \
  packaging/arch/PKGBUILD > "$work/PKGBUILD"
(cd "$work" && PKGDEST="$root/dist" makepkg -f --noconfirm)
ls -la "$root"/dist/xcleaner-"$ver"-*.pkg.tar.zst
