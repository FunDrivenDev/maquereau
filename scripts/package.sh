#!/usr/bin/env bash
# Usage: package.sh <version>. Run through `just package`, which sets APP.
set -euo pipefail

archive="src-tauri/target/dist/maquereau-$1-macos-arm64.zip"
mkdir -p "$(dirname "$archive")"
rm -f "$archive"
ditto -c -k --keepParent "$APP" "$archive"
