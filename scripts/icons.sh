#!/usr/bin/env bash
# Usage: icons.sh. Generates the app icons from assets/icon.svg.
set -euo pipefail

out=$(mktemp -d)
trap 'rm -rf "$out"' EXIT
log=$(deno task --quiet tauri icon assets/icon.svg --output "$out" 2>&1) || {
  echo "$log" >&2
  exit 1
}
mkdir -p src-tauri/icons
for icon in 32x32.png 128x128.png 128x128@2x.png icon.icns icon.png; do cp "$out/$icon" src-tauri/icons/; done
