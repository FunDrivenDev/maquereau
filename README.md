# maquereau

Keeps focus on four priority topics, one per slot, with their issues, pull requests, notes, flow times and Claude sessions.

A keyboard-driven macOS app on the MOSST stack: Tauri 2 (a Rust backend and the system WebView, so no bundled browser) and a Svelte 5 front end built by Vite, all run through Deno, with every tool pinned by mise.

## Concepts

- **Topic**: one piece of work worth focusing on. It gathers what lives elsewhere (Linear issues, pull requests, GitHub issues, Slack threads, any URL) with what lives only here: its steps, short notes, blocks and rework.
- **Slots**: four, one topic each: **Feature**, **Bug / Run**, **Exploration** and **Tooling**. The focus view shows them side by side, each with its next step, its session state and the state of its links.
- **Backlog** (`b`): hidden from the focus view. It holds the parked topics, the Linear issues assigned to you that no open topic links, and the done topics. `1`–`4` on any of them puts it in that slot, parking what was there.
- **Flow times**: lead time runs from when a topic is written down, cycle time from when it first takes its slot, both until it is done. Blocks (`!`) count as blocked time, and rework (`R`) records what came back. `t` shows the medians and 85th percentile per slot over a window set in Settings.
- **Initiative** (`i`): a topic can belong to a Linear initiative; the one its issues belong to is suggested first.
- **Session** (`s`): every topic has a [herdr](https://herdr.dev) session, named after it (`mq-<title>`) and started in its folder. `s` opens a terminal (Ghostty or Terminal, in Settings) attached to it, starting it if needed.

## Sources

- **herdr** is read with `herdr --session <name> agent list` every few seconds. When an agent turns *blocked* or *done*, a notification says so, and the slot shows it in red or green.
- **Linear** needs a personal API key (in Linear's settings, under *Personal API keys*). Paste it in Settings (`⌘,`, *Linear API key*): it goes to the macOS keychain, never to a file. It brings the assigned issues, the state and comments of linked issues, and the open initiatives.
- **GitHub** goes through the `gh` CLI, logged in as you, for the state, reviews and comments of linked pull requests and issues.
- **Slack** links are kept as links: opened with `o`, not read.

Linear and GitHub are read every few minutes, and at once with `.`; a comment by someone else on a linked issue or pull request brings a notification. Every interval and notification is a setting.

## Keys

| Key | Action |
| --- | --- |
| `⌘K`, `/` | Palette: fuzzy search over the topics; start with `>` for commands |
| `1`–`4` | Go to a slot; in the backlog, put the selected topic or issue in it |
| `j` `k`, `↓` `↑`, `g` `G` | Next, previous, first, last |
| `↵` | Walk the topic's steps, links and notes; on a step, check it; on a link, open it |
| `Esc` | Back |
| `b` | Backlog |
| `s` | Open the topic's herdr session |
| `n` | New topic |
| `a`, `x` | Add a step, check the next one |
| `J` `K` | Move the selected step down or up |
| `w` | Write a note |
| `l` | Link an issue, pull request or thread (a Linear key such as `ENG-12` is enough) |
| `o` | Open the link |
| `i` | Link to a Linear initiative |
| `!` | Mark blocked, or unblocked |
| `R` | Record rework (reopens a done topic) |
| `f`, `p`, `m` | Finish, park, move to another slot |
| `r` | Rename |
| `c`, `C` | Change the session's folder, or its name |
| `d`, `⌫` | Delete the selected row, or the topic |
| `t` | Flow stats |
| `.` | Read Linear, GitHub and herdr now |
| `⌘Z`, `u` | Undo |
| `⌘⇧Z`, `U` | Redo |
| `⌘,` | Settings |
| `?` | Keyboard shortcuts |

Every search is fuzzy and every word must match; as in fzf, `'word` matches exactly, `^word` at the start, `word$` at the end, and `!word` excludes. Every action can be undone, so none asks for confirmation. Every setting lives in the Settings view; the app saves its data and settings in `~/Library/Application Support/dev.fundrivendev.maquereau`.

[Raycast](docs/raycast.md) can list the slots and open their sessions, from that data.

## Install

```sh
brew install fundrivendev/tap/maquereau
```

## Releases

Every merge to `main` is released once CI passes on it. The Release workflow waits 5 minutes and releases only `main`'s head, so pull requests merged close together ship as one release. It bumps the patch version, or the minor or major one when a pull request in the release carries the `minor` or `major` label. It builds the app on macOS and installs it there from the cask, then, on Linux, scans it for secrets and home directory paths, tags the commit and releases it. It then copies the app to [FunDrivenDev/homebrew-tap](https://github.com/FunDrivenDev/homebrew-tap) and updates its cask, so `brew upgrade` picks it up. The tag and the cask commit are signed as Fun Driven Stuff <stuff@fundriven.dev>. If a release fails, "Re-run failed jobs" finishes it.

maquereau is ad-hoc signed, without an Apple Developer ID; the cask lifts the quarantine flag macOS puts on the download.

## Development

Every tool is pinned in `mise.toml`, and the Justfile is the entry point:

```sh
just deps      # install the tools and dependencies; `just deps update` moves them to the newest week-old versions
just dev       # run with hot reload
just check     # lint (Rust, Svelte/TypeScript, TOML, Markdown, Justfile, shell, workflows, spelling), then test
just act       # run the CI workflow locally, in Docker
just audit     # secrets and personal data in the history, vulnerable dependencies, unsafe workflows
just install   # build the release bundle into ~/Applications and open it
just icons     # regenerate the icons from assets/icon.svg
just cask      # check the cask against Homebrew's rules (macOS)
```

Deno installs the npm packages itself; npm is never used. CI runs on `ubuntu-latest`, releases build on `macos-latest`.
