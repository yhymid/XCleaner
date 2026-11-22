#!/usr/bin/env bash
# Print GitHub release notes for the version in Cargo.toml:
# Download / Install / Launch + the matching CHANGELOG.md section (English;
# the Polish changelog is linked).
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ver="$(sed -n 's/^version = "\(.*\)"/\1/p' "$root/Cargo.toml" | head -1)"
section="$(awk -v v="## $ver - " 'index($0, v) == 1 {on=1; next} /^## / && on {exit} on' "$root/CHANGELOG.md")"
[ -n "$section" ] || { echo "CHANGELOG.md has no '## $ver - <Name>' section" >&2; exit 1; }
cat <<MD
## Download
| File | For |
|---|---|
| \`XCleaner-$ver-x86_64.AppImage\` | any distribution with glibc 2.39+, no installation |
| \`xcleaner_$ver-1_amd64.deb\` | Ubuntu 24.04+, Debian 13, Linux Mint 22, Pop!_OS |
| \`xcleaner-$ver-1.x86_64.rpm\` | Fedora 41+, openSUSE Tumbleweed |
| \`xcleaner-$ver-1-x86_64.pkg.tar.zst\` | Arch, CachyOS, EndeavourOS, Manjaro |
| \`SHA256SUMS\` | checksums of all files |

## Install
Check the downloaded files first: \`sha256sum -c --ignore-missing SHA256SUMS\`

**AppImage**
\`\`\`sh
chmod +x XCleaner-$ver-x86_64.AppImage
\`\`\`
**Debian / Ubuntu / Mint**
\`\`\`sh
sudo apt install ./xcleaner_$ver-1_amd64.deb
\`\`\`
**Fedora**
\`\`\`sh
sudo dnf install ./xcleaner-$ver-1.x86_64.rpm
\`\`\`
**openSUSE Tumbleweed**
\`\`\`sh
sudo zypper install --allow-unsigned-rpm ./xcleaner-$ver-1.x86_64.rpm
\`\`\`
**Arch / CachyOS**
\`\`\`sh
sudo pacman -U xcleaner-$ver-1-x86_64.pkg.tar.zst
\`\`\`
Needs GTK 4.10+, libadwaita 1.5+ (the packages pull them in; the AppImage bundles them),
a terminal emulator and \`sudo\`, \`doas\` or \`su\` for system cleaning.
Does not run on Debian 12 or Ubuntu 22.04 (glibc and GTK too old).

## Launch
\`\`\`sh
xcleaner                            # or from the menu: XCleaner (AppImage: ./XCleaner-$ver-x86_64.AppImage)
xcleaner --scan --details           # report in the terminal, deletes nothing
\`\`\`
Run it **without sudo**. The password is needed only for system cleaning —
the program then opens a terminal.
$section

Po polsku: [CHANGELOG.pl.md](https://github.com/yhymid/XCleaner/blob/v$ver/CHANGELOG.pl.md)
MD
