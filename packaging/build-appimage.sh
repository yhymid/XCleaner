#!/usr/bin/env bash
# Build the XCleaner AppImage (GTK4/libadwaita bundled via linuxdeploy).
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

: "${VERSION:=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)}"
ARCH=x86_64

for tool in curl cargo file git sha256sum python3; do
  command -v "$tool" >/dev/null || { echo "Missing build tool: $tool" >&2; exit 1; }
done

build="$root/build/appimage"
mkdir -p "$build/tools" "$root/dist"
rm -rf "$build/AppDir"
mkdir -p "$build/AppDir"
install -Dm644 LICENSE "$build/AppDir/usr/share/licenses/xcleaner/LICENSE"

# Keep the builder's HOME and checkout path out of panic messages.
export CARGO_ENCODED_RUSTFLAGS="$(python3 - "$root" <<'PY'
import os, pathlib, sys
encoded = os.environ.get('CARGO_ENCODED_RUSTFLAGS')
flags = encoded.split('\x1f') if encoded else os.environ.get('RUSTFLAGS', '').split()
flags += [f'--remap-path-prefix={pathlib.Path.home()}=/build-home',
          f'--remap-path-prefix={sys.argv[1]}=/usr/src/xcleaner']
print('\x1f'.join(f for f in flags if f), end='')
PY
)"
cargo build --locked --release -p xcleaner
test -x target/release/xcleaner

linuxdeploy="$build/tools/linuxdeploy-x86_64.AppImage"
gtk_plugin="$build/tools/linuxdeploy-plugin-gtk.sh"
# Same audited versions as XUninstaller. The "continuous" release is
# mutable, so only this digest is accepted.
linuxdeploy_expected_sha="8aea8da0f7f7039d2a2cecb14657d752a222a5e1d3825caeef186c82f751cdd1"
plugin_commit="7a3fbc31a9e5075073ff8790f26effbac5f84453"
plugin_expected_git_blob="78c5e0beb9257b0cc1e0f36bb1dec59eac271fce"

verify_linuxdeploy() { printf '%s  %s\n' "$linuxdeploy_expected_sha" "$linuxdeploy" | sha256sum --check --status; }
verify_plugin() { [[ -f "$gtk_plugin" ]] && [[ "$(git hash-object "$gtk_plugin")" == "$plugin_expected_git_blob" ]]; }

verify_linuxdeploy || curl --fail --location --retry 3 --silent --show-error -o "$linuxdeploy" \
  "https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-x86_64.AppImage"
verify_plugin || curl --fail --location --retry 3 --silent --show-error -o "$gtk_plugin" \
  "https://raw.githubusercontent.com/linuxdeploy/linuxdeploy-plugin-gtk/$plugin_commit/linuxdeploy-plugin-gtk.sh"
verify_linuxdeploy || { echo "linuxdeploy SHA-256 mismatch: refusing to run it" >&2; exit 1; }
verify_plugin || { echo "GTK plugin Git blob mismatch: refusing to run it" >&2; exit 1; }
chmod +x "$linuxdeploy" "$gtk_plugin"

# AppImageLauncher binfmt handlers can intercept build tools: run a copy with
# the AppImage magic bytes zeroed (code and payload untouched, verified first).
runtime="$(mktemp "$build/tools/linuxdeploy-runtime.XXXXXX.elf")"
trap 'rm -f "$runtime"' EXIT
python3 - "$linuxdeploy" "$runtime" "$linuxdeploy_expected_sha" <<'PY'
import hashlib, pathlib, sys
src = pathlib.Path(sys.argv[1]).read_bytes()
assert hashlib.sha256(src).hexdigest() == sys.argv[3]
assert src[8:11] == b'AI\x02'
pathlib.Path(sys.argv[2]).write_bytes(src[:8] + b'\0\0\0' + src[11:])
PY
chmod 700 "$runtime"

export APPIMAGE_EXTRACT_AND_RUN=1
export DEPLOY_GTK_VERSION=4
"$runtime" \
  --appdir "$build/AppDir" \
  --executable "$root/target/release/xcleaner" \
  --desktop-file "$root/packaging/io.github.yhymid.XCleaner.desktop" \
  --icon-file "$root/packaging/io.github.yhymid.XCleaner.svg" \
  --plugin gtk

# The plugin forces X11 and a GTK theme; libadwaita should use Wayland and its own style.
hook="$build/AppDir/apprun-hooks/linuxdeploy-plugin-gtk.sh"
test -f "$hook" || { echo "GTK AppRun hook missing" >&2; exit 1; }
sed -i '/^export GDK_BACKEND=x11/d; /^export GTK_THEME=/d' "$hook"

out="$root/dist/XCleaner-${VERSION}-${ARCH}.AppImage"
rm -f "$out"
export LDAI_OUTPUT="$out"
"$runtime" --appdir "$build/AppDir" --output appimage
test -s "$out"
chmod +x "$out"
file "$out"
(cd "$(dirname "$out")" && sha256sum "$(basename "$out")") > "$out.sha256"
echo "Built $out"
