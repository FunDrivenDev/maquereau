#!/bin/bash

# @raycast.schemaVersion 1
# @raycast.title Maquereau Attention
# @raycast.mode inline
# @raycast.refreshTime 1m
# @raycast.packageName maquereau
# @raycast.icon 🐟
# @raycast.description The topics whose herdr session waits for you.

set -euo pipefail

data="$HOME/Library/Application Support/dev.fundrivendev.maquereau"

jq -r --slurpfile live "$data/live.json" '
  ($live[0].sessions // {}) as $sessions
  | [.list[] | select(.stage != "done")
      | select(($sessions[.session].agents // []) | any(.status == "blocked"))
      | .title]
  | if length == 0 then "Nothing waits for you" else "● " + join(", ") end
' "$data/topics.json"
