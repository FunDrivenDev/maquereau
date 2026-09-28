# maquereau — working rules

## Design rules

Every change to the app keeps these four; a feature that cannot is discussed first.

- **Keyboard first.** Every action has a key. The `commands` list in `src/App.svelte` is the one place that declares them: it drives the key handler, the palette (`⌘K`, then `>`) and the help (`?`), so a new action goes there and shows up in all three. The mouse is optional; nothing needs it.
- **`⌘,` opens Settings, and every setting lives there.** A setting is a field of `Settings` in `src-tauri/src/settings.rs`, with its default, and an entry in `fields` in `src/lib/Settings.svelte`. No configuration file to edit by hand, no environment variable, no hidden flag or constant a user would want to change.
- **Search is fuzzy, everywhere.** Every search (items, commands, settings, anything new) goes through `search::fuzzy` (nucleo, with fzf's syntax), and shows the matched characters with `Highlight.svelte`.
- **Actions are undoable, not confirmed.** Every change of state is a `Change` in `src-tauri/src/model.rs` whose `apply` returns its inverse, performed through `History::perform` with a label; `⌘Z` and `⌘⇧Z` take it back and forth. No confirmation dialog: the toast says what happened and how to undo it. An effect outside the app (a file deleted, a message sent) is made reversible too: move to the Trash rather than delete, or wait until the undo window has passed.

The front end holds no state of its own: every Rust command returns a `Snapshot`, and `⌘,`, `⌘Z` and `⌘⇧Z` come from the native menu (`menu` in `src-tauri/src/lib.rs`) as `menu` events.

## Working

- Work through the Justfile: run the `just` recipes, never the tools behind them directly, and add a recipe when one is missing. Tools are pinned in `mise.toml`: never Homebrew or a global npm for them. Deno is the JavaScript runtime and package manager; npm is never used.
- Dependencies come in a week after their release at the earliest: `just deps update` (the tools, the crates and the npm packages at once), or the grouped Dependabot pull request. The tauri crates and the @tauri-apps packages stay on the same minor release.
- `just check` before every commit; `just act` runs the CI workflow locally when a workflow changes.
- Work on a branch and merge through a pull request once CI passes.

## Domain

- A **topic** gathers links (Linear issues, pull requests, GitHub issues, Slack threads, URLs) with what lives only here: steps, notes, blocks, rework, an optional Linear initiative, a herdr session and its folder. `Topic` in `src-tauri/src/model.rs`.
- Four **slots** (`Slot`): Feature, Bug / Run, Exploration, Tooling; at most one `active` topic each. Activating a topic parks the one in its slot (`Model::activation`). The **backlog** is the `queued` topics plus the Linear issues assigned to the user that no open topic links.
- **Flow times** (`src-tauri/src/flow.rs`): lead time from `created_at`, cycle time from `started_at` (the first activation, kept on reopening), both to `finished_at` or now; blocked time is the sum of the blocks.
- **Outside sources** are read, never written: herdr (`agent list`) every `session_seconds`, Linear (GraphQL, key in the keychain) and GitHub (`gh`) every `refresh_minutes`. They land in `Live`, saved to `live.json`, and are never part of the undo history. Commands run with the login shell's PATH (`shell.rs`), since a GUI app does not inherit it.
- Notifications fire on a change between two reads (`live::alerts`), never on the first read after launch.
