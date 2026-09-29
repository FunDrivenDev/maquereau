# The only entry point for developing maquereau. Every tool comes pinned from mise.toml.

set shell := ["bash", "-euo", "pipefail", "-c"]

# Exported, so the scripts under scripts/ read them too.
export MANIFEST := "src-tauri/Cargo.toml"
export APP := "src-tauri/target/release/bundle/macos/maquereau.app"

# List the recipes.
default:
    @just --list --unsorted

alias dependencies := deps

# Install the dependencies: `install` (default), `frozen` (from the lockfiles, as in CI), `clean` (wipe, then frozen) or `update` (the tools, then the newest compatible packages, a week old at least).
deps mode="install":
    scripts/deps.sh {{ quote(mode) }}

# Format everything, fixing what the formatters can.
fmt:
    cargo fmt --manifest-path {{ MANIFEST }}
    deno fmt --quiet
    taplo fmt --colors never 2>&1 | { grep -v ' INFO ' || true; }
    rumdl fmt --quiet .
    just --fmt --unstable

# Lint everything; `family` narrows it to rust, web, toml, markdown, just, shell, workflows or spelling.
lint family="":
    scripts/lint.sh {{ quote(family) }}

# Run the Rust tests.
test:
    cargo nextest run --manifest-path {{ MANIFEST }} --locked

# Lint, then test.
check: lint test

# Run the app with hot reload of the front end, served on a free port.
dev:
    scripts/dev.sh

# Check `just dev` would start, without opening the app: the debug build, and the dev server serving the front end.
dev-check:
    scripts/dev.sh check

# Build the release app bundle, with home directory paths trimmed to `~` in the binary.
build:
    RUSTFLAGS="--remap-path-prefix=$HOME=~" deno task tauri build --bundles app
    @du -sh {{ APP }} | sed 's/\t/  /'

# Install the release app in ~/Applications and open it.
install: build
    mkdir -p ~/Applications
    rm -rf ~/Applications/maquereau.app
    cp -R {{ APP }} ~/Applications/maquereau.app
    open ~/Applications/maquereau.app

# Generate the app icons from assets/icon.svg.
icons:
    scripts/icons.sh

# What CI runs on Linux: check, build for the host without bundling, and lint the macOS code (clang compiles its Objective-C without an SDK).
ci: check
    deno task tauri build --no-bundle
    CC_aarch64_apple_darwin=clang cargo clippy --manifest-path {{ MANIFEST }} --target aarch64-apple-darwin --all-targets --locked -- -D warnings

# Run a GitHub Actions workflow locally in Docker, through act: `event` is pull_request, push or workflow_dispatch. .actrc maps the runner to an image, and mounts no Docker socket into it (which fails under colima).
act event="pull_request" workflow="ci":
    act {{ event }} --workflows .github/workflows/{{ workflow }}.yml -s GITHUB_TOKEN="$(gh auth token)"

# Audit security: secrets or personal data anywhere in the history, vulnerable or unknown-source dependencies, unsafe workflows.
audit:
    gitleaks git --config .gitleaks.toml --redact --no-banner --log-level warn .
    cargo deny --manifest-path {{ MANIFEST }} --config deny.toml check advisories sources
    deno audit
    zizmor --quiet .

# Refuse an app bundle that carries a secret or a home directory path, before it goes public.
scan-app path=APP:
    gitleaks dir --config .gitleaks.toml --redact --no-banner --log-level warn "{{ path }}"
    strings -a -n 6 "{{ path }}/Contents/MacOS/maquereau" | gitleaks stdin --config .gitleaks.toml --redact --no-banner --log-level warn

# Set `version` in Cargo.toml and its lockfile.
set-version version:
    scripts/set-version.sh {{ quote(version) }}

# Zip the release bundle as src-tauri/target/dist/maquereau-<version>-macos-arm64.zip, keeping its signature.
package version:
    scripts/package.sh {{ quote(version) }}

# Check the cask on macOS against Homebrew's style and audit rules, where deprecations fail; given an app archive, also install it from the cask into a scratch folder and check the app lost its quarantine flag.
cask archive="":
    scripts/cask.sh {{ quote(archive) }}

# Print the size of the release binary and bundle, and of the front end.
size:
    @du -sh src-tauri/target/release/maquereau {{ APP }} dist 2>/dev/null | sed 's/\t/  /' || true
