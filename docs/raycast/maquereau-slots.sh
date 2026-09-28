#!/bin/bash

# @raycast.schemaVersion 1
# @raycast.title Maquereau Slots
# @raycast.mode inline
# @raycast.refreshTime 5m
# @raycast.packageName maquereau
# @raycast.icon 🐟
# @raycast.description The topic in each slot, with its next step.

set -euo pipefail

data="$HOME/Library/Application Support/dev.fundrivendev.maquereau"

jq -r '
  def label: {feature: "F", bug_run: "B", exploration: "E", tooling: "T"}[.];
  [.list[] | select(.stage == "active")
    | "\(.slot | label) \(.title)"
      + (([.steps[] | select(.done_at == null)][0].text // null) as $step
         | if $step then " → \($step)" else "" end)]
  | if length == 0 then "No topic in a slot" else join("  ·  ") end
' "$data/topics.json"
