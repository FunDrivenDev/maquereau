# The only entry point for developing maquereau. Every tool comes pinned from mise.toml.

set shell := ["bash", "-euo", "pipefail", "-c"]

manifest := "src-tauri/Cargo.toml"
app := "src-tauri/target/release/bundle/macos/maquereau.app"

# List the recipes.
default:
    @just --list --unsorted

alias dependencies := deps

# Install the dependencies: `install` (default), `frozen` (from the lockfiles, as in CI), `clean` (wipe, then frozen) or `update` (the tools, then the newest compatible packages, a week old at least).
deps mode="install":
    #!/usr/bin/env bash
    set -euo pipefail
    mise install --quiet
    case "{{ mode }}" in
        install) deno install --quiet && cargo fetch --manifest-path {{ manifest }} ;;
        frozen) deno install --quiet --frozen && cargo fetch --locked --manifest-path {{ manifest }} ;;
        clean) rm -rf node_modules && cargo clean --manifest-path {{ manifest }} && deno install --quiet --frozen && cargo fetch --locked --manifest-path {{ manifest }} ;;
        update)
            # A version must be a week old, as Dependabot's cooldown, so a compromised release is
            # usually caught before it lands here. One cutoff for both registries keeps the tauri
            # crates and the @tauri-apps packages on the same release. Cargo's cutoff is still
            # unstable, hence RUSTC_BOOTSTRAP.
            cutoff=$(date -u -v-7d +%Y-%m-%dT%H:%M:%SZ 2>/dev/null || date -u -d '7 days ago' +%Y-%m-%dT%H:%M:%SZ)
            mise upgrade --local --bump --quiet --minimum-release-age 7d
            deno outdated --update --compatible --min-dep-age "$cutoff"
            deno install --quiet --min-dep-age "$cutoff"
            RUSTC_BOOTSTRAP=1 cargo -Zunstable-options generate-lockfile --quiet --manifest-path {{ manifest }} --publish-time "$cutoff"
            cargo fetch --locked --manifest-path {{ manifest }}
            ;;
        *) echo "unknown mode {{ mode }}: install, frozen, clean or update" >&2; exit 2 ;;
    esac

# Format everything, fixing what the formatters can.
fmt:
    cargo fmt --manifest-path {{ manifest }}
    deno fmt --quiet
    taplo fmt --colors never 2>&1 | { grep -v ' INFO ' || true; }
    rumdl fmt --quiet .
    just --fmt --unstable

# Lint everything; `family` narrows it to rust, web, toml, markdown, just, workflows or spelling.
lint family="":
    #!/usr/bin/env bash
    set -euo pipefail
    want() { [[ -z "{{ family }}" || "{{ family }}" == "$1" ]]; }
    case "{{ family }}" in ""|rust|web|toml|markdown|just|workflows|spelling) ;; *) echo "unknown family {{ family }}" >&2; exit 2 ;; esac
    if want rust; then
        cargo fmt --manifest-path {{ manifest }} --check
        cargo clippy --manifest-path {{ manifest }} --all-targets --locked -- -D warnings
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
    if want workflows; then actionlint; fi
    if want spelling; then typos; fi

# Run the Rust tests.
test:
    cargo nextest run --manifest-path {{ manifest }} --locked

# Lint, then test.
check: lint test

# Run the app with hot reload of the front end.
dev:
    deno task tauri dev

# Build the release app bundle, with home directory paths trimmed to `~` in the binary.
build:
    RUSTFLAGS="--remap-path-prefix=$HOME=~" deno task tauri build --bundles app
    @du -sh {{ app }} | sed 's/\t/  /'

# Install the release app in ~/Applications and open it.
install: build
    mkdir -p ~/Applications
    rm -rf ~/Applications/maquereau.app
    cp -R {{ app }} ~/Applications/maquereau.app
    open ~/Applications/maquereau.app

# Generate the app icons from assets/icon.svg.
icons:
    #!/usr/bin/env bash
    set -euo pipefail
    out=$(mktemp -d)
    trap 'rm -rf "$out"' EXIT
    log=$(deno task --quiet tauri icon assets/icon.svg --output "$out" 2>&1) || { echo "$log" >&2; exit 1; }
    mkdir -p src-tauri/icons
    for icon in 32x32.png 128x128.png 128x128@2x.png icon.icns icon.png; do cp "$out/$icon" src-tauri/icons/; done

# What CI runs on Linux: check, build for the host without bundling, and lint the macOS code (clang compiles its Objective-C without an SDK).
ci: check
    deno task tauri build --no-bundle
    CC_aarch64_apple_darwin=clang cargo clippy --manifest-path {{ manifest }} --target aarch64-apple-darwin --all-targets --locked -- -D warnings

# Run a GitHub Actions workflow locally in Docker, through act: `event` is pull_request, push or workflow_dispatch. .actrc maps the runner to an image, and mounts no Docker socket into it (which fails under colima).
act event="pull_request" workflow="ci":
    act {{ event }} --workflows .github/workflows/{{ workflow }}.yml -s GITHUB_TOKEN="$(gh auth token)"

# Audit security: secrets or personal data anywhere in the history, vulnerable or unknown-source dependencies, unsafe workflows.
audit:
    gitleaks git --config .gitleaks.toml --redact --no-banner --log-level warn .
    cargo deny --manifest-path {{ manifest }} --config deny.toml check advisories sources
    deno audit
    zizmor --quiet .

# Refuse an app bundle that carries a secret or a home directory path, before it goes public.
scan-app path=app:
    gitleaks dir --config .gitleaks.toml --redact --no-banner --log-level warn "{{ path }}"
    strings -a -n 6 "{{ path }}/Contents/MacOS/maquereau" | gitleaks stdin --config .gitleaks.toml --redact --no-banner --log-level warn

# Set `version` in Cargo.toml and its lockfile.
set-version version:
    #!/usr/bin/env bash
    set -euo pipefail
    [[ "{{ version }}" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo "{{ version }} is not a semantic version" >&2; exit 2; }
    perl -0pi -e 's/(\[package\]\nname = "maquereau"\nversion = )"[^"]*"/$1"{{ version }}"/' {{ manifest }}
    cargo update --manifest-path {{ manifest }} --workspace --quiet

# Scan the release bundle, then zip it as src-tauri/target/dist/maquereau-<version>-macos-arm64.zip and print that path.
package version: scan-app
    #!/usr/bin/env bash
    set -euo pipefail
    archive="src-tauri/target/dist/maquereau-{{ version }}-macos-arm64.zip"
    mkdir -p "$(dirname "$archive")"
    rm -f "$archive"
    ditto -c -k --keepParent {{ app }} "$archive"
    echo "$archive"

# Fallback for when the Release workflow cannot run: publish `version` from this Mac, once CI passed on the pushed main. The app is built in a temporary worktree, so no path in it names you.
publish version:
    #!/usr/bin/env bash
    set -euo pipefail
    version="{{ version }}"
    tag="v$version"
    [[ "$(git branch --show-current)" == main ]] || { echo "publish from main" >&2; exit 1; }
    [[ -z "$(git status --porcelain)" ]] || { echo "commit or stash the changes first" >&2; exit 1; }
    git fetch --quiet --tags origin main
    [[ "$(git rev-parse HEAD)" == "$(git rev-parse origin/main)" ]] || { echo "main differs from origin/main: push or pull first" >&2; exit 1; }
    if git rev-parse --quiet --verify "refs/tags/$tag" >/dev/null; then echo "$tag already exists" >&2; exit 1; fi
    ci=$(gh run list --commit "$(git rev-parse HEAD)" --workflow CI --json status,conclusion --jq '.[0] | "\(.status) \(.conclusion)"')
    [[ "$ci" == "completed success" ]] || { echo "CI has not passed on HEAD (${ci:-no run}): wait for it or fix it" >&2; exit 1; }
    just set-version "$version"
    git diff --quiet || git commit --quiet -m "Publish $version" {{ manifest }} src-tauri/Cargo.lock
    tree=$(mktemp -d)
    trap 'git worktree remove --force "$tree"' EXIT
    git worktree add --quiet --detach "$tree" HEAD
    (cd "$tree" && mise trust --quiet && just deps frozen && just build)
    archive=$(cd "$tree" && just package "$version")
    git tag -a "$tag" -m "maquereau $version"
    git push --quiet origin main "$tag"
    gh release create "$tag" --title "maquereau $version" --generate-notes "$tree/$archive"

# Print the size of the release binary and bundle, and of the front end.
size:
    @du -sh src-tauri/target/release/maquereau {{ app }} dist 2>/dev/null | sed 's/\t/  /' || true
