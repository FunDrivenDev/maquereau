#!/bin/bash

# @raycast.schemaVersion 1
# @raycast.title Maquereau Session
# @raycast.mode silent
# @raycast.packageName maquereau
# @raycast.icon 🐟
# @raycast.argument1 { "type": "dropdown", "placeholder": "Slot", "data": [{"title": "Feature", "value": "feature"}, {"title": "Bug / Run", "value": "bug_run"}, {"title": "Exploration", "value": "exploration"}, {"title": "Tooling", "value": "tooling"}] }
# @raycast.description Open the herdr session of the topic in a slot, in Ghostty.

set -euo pipefail

data="$HOME/Library/Application Support/dev.fundrivendev.maquereau"

topic=$(jq -c --arg slot "$1" \
  '[.list[] | select(.stage == "active" and .slot == $slot)][0] // empty' "$data/topics.json")
if [ -z "$topic" ]; then
  echo "No topic in this slot"
  exit 1
fi

session=$(jq -r .session <<<"$topic")
folder=$(jq -r .folder <<<"$topic")
folder="${folder/#\~/$HOME}"

# The same command the app runs for Ghostty: a login shell, so herdr is on the PATH.
open -na Ghostty --args "--command=/bin/zsh -ilc 'cd $(printf %q "$folder") && exec herdr --session $session'"
echo "Opening $session"
