#!/usr/bin/env bash
# Usage: cask.sh [archive]. macOS only.
set -euo pipefail

# Developer mode turns a deprecation into an error, and lets a plain untap leave installed casks alone.
export HOMEBREW_DEVELOPER=1 HOMEBREW_NO_AUTO_UPDATE=1 HOMEBREW_NO_ENV_HINTS=1
scratch=$(mktemp -d)
# A copy of the user's tap trust, so trusting the scratch tap leaves theirs as it was.
trust="${XDG_CONFIG_HOME:-}/homebrew/trust.json"
[[ -n "${XDG_CONFIG_HOME:-}" && -f "$trust" ]] || trust=~/.homebrew/trust.json
export XDG_CONFIG_HOME="$scratch/config"
mkdir -p "$XDG_CONFIG_HOME/homebrew"
[[ ! -f "$trust" ]] || cp "$trust" "$XDG_CONFIG_HOME/homebrew/trust.json"
tap=maquereau-check/scratch
installed=""
# Never `untap --force`: it uninstalls every installed cask sharing a token with the tap, the user's maquereau included.
cleanup() {
  [[ -z "$installed" ]] || brew uninstall --cask "$tap/maquereau-check" >/dev/null || true
  brew untap "$tap" >/dev/null 2>&1 || true
  rm -rf "$scratch"
}
trap cleanup EXIT
brew tap-new --no-git "$tap" >/dev/null
brew trust --tap "$tap" >/dev/null
casks="$(brew --repo "$tap")/Casks"
mkdir -p "$casks"
cp packaging/maquereau.rb "$casks/maquereau.rb"
brew style "$tap/maquereau"
brew audit --cask --strict "$tap/maquereau"
[[ -n "${1:-}" ]] || exit 0

# Its own token, so the install never touches an installed maquereau.
archive=$(realpath "$1")
sed -e 's/^cask "maquereau"/cask "maquereau-check"/' \
  -e "s|^  sha256 .*|  sha256 \"$(shasum -a 256 "$archive" | cut -d' ' -f1)\"|" \
  -e "s|^  url .*|  url \"file://$archive\"|" \
  packaging/maquereau.rb >"$casks/maquereau-check.rb"
installed=1
brew install --cask --appdir="$scratch/Applications" "$tap/maquereau-check"
if xattr -p com.apple.quarantine "$scratch/Applications/maquereau.app" >/dev/null 2>&1; then
  echo "maquereau.app still carries the quarantine flag after install" >&2
  exit 1
fi
