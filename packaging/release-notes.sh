#!/usr/bin/env bash
# Print GitHub release notes for the version in Cargo.toml:
# Download / Install / Launch + the matching CHANGELOG section.
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ver="$(sed -n 's/^version = "\(.*\)"/\1/p' "$root/Cargo.toml" | head -1)"
section="$(awk -v v="## $ver - " 'index($0, v) == 1 {on=1; next} /^## / && on {exit} on' "$root/CHANGELOG.md")"
[ -n "$section" ] || { echo "CHANGELOG.md has no '## $ver - <Name>' section" >&2; exit 1; }
cat <<MD
## Download
| Plik | Dla |
|---|---|
| \`XCleaner-$ver-x86_64.AppImage\` | każda dystrybucja z glibc 2.39+, bez instalacji |
| \`xcleaner_$ver-1_amd64.deb\` | Ubuntu 24.04+, Debian 13, Linux Mint 22, Pop!_OS |
| \`xcleaner-$ver-1.x86_64.rpm\` | Fedora 41+, openSUSE Tumbleweed |
| \`xcleaner-$ver-1-x86_64.pkg.tar.zst\` | Arch, CachyOS, EndeavourOS, Manjaro |
| \`SHA256SUMS\` | sumy kontrolne wszystkich plików |

## Install
Najpierw sprawdź pobrane pliki: \`sha256sum -c --ignore-missing SHA256SUMS\`

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
Wymaga: GTK 4.10+, libadwaita 1.5+ (paczki dociągną je same; AppImage ma je w środku),
emulatora terminala oraz \`sudo\`, \`doas\` albo \`su\` do czyszczenia systemu.
Nie działa na Debianie 12 i Ubuntu 22.04 (za stara glibc i GTK).

## Launch
\`\`\`sh
xcleaner                            # albo z menu: XCleaner (AppImage: ./XCleaner-$ver-x86_64.AppImage)
xcleaner --scan --details           # raport w terminalu, nic nie usuwa
\`\`\`
Uruchamiaj **bez sudo**. Hasło jest potrzebne tylko przy czyszczeniu systemu —
program otworzy wtedy terminal.
$section
MD
