# maquereau

Keeps focus on four priority topics, one per slot, with their issues, pull requests, notes, flow times and Claude sessions.

A keyboard-driven macOS app on the MOSST stack: Tauri 2 (a Rust backend and the system WebView, so no bundled browser) and a Svelte 5 front end built by Vite, all run through Deno, with every tool pinned by mise.

## Keys

| Key | Action |
| --- | --- |
| `⌘K`, `/` | Palette: fuzzy search over the items; start with `>` for commands |
| `j` `k`, `↓` `↑` | Next or previous item |
| `g` `G` | First or last item |
| `n` | Add an item |
| `r`, `↵` | Rename the item |
| `d`, `⌫` | Delete the item |
| `⌘Z`, `u` | Undo |
| `⌘⇧Z`, `U` | Redo |
| `⌘,` | Settings |
| `?` | Keyboard shortcuts |

Every search is fuzzy and every word must match; as in fzf, `'word` matches exactly, `^word` at the start, `word$` at the end, and `!word` excludes. Every action can be undone, so none asks for confirmation. Every setting lives in the Settings view; the app saves its data and settings in `~/Library/Application Support/dev.fundrivendev.maquereau`.

## Releases

Releases come from the Release workflow, started from the Actions tab or with `gh workflow run release -f version=0.2.0`. Once CI has passed on `main`, it builds the app on macOS, scans it for secrets and home directory paths, then commits the version, tags it and attaches the app to a GitHub release.
`just publish <version>` does the same from a Mac, as a fallback.

maquereau is ad-hoc signed, without an Apple Developer ID, so macOS quarantines a downloaded copy until the flag is lifted (`xattr -dr com.apple.quarantine maquereau.app`).

## Development

Every tool is pinned in `mise.toml`, and the Justfile is the entry point:

```sh
just deps      # install the tools and dependencies; `just deps update` moves them to the newest week-old versions
just dev       # run with hot reload
just check     # lint (Rust, Svelte/TypeScript, TOML, Markdown, Justfile, workflows, spelling), then test
just act       # run the CI workflow locally, in Docker
just audit     # secrets and personal data in the history, vulnerable dependencies, unsafe workflows
just install   # build the release bundle into ~/Applications and open it
just icons     # regenerate the icons from assets/icon.svg
```

Deno installs the npm packages itself; npm is never used. CI runs on `ubuntu-latest`, releases build on `macos-latest`.
