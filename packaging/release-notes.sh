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
- \`XCleaner-$ver-x86_64.AppImage\` — działa bez instalacji (+ \`.sha256\`)
- \`xcleaner-$ver-1-x86_64.pkg.tar.zst\` — paczka dla Arch / CachyOS (pacman)

## Install
**AppImage**
\`\`\`sh
sha256sum -c XCleaner-$ver-x86_64.AppImage.sha256
chmod +x XCleaner-$ver-x86_64.AppImage
\`\`\`
**pacman (Arch / CachyOS)**
\`\`\`sh
sudo pacman -U xcleaner-$ver-1-x86_64.pkg.tar.zst
\`\`\`
Wymaga: GTK 4.12+, libadwaita 1.6+, \`sudo\` i emulatora terminala do czyszczenia systemu,
\`pacman-contrib\` do cache pacmana, \`fuse2\` dla AppImage.

## Launch
\`\`\`sh
./XCleaner-$ver-x86_64.AppImage     # albo z menu: XCleaner (po instalacji pacmanem: xcleaner)
xcleaner --scan --details           # raport w terminalu, nic nie usuwa
\`\`\`
Uruchamiaj **bez sudo**. Hasło sudo jest potrzebne tylko przy czyszczeniu systemu —
program otworzy wtedy terminal.
$section
MD
