# Raycast

These are examples, not tested: Raycast reads maquereau's data files, it does not talk to the app. The files are in `~/Library/Application Support/dev.fundrivendev.maquereau`:

- `topics.json`: `{ "list": [topic…], "next_id": n }`. A topic has `title`, `slot` (`feature`, `bug_run`, `exploration`, `tooling`), `stage` (`queued` for the backlog, `active` in its slot, `done`), `steps` (`text`, `done_at`, null until checked), `links` (`url`, `kind`), `session` (the herdr session name) and `folder`.
- `live.json`: what was last read from outside. `sessions` maps a session name to `{ "running": bool, "agents": [{ "name", "status" }] }`, where `status` is herdr's: `idle`, `working`, `blocked`, `done` or `unknown`.

Both are rewritten whole on every change, so a reader never sees half a file.

## Script commands

[Script commands](https://github.com/raycast/script-commands) are the lightest way: shell scripts with a comment header. The three in [`raycast/`](raycast) need `jq` (`brew install jq`):

| Script | Mode | What it does |
| --- | --- | --- |
| [`maquereau-slots.sh`](raycast/maquereau-slots.sh) | inline, every 5 minutes | The topic in each slot, with its next step |
| [`maquereau-attention.sh`](raycast/maquereau-attention.sh) | inline, every minute | The topics whose herdr session has a blocked agent |
| [`maquereau-session.sh`](raycast/maquereau-session.sh) | silent, with a slot dropdown | Opens the herdr session of the topic in that slot, in Ghostty |

To use them, copy them to a folder and add that folder as a script command directory in Raycast's settings; the [script-commands README](https://github.com/raycast/script-commands) covers the setup and how inline commands show their output.

`maquereau-session.sh` runs what the app runs on `s`, with Ghostty: a login shell (so `herdr` is on the PATH) that goes to the topic's folder and starts or attaches to its session. For Terminal, replace the `open` line with the `osascript` form in `src-tauri/src/herdr.rs`.

## A full extension

A [Raycast extension](https://developers.raycast.com) (TypeScript and React) could list every topic with a `List`, filter by slot, and show the next step and session state as accessories:

```tsx
import { Action, ActionPanel, Icon, List } from "@raycast/api";
import { execFile } from "node:child_process";
import { readFileSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";

const data = join(homedir(), "Library/Application Support/dev.fundrivendev.maquereau");

interface Topic {
  id: number;
  title: string;
  slot: "feature" | "bug_run" | "exploration" | "tooling";
  stage: "queued" | "active" | "done";
  steps: { text: string; done_at: number | null }[];
  links: { url: string }[];
  session: string;
  folder: string;
}

const slotLabel = { feature: "Feature", bug_run: "Bug / Run", exploration: "Exploration", tooling: "Tooling" };

function openSession(topic: Topic) {
  const folder = topic.folder.replace(/^~/, homedir());
  execFile("open", ["-na", "Ghostty", "--args", `--command=/bin/zsh -ilc 'cd "${folder}" && exec herdr --session ${topic.session}'`]);
}

export default function Command() {
  const topics: Topic[] = JSON.parse(readFileSync(join(data, "topics.json"), "utf8")).list;
  const live = JSON.parse(readFileSync(join(data, "live.json"), "utf8"));
  const open = topics.filter((t) => t.stage !== "done").sort((a, b) => (a.stage === "active" ? -1 : 1) - (b.stage === "active" ? -1 : 1));

  return (
    <List>
      {open.map((topic) => {
        const step = topic.steps.find((s) => s.done_at === null);
        const agents: { status: string }[] = live.sessions?.[topic.session]?.agents ?? [];
        const waiting = agents.some((a) => a.status === "blocked");
        return (
          <List.Item
            key={topic.id}
            title={topic.title}
            subtitle={step?.text}
            icon={topic.stage === "active" ? Icon.Star : Icon.Circle}
            accessories={[{ tag: slotLabel[topic.slot] }, ...(waiting ? [{ icon: Icon.ExclamationMark, tooltip: "Waits for you" }] : [])]}
            actions={
              <ActionPanel>
                <Action title="Open Session" icon={Icon.Terminal} onAction={() => openSession(topic)} />
                {topic.links[0] && <Action.OpenInBrowser url={topic.links[0].url} />}
              </ActionPanel>
            }
          />
        );
      })}
    </List>
  );
}
```

Changing topics from Raycast is left out on purpose: maquereau keeps its undo history in memory and would overwrite a file changed behind its back. A deep link (`maquereau://…`) handled by the app would be the way to add that.
