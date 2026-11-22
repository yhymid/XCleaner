#!/usr/bin/env bash
# Build the portable AppImage inside Ubuntu 24.04 (glibc 2.39, GTK 4.14,
# libadwaita 1.5), so it runs on every distro with glibc >= 2.39 instead of
# only on the newest ones (a CachyOS build needs glibc 2.44).
# See docs/research/DISTRO-PACKAGING-2026-11.md.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
image="${XCLEANER_BUILD_IMAGE:-docker.io/library/ubuntu:24.04}"
engine="$(command -v podman || command -v docker || true)"
[[ -n "$engine" ]] || { echo "Need podman or docker" >&2; exit 1; }

# Persist rustup, cargo registry and apt downloads between runs.
cache="$root/build/container-cache"
mkdir -p "$cache/rustup" "$cache/cargo" "$cache/apt"

user_args=()
[[ "$(basename "$engine")" == podman ]] && user_args=(--userns=keep-id)

"$engine" run --rm "${user_args[@]}" \
  -v "$root:/src:Z" \
  -v "$cache/rustup:/opt/rustup:Z" \
  -v "$cache/cargo:/opt/cargo:Z" \
  -v "$cache/apt:/var/cache/apt/archives:Z" \
  -e RUSTUP_HOME=/opt/rustup -e CARGO_HOME=/opt/cargo \
  -e HOME=/tmp/home \
  -w /src \
  --user 0 \
  "$image" bash -euo pipefail -c '
    export DEBIAN_FRONTEND=noninteractive
    rm -f /etc/apt/apt.conf.d/docker-clean
    apt-get update -qq
    apt-get install -y -qq --no-install-recommends \
      build-essential pkg-config ca-certificates curl git file python3 \
      libgtk-4-dev libadwaita-1-dev librsvg2-common gdk-pixbuf2.0-bin \
      >/dev/null
    if [[ ! -x /opt/cargo/bin/cargo ]]; then
      curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs \
        | sh -s -- -y --profile minimal --default-toolchain stable --no-modify-path >/dev/null
    fi
    export PATH=/opt/cargo/bin:$PATH
    mkdir -p "$HOME"
    git config --global --add safe.directory /src
    # Separate target dir: host objects are built against a newer glibc.
    export CARGO_TARGET_DIR=/src/build/target-ubuntu24
    ./packaging/build-appimage.sh
    chown -R '"$(id -u):$(id -g)"' /src/build /src/dist /opt/rustup /opt/cargo
  '
