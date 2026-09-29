#!/usr/bin/env bash
# Usage: set-version.sh <version>. Run through `just set-version`, which sets MANIFEST.
set -euo pipefail

version=$1
[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || {
  echo "$version is not a semantic version" >&2
  exit 2
}
VERSION=$version perl -0pi -e 's/(\[package\]\nname = "maquereau"\nversion = )"[^"]*"/$1"$ENV{VERSION}"/' "$MANIFEST"
cargo update --manifest-path "$MANIFEST" --workspace --quiet
