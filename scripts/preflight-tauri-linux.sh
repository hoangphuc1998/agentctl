#!/usr/bin/env bash
set -euo pipefail

if [[ "$(uname -s)" != "Linux" ]]; then
  exit 0
fi

bundle_mode=false
for arg in "$@"; do
  if [[ "$arg" == "--bundle" ]]; then
    bundle_mode=true
  fi
done

missing_commands=()
for command in pkg-config; do
  if ! command -v "$command" >/dev/null 2>&1; then
    missing_commands+=("$command")
  fi
done

if [[ "$bundle_mode" == true ]]; then
  for command in curl wget file; do
    if ! command -v "$command" >/dev/null 2>&1; then
      missing_commands+=("$command")
    fi
  done
fi

missing=()
if command -v pkg-config >/dev/null 2>&1; then
  packages=(gdk-3.0 gdk-pixbuf-2.0 webkit2gtk-4.1 dbus-1 openssl)

  for package in "${packages[@]}"; do
    if ! pkg-config --exists "$package"; then
      missing+=("$package")
    fi
  done
fi

if ((${#missing_commands[@]} > 0 || ${#missing[@]} > 0)); then
  if ((${#missing_commands[@]} > 0)); then
    printf 'Missing Linux build commands: %s\n' "${missing_commands[*]}" >&2
  fi
  if ((${#missing[@]} > 0)); then
    printf 'Missing Tauri Linux pkg-config packages: %s\n' "${missing[*]}" >&2
  fi
  printf '\n' >&2
  cat >&2 <<'MSG'
On Ubuntu 22.04, install:
  sudo apt-get update
  sudo apt-get install -y build-essential pkg-config curl wget file libssl-dev libgtk-3-dev libgdk-pixbuf-2.0-dev libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libdbus-1-dev

Then retry:
  npm run tauri:build
MSG
  exit 1
fi
