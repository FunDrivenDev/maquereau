#!/usr/bin/env bash
# Usage: deps.sh [install|frozen|clean|update]. Run through `just deps`, which sets MANIFEST.
set -euo pipefail

mode=${1:-install}
mise install --quiet
case "$mode" in
  install) deno install --quiet && cargo fetch --manifest-path "$MANIFEST" ;;
  frozen) deno install --quiet --frozen && cargo fetch --locked --manifest-path "$MANIFEST" ;;
  clean) rm -rf node_modules && cargo clean --manifest-path "$MANIFEST" && deno install --quiet --frozen && cargo fetch --locked --manifest-path "$MANIFEST" ;;
  update)
    # A version must be a week old, as Dependabot's cooldown, so a compromised release is
    # usually caught before it lands here. One cutoff for both registries keeps the tauri
    # crates and the @tauri-apps packages on the same release. Cargo's cutoff is still
    # unstable, hence RUSTC_BOOTSTRAP.
    cutoff=$(date -u -v-7d +%Y-%m-%dT%H:%M:%SZ 2>/dev/null || date -u -d '7 days ago' +%Y-%m-%dT%H:%M:%SZ)
    mise upgrade --local --bump --quiet --minimum-release-age 7d
    deno outdated --update --compatible --min-dep-age "$cutoff"
    deno install --quiet --min-dep-age "$cutoff"
    RUSTC_BOOTSTRAP=1 cargo -Zunstable-options generate-lockfile --quiet --manifest-path "$MANIFEST" --publish-time "$cutoff"
    cargo fetch --locked --manifest-path "$MANIFEST"
    ;;
  *)
    echo "unknown mode $mode: install, frozen, clean or update" >&2
    exit 2
    ;;
esac
