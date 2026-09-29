#!/usr/bin/env bash
# Usage: dev.sh [check]. Run through `just dev`, which sets MANIFEST.
#
# Serves the front end on a free port rather than a fixed one, so maquereau runs beside other
# projects' dev servers: Vite takes the port from MAQUEREAU_PORT, and Tauri is pointed at it.
#
# `check` does what `just dev` needs without opening the app, which would take the focus:
# the debug build, and Vite serving every module of the front end with the real config.
set -euo pipefail

port=$(deno eval 'const l = Deno.listen({ hostname: "127.0.0.1", port: 0 }); console.log(l.addr.port); l.close();')
export MAQUEREAU_PORT=$port
url="http://127.0.0.1:$port"

case "${1:-}" in
  "") exec deno task tauri dev --config "{\"build\":{\"devUrl\":\"$url\"}}" ;;
  check) ;;
  *)
    echo "unknown mode $1: check" >&2
    exit 2
    ;;
esac

cargo build --manifest-path "$MANIFEST" --locked --quiet

log=$(mktemp)
# In its own process group, so that stopping it stops Vite too, not only `deno task`.
set -m
deno task --quiet web:dev >"$log" 2>&1 &
server=$!
set +m
trap 'kill -- -"$server" 2>/dev/null || true; rm -f "$log"' EXIT

for _ in $(seq 150); do
  if curl -fs -o /dev/null "$url/"; then break; fi
  if ! kill -0 "$server" 2>/dev/null; then
    echo "The dev server did not start:" >&2
    cat "$log" >&2
    exit 1
  fi
  sleep 0.1
done

failed=0
while IFS= read -r file; do
  if ! curl -fsS -o /dev/null "$url/$file"; then
    echo "The dev server cannot serve $file" >&2
    failed=1
  fi
done < <(git ls-files 'src/*.ts' 'src/*.svelte' 'src/*.css')
if ((failed)); then
  cat "$log" >&2
  exit 1
fi
