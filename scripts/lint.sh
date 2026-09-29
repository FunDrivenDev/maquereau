#!/usr/bin/env bash
# Usage: lint.sh [family]. Run through `just lint`, which sets MANIFEST.
set -euo pipefail

family=${1:-}
want() { [[ -z "$family" || "$family" == "$1" ]]; }
case "$family" in
  "" | rust | web | toml | markdown | just | shell | workflows | spelling) ;;
  *)
    echo "unknown family $family" >&2
    exit 2
    ;;
esac

if want rust; then
  cargo fmt --manifest-path "$MANIFEST" --check
  cargo clippy --manifest-path "$MANIFEST" --all-targets --locked -- -D warnings
fi
if want web; then
  deno fmt --check --quiet
  deno lint --quiet
  deno task --quiet web:check
fi
if want toml; then
  taplo fmt --colors never --check 2>&1 | { grep -v ' INFO ' || true; }
  taplo lint --colors never 2>&1 | { grep -v ' INFO ' || true; }
fi
if want markdown; then rumdl check --quiet .; fi
if want just; then just --fmt --unstable --check; fi
if want shell; then shellcheck scripts/*.sh; fi
if want workflows; then actionlint; fi
if want spelling; then typos; fi
