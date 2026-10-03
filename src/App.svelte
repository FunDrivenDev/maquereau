<script lang="ts">
  import { SvelteSet } from "svelte/reactivity";
  import * as api from "./lib/api";
  import { type Initiative, type Issue, type Settings, type Slot, slots, type Snapshot, type Topic } from "./lib/api";
  import { applies, type Command } from "./lib/commands";
  import Detail from "./lib/Detail.svelte";
  import { ago, isBlocked, linkStatus, linkUrl, nextStep, rows as rowsOf, sessionState, stateMark } from "./lib/format";
  import Help from "./lib/Help.svelte";
  import IssueView from "./lib/IssueView.svelte";
  import Palette from "./lib/Palette.svelte";
  import Pick from "./lib/Pick.svelte";
  import Prompt from "./lib/Prompt.svelte";
  import { reveal } from "./lib/scroll";
  import SettingsView from "./lib/Settings.svelte";
  import SlotCard from "./lib/SlotCard.svelte";
  import Stats from "./lib/Stats.svelte";

  type Overlay = "palette" | "settings" | "help" | "stats" | "prompt" | "pick";
  type View = "focus" | "backlog";
  type BacklogRow =
    | { kind: "topic"; key: string; topic: Topic }
    | { kind: "issue"; key: string; issue: Issue; depth: number; children: number; folded: boolean };

  interface Ask {
    title: string;
    value?: string;
    placeholder?: string;
    secret?: boolean;
    allowEmpty?: boolean;
    action: string;
    submit: (text: string) => void;
  }

  interface Choice {
    title: string;
    options: { label: string; detail?: string; value: unknown }[];
    pick: (value: unknown) => void;
  }

  let data = $state<Snapshot>({
    topics: [],
    times: [],
    stats: [],
    live: { links: {}, assigned: [], sub_issues: [], initiatives: [], sessions: {}, refreshed_at: null, errors: [] },
    backlog: [],
    settings: {
      theme: "system",
      search_limit: 50,
      terminal: "ghostty",
      folder: "~/Code",
      refresh_minutes: 5,
      session_seconds: 15,
      notify_sessions: true,
      notify_comments: true,
      stats_days: 90,
    },
    has_linear_key: false,
    now: 0,
    undo: null,
    redo: null,
  });
  let loaded = $state(false);
  let view = $state<View>("focus");
  let slotIndex = $state(0);
  let backlogKey = $state<string | null>(null);
  /** The keys of the backlog issues whose sub-issues are hidden. */
  const folded = new SvelteSet<string>();
  /** The detail row the keys act on, or -1 while the list has them. */
  let detailRow = $state(-1);
  let overlay = $state<Overlay | null>(null);
  let ask = $state<Ask | null>(null);
  let choice = $state<Choice | null>(null);
  let toast = $state<{ text: string; error: boolean } | null>(null);
  let list: HTMLElement | undefined = $state();
  let issuePane: HTMLElement | undefined = $state();
  let toastTimer: ReturnType<typeof setTimeout> | undefined;

  const times = $derived(new Map(data.times));
  const openTopics = $derived(data.topics.filter((t) => t.stage !== "done"));
  const activeOf = (slot: Slot) => data.topics.find((t) => t.slot === slot && t.stage === "active") ?? null;
  const slotOrder = (slot: Slot) => slots.findIndex((s) => s.slot === slot);

  const backlogRows: BacklogRow[] = $derived([
    ...data.topics
      .filter((t) => t.stage === "queued")
      .sort((a, b) => slotOrder(a.slot) - slotOrder(b.slot))
      .map((topic): BacklogRow => ({ kind: "topic", key: `t${topic.id}`, topic })),
    ...issueRows(data.backlog, folded),
    ...data.topics
      .filter((t) => t.stage === "done")
      .sort((a, b) => (b.finished_at ?? 0) - (a.finished_at ?? 0))
      .slice(0, 30)
      .map((topic): BacklogRow => ({ kind: "topic", key: `t${topic.id}`, topic })),
  ]);
  const priorityMark = ["·", "‼", "▮▮▮", "▮▮▯", "▮▯▯"];

  /** The backlog's issues as rows, but the sub-issues of a folded one. */
  function issueRows(entries: api.BacklogEntry[], folded: Set<string>): BacklogRow[] {
    const rows: BacklogRow[] = [];
    let hideBelow = Infinity;
    for (const { issue, depth, children } of entries) {
      if (depth > hideBelow) continue;
      const fold = children > 0 && folded.has(issue.key);
      hideBelow = fold ? depth : Infinity;
      rows.push({ kind: "issue", key: `i${issue.key}`, issue, depth, children, folded: fold });
    }
    return rows;
  }

  /** The sub-issues right under the backlog's issue `key`. */
  function subIssues(key: string): Issue[] {
    const at = data.backlog.findIndex((e) => e.issue.key === key);
    if (at < 0) return [];
    const depth = data.backlog[at]!.depth;
    const out: Issue[] = [];
    for (const entry of data.backlog.slice(at + 1)) {
      if (entry.depth <= depth) break;
      if (entry.depth === depth + 1) out.push(entry.issue);
    }
    return out;
  }

  const backlogIndex = $derived(backlogRows.findIndex((r) => r.key === backlogKey));
  const backlogRow = $derived(backlogRows[backlogIndex] ?? null);

  const selected: Topic | null = $derived(
    view === "focus"
      ? activeOf(slots[slotIndex]!.slot)
      : backlogRow?.kind === "topic"
      ? backlogRow.topic
      : null,
  );
  const selectedIssue = $derived(view === "backlog" && backlogRow?.kind === "issue" ? backlogRow.issue : null);
  const detailRows = $derived(selected ? rowsOf(selected) : []);
  const row = $derived(detailRow >= 0 ? detailRows[detailRow] ?? null : null);
  const attention = $derived(
    openTopics.filter((t) => sessionState(data.live.sessions[t.session]) === "blocked").length,
  );

  function show(text: string, error = false) {
    toast = { text, error };
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => (toast = null), error ? 6000 : 4000);
  }

  /** Takes a new snapshot, keeping the backlog selection or the row now in its place. */
  function apply(next: Snapshot) {
    const before = backlogIndex;
    data = next;
    loaded = true;
    if (backlogKey === null || !backlogRows.some((r) => r.key === backlogKey)) {
      backlogKey = backlogRows[Math.max(0, Math.min(before, backlogRows.length - 1))]?.key ?? null;
    }
    if (detailRow >= detailRows.length) detailRow = detailRows.length - 1;
  }

  /** Runs a change on the Rust side and says what it did, with how to take it back. */
  async function run(change: Promise<Snapshot>, done: (next: Snapshot) => string | null = undoable) {
    try {
      const next = await change;
      apply(next);
      const text = done(next);
      if (text) show(text);
    } catch (e) {
      show(String(e), true);
    }
  }

  const undoable = (next: Snapshot) => (next.undo ? `${next.undo} · ⌘Z to undo` : null);

  function outside(action: Promise<void>, text?: string) {
    action.then(() => text && show(text), (e) => show(String(e), true));
  }

  function undo() {
    const label = data.undo;
    if (!label) return show("Nothing to undo");
    run(api.undo(), () => `Undid ${label} · ⌘⇧Z to redo`);
  }

  function redo() {
    const label = data.redo;
    if (!label) return show("Nothing to redo");
    run(api.redo());
  }

  function changeSettings(settings: Settings) {
    run(api.setSettings(settings), () => null);
  }

  function prompt(next: Ask) {
    ask = next;
    overlay = "prompt";
  }

  function pickOne<T>(title: string, options: { label: string; detail?: string; value: T }[], pick: (v: T) => void) {
    choice = { title, options, pick: pick as (value: unknown) => void };
    overlay = "pick";
  }

  function pickSlot(title: string, then: (slot: Slot) => void) {
    pickOne(title, slots.map((s, i) => ({ label: s.label, detail: String(i + 1), value: s.slot })), then);
  }

  $effect(() => {
    document.documentElement.dataset.theme = data.settings.theme;
  });

  $effect(() => {
    api.snapshot().then(apply, (e) => show(String(e), true));
    const unlistenMenu = api.onMenu((id) => {
      if (id === "settings") overlay = overlay === "settings" ? null : "settings";
      else if (id === "undo") undo();
      else if (id === "redo") redo();
    });
    const unlistenSnapshot = api.onSnapshot(apply);
    return () => {
      void unlistenMenu.then((stop) => stop());
      void unlistenSnapshot.then((stop) => stop());
    };
  });

  $effect(() => {
    const key = backlogKey;
    const element = list?.querySelector<HTMLElement>(`[data-key="${key}"]`);
    if (list && element) reveal(list, element);
  });

  function move(delta: number) {
    if (detailRow >= 0) {
      const n = detailRows.length;
      if (n) detailRow = Math.max(0, Math.min(n - 1, detailRow + delta));
    } else if (view === "focus") {
      slotIndex = Math.max(0, Math.min(3, slotIndex + delta));
    } else {
      const n = backlogRows.length;
      if (!n) return;
      const i = backlogIndex < 0 ? 0 : Math.max(0, Math.min(n - 1, backlogIndex + delta));
      backlogKey = backlogRows[i]?.key ?? null;
    }
  }

  function selectSlot(slot: Slot) {
    view = "focus";
    detailRow = -1;
    slotIndex = slotOrder(slot);
  }

  /** Shows `topic` where it lives: its slot when active, else the backlog. */
  function reach(topic: Topic) {
    detailRow = -1;
    if (topic.stage === "active") return selectSlot(topic.slot);
    view = "backlog";
    backlogKey = `t${topic.id}`;
  }

  function withTopic(action: (topic: Topic) => void) {
    return () => {
      if (selected) action(selected);
      else show(view === "focus" ? "This slot is empty · n adds a topic" : "Pick a topic first");
    };
  }

  function newTopic() {
    const inFocus = view === "focus";
    const create = (slot: Slot) =>
      prompt({
        title: `New ${api.slotLabel(slot)} topic`,
        action: "add",
        submit: (title) => {
          const activate = inFocus && !activeOf(slot);
          run(api.createTopic(title, slot, activate), (next) => {
            const topic = [...next.topics].reverse().find((t) => t.title === title);
            if (topic) reach(topic);
            return activate ? undoable(next) : `Parked in the backlog · ↵ there puts it in its slot · ⌘Z to undo`;
          });
        },
      });
    if (inFocus) create(slots[slotIndex]!.slot);
    else pickSlot("New topic in", create);
  }

  function focusIssue(issue: Issue, slot: Slot) {
    run(api.topicFromIssue(issue.key, slot), (next) => {
      selectSlot(slot);
      return undoable(next);
    });
  }

  function putIn(slot: Slot) {
    if (selectedIssue) return focusIssue(selectedIssue, slot);
    if (view === "backlog" && selected) {
      const topic = selected;
      const moved = topic.slot === slot ? api.activate(topic.id) : api.moveTopic(topic.id, slot);
      return run(moved, (next) => {
        const now = next.topics.find((t) => t.id === topic.id);
        if (now?.stage !== "active") void run(api.activate(topic.id));
        selectSlot(slot);
        return undoable(next);
      });
    }
    selectSlot(slot);
  }

  function enter() {
    if (row) {
      if (row.kind === "step") return run(api.toggleStep(selected!.id, row.step.id));
      if (row.kind === "link") return openLink(row.link);
      return;
    }
    if (selectedIssue) return pickSlot(`Focus on ${selectedIssue.key} in`, (slot) => focusIssue(selectedIssue, slot));
    if (!selected) return show(view === "focus" ? "This slot is empty · n adds a topic" : "Nothing selected");
    if (view === "backlog" && selected.stage === "queued") {
      const topic = selected;
      return run(api.activate(topic.id), (next) => {
        selectSlot(topic.slot);
        return undoable(next);
      });
    }
    if (detailRows.length) detailRow = 0;
    else show("Nothing listed yet · a adds a step, l a link, w a note");
  }

  function back() {
    if (detailRow >= 0) detailRow = -1;
    else if (view === "backlog") view = "focus";
  }

  function openLink(link: Topic["links"][number]) {
    const url = linkUrl(data.live, link);
    if (!url) return show(`${link.url} has no address yet: it shows once Linear is read`, true);
    outside(api.openUrl(url));
  }

  function open() {
    if (row?.kind === "link") return openLink(row.link);
    if (selectedIssue) return outside(api.openUrl(selectedIssue.url));
    withTopic((topic) => {
      const link = topic.links[0];
      if (link) openLink(link);
      else show("No link yet · l adds one");
    })();
  }

  function doneStep() {
    withTopic((topic) => {
      const step = row?.kind === "step" ? row.step : nextStep(topic);
      if (!step) return show("No step to mark done · a adds one");
      run(api.toggleStep(topic.id, step.id));
    })();
  }

  function moveStep(delta: number) {
    if (row?.kind !== "step" || !selected) return show("Select a step first (↵, then ↓)");
    const topic = selected;
    const step = row.step;
    run(api.moveStep(topic.id, step.id, delta), (next) => {
      const moved = rowsOf(next.topics.find((t) => t.id === topic.id)!).findIndex((r) => r.id === step.id);
      if (moved >= 0) detailRow = moved;
      return undoable(next);
    });
  }

  function linkSomething() {
    if (selectedIssue) {
      const issue = selectedIssue;
      return pickOne(
        `Link ${issue.key} to`,
        openTopics.map((t) => ({ label: t.title, detail: api.slotLabel(t.slot), value: t.id })),
        (id) => run(api.addLink(id, issue.url)),
      );
    }
    withTopic((topic) =>
      prompt({
        title: `Link to “${topic.title}”`,
        placeholder: "A Linear key (BIM-123), or the URL of an issue, pull request, Slack thread…",
        action: "link",
        submit: (text) => run(api.addLink(topic.id, text)),
      })
    )();
  }

  function pickInitiative() {
    withTopic((topic) => {
      if (!data.has_linear_key) return show("Set a Linear API key first (⌘,)", true);
      const suggested = topic.links.flatMap((l) => linkStatus(data.live, l)?.initiatives ?? []);
      const seen = new Set<string>();
      const options: { label: string; detail?: string; value: Initiative | null }[] = [];
      for (const i of [...suggested, ...data.live.initiatives]) {
        if (seen.has(i.id)) continue;
        seen.add(i.id);
        options.push({ label: i.name, detail: suggested.includes(i) ? "from its issues" : undefined, value: i });
      }
      if (topic.initiative) options.unshift({ label: "No initiative", value: null });
      if (!options.length) return show("No initiative read from Linear yet · . refreshes", true);
      pickOne(`Initiative of “${topic.title}”`, options, (initiative) => run(api.setInitiative(topic.id, initiative)));
    })();
  }

  function remove() {
    if (row && selected) return run(api.removePart(selected.id, row.id));
    withTopic((topic) => run(api.remove(topic.id)))();
  }

  function editSetting(key: keyof Settings, label: string) {
    const settings = data.settings;
    prompt({
      title: label,
      value: String(settings[key]),
      action: "save",
      submit: (text) => {
        overlay = "settings";
        changeSettings({ ...settings, [key]: text });
      },
    });
  }

  function linearKey() {
    prompt({
      title: "Linear API key",
      placeholder: data.has_linear_key ? "A new key replaces the one set; empty forgets it" : "lin_api_…",
      secret: true,
      allowEmpty: true,
      action: "save in the keychain",
      submit: (key) =>
        run(api.setLinearKey(key), () => key ? "Linear key saved · reading Linear now" : "Linear key forgotten"),
    });
  }

  const onStep = () => row?.kind === "step";
  const onIssue = () => selectedIssue !== null;
  const onIssueRow = () => onIssue() && detailRow < 0;

  /** Folds the selected issue's sub-issues, or else selects its parent. */
  function fold() {
    if (backlogRow?.kind !== "issue") return;
    if (backlogRow.children && !backlogRow.folded) return folded.add(backlogRow.issue.key);
    const parent = backlogRow.issue.parent?.key;
    if (parent && backlogRows.some((r) => r.key === `i${parent}`)) backlogKey = `i${parent}`;
  }

  /** Unfolds the selected issue's sub-issues, or else selects the first one. */
  function unfold() {
    if (backlogRow?.kind !== "issue" || !backlogRow.children) return;
    if (backlogRow.folded) return folded.delete(backlogRow.issue.key);
    backlogKey = backlogRows[backlogIndex + 1]?.key ?? backlogKey;
  }

  function foldAll() {
    const parents = data.backlog.filter((e) => e.children > 0 && e.depth === 0).map((e) => e.issue.key);
    const unfold = parents.length > 0 && parents.every((key) => folded.has(key));
    if (unfold) return folded.clear();
    const row = backlogRow;
    for (const key of parents) folded.add(key);
    if (row?.kind !== "issue") return;
    let at = data.backlog.findIndex((e) => e.issue.key === row.issue.key);
    while (at > 0 && data.backlog[at]!.depth > 0) at--;
    const top = data.backlog[at];
    if (top) backlogKey = `i${top.issue.key}`;
  }

  /** Scrolls the issue shown by `pages` of its pane. */
  function scrollIssue(pages: number) {
    if (issuePane) issuePane.scrollBy({ top: pages * issuePane.clientHeight * 0.85, behavior: "smooth" });
  }

  const commands: Command[] = [
    { id: "palette", label: "Search and commands", keys: ["⌘K", "/"], run: () => (overlay = "palette") },
    {
      id: "slot",
      label: "Go to slot 1–4; in the backlog, put the selection in it",
      keys: ["1", "2", "3", "4"],
      run: (key) => {
        const slot = slots[Number(key) - 1]?.slot;
        if (slot) putIn(slot);
        else pickSlot("Go to", putIn);
      },
    },
    { id: "next", label: "Next", keys: ["j", "ArrowDown"], run: () => move(1) },
    { id: "previous", label: "Previous", keys: ["k", "ArrowUp"], run: () => move(-1) },
    { id: "first", label: "First", keys: ["g"], run: () => move(-Infinity) },
    { id: "last", label: "Last", keys: ["G"], run: () => move(Infinity) },
    {
      id: "enter",
      label: "Open the topic's list; on a row, check the step or open the link; in the backlog, focus on it",
      keys: ["Enter"],
      run: enter,
    },
    { id: "back", label: "Back", keys: ["Escape"], run: back },
    {
      id: "backlog",
      label: "Backlog: parked topics, assigned Linear issues, done topics",
      keys: ["b"],
      run: () => {
        detailRow = -1;
        view = view === "backlog" ? "focus" : "backlog";
      },
    },
    {
      id: "session",
      label: "Open the topic's herdr session",
      keys: ["s"],
      run: withTopic((t) => outside(api.openSession(t.id), `Opening herdr session ${t.session}`)),
    },
    { id: "new", label: "New topic", keys: ["n"], run: newTopic },
    {
      id: "step",
      label: "Add a step",
      keys: ["a"],
      run: withTopic((t) =>
        prompt({ title: `Next step for “${t.title}”`, action: "add", submit: (text) => run(api.addStep(t.id, text)) })
      ),
    },
    { id: "done-step", label: "Check the next step (or the selected one)", keys: ["x"], run: doneStep },
    { id: "step-up", label: "Move the step up", keys: ["K"], when: onStep, run: () => moveStep(-1) },
    { id: "step-down", label: "Move the step down", keys: ["J"], when: onStep, run: () => moveStep(1) },
    {
      id: "fold",
      label: "Fold the issue's sub-issues, or go to its parent",
      keys: ["ArrowLeft", "h"],
      when: onIssueRow,
      run: fold,
    },
    {
      id: "unfold",
      label: "Unfold the issue's sub-issues, or go to the first one",
      keys: ["ArrowRight"],
      when: onIssueRow,
      run: unfold,
    },
    { id: "fold-all", label: "Fold or unfold every sub-issue", keys: ["z"], when: onIssue, run: foldAll },
    { id: "scroll-down", label: "Scroll the issue down", keys: [" ", "J", "PageDown"], when: onIssue, run: (key) =>
      scrollIssue(key === "J" ? 0.25 : 1) },
    { id: "scroll-up", label: "Scroll the issue up", keys: ["K", "PageUp"], when: onIssue, run: (key) =>
      scrollIssue(key === "K" ? -0.25 : -1) },
    {
      id: "note",
      label: "Write a note",
      keys: ["w"],
      run: withTopic((t) =>
        prompt({ title: `Note on “${t.title}”`, action: "add", submit: (text) => run(api.addNote(t.id, text)) })
      ),
    },
    { id: "link", label: "Link an issue, pull request or thread", keys: ["l"], run: linkSomething },
    { id: "open", label: "Open the link (the first one, or the selected one)", keys: ["o"], run: open },
    { id: "initiative", label: "Link to a Linear initiative", keys: ["i"], run: pickInitiative },
    {
      id: "block",
      label: "Mark blocked, or unblocked",
      keys: ["!"],
      run: withTopic((t) => {
        if (isBlocked(t)) return run(api.unblock(t.id));
        prompt({
          title: `What blocks “${t.title}”?`,
          action: "block",
          submit: (reason) => run(api.block(t.id, reason)),
        });
      }),
    },
    {
      id: "rework",
      label: "Record rework (reopens a done topic)",
      keys: ["R"],
      run: withTopic((t) =>
        prompt({
          title: `What came back on “${t.title}”?`,
          placeholder: "Changes requested, bug found after release…",
          action: "record",
          submit: (reason) => run(api.rework(t.id, reason)),
        })
      ),
    },
    { id: "finish", label: "Finish the topic", keys: ["f"], run: withTopic((t) => run(api.finish(t.id))) },
    {
      id: "park",
      label: "Park the topic in the backlog",
      keys: ["p"],
      run: withTopic((t) => run(api.park(t.id))),
    },
    {
      id: "move",
      label: "Move the topic to another slot",
      keys: ["m"],
      run: withTopic((t) =>
        pickSlot(`Move “${t.title}” to`, (slot) =>
          run(api.moveTopic(t.id, slot), (next) => {
            if (t.stage === "active") selectSlot(slot);
            return undoable(next);
          }))
      ),
    },
    {
      id: "rename",
      label: "Rename the topic",
      keys: ["r"],
      run: withTopic((t) =>
        prompt({ title: "Rename", value: t.title, action: "rename", submit: (title) => run(api.rename(t.id, title)) })
      ),
    },
    {
      id: "folder",
      label: "Change the folder the session starts in",
      keys: ["c"],
      run: withTopic((t) =>
        prompt({
          title: `Folder of “${t.title}”`,
          value: t.folder,
          action: "save",
          submit: (folder) => run(api.setFolder(t.id, folder)),
        })
      ),
    },
    {
      id: "session-name",
      label: "Rename the herdr session",
      keys: ["C"],
      run: withTopic((t) =>
        prompt({
          title: `herdr session of “${t.title}”`,
          value: t.session,
          action: "save",
          submit: (name) => run(api.setSession(t.id, name)),
        })
      ),
    },
    { id: "delete", label: "Delete the selected row, or the topic", keys: ["d", "Backspace"], run: remove },
    { id: "stats", label: "Flow stats: lead, cycle, blocked, rework", keys: ["t"], run: () => (overlay = "stats") },
    {
      id: "refresh",
      label: "Read Linear, GitHub and herdr now",
      keys: ["."],
      run: () => outside(api.refresh(), "Reading Linear, GitHub and herdr…"),
    },
    { id: "undo", label: "Undo", keys: ["⌘Z", "u"], run: undo },
    { id: "redo", label: "Redo", keys: ["⌘⇧Z", "U"], run: redo },
    { id: "settings", label: "Settings", keys: ["⌘,"], run: () => (overlay = "settings") },
    { id: "help", label: "Keyboard shortcuts", keys: ["?"], run: () => (overlay = "help") },
  ];

  function onKeydown(event: KeyboardEvent) {
    // ⌘, ⌘Z and ⌘⇧Z belong to the native menu, which sends them as `menu` events.
    if (event.metaKey && event.key === "k") {
      event.preventDefault();
      overlay = overlay === "palette" ? null : "palette";
      return;
    }
    const target = event.target as HTMLElement;
    if (overlay || event.defaultPrevented || event.metaKey || event.ctrlKey || event.altKey || target.closest("input, textarea")) return;
    const command = commands.find((c) => c.keys.includes(event.key) && applies(c));
    if (command) {
      event.preventDefault();
      command.run(event.key);
    }
  }

  function closeOverlay() {
    overlay = null;
  }

</script>

<svelte:window onkeydown={onKeydown} />

<main>
  <header data-tauri-drag-region>
    <h1 data-tauri-drag-region>maquereau</h1>
    <nav data-tauri-drag-region>
      <span class:current={view === "focus"}>Focus</span>
      <span class:current={view === "backlog"}>
        Backlog <small>
          {data.topics.filter((t) => t.stage === "queued").length +
          data.backlog.filter((e) => e.issue.tone !== "done" && e.issue.tone !== "closed").length}
        </small>
      </span>
    </nav>
    <span class="status" data-tauri-drag-region>
      {#if attention}<span class="attention">● {attention} waiting for you</span> ·{/if}
      {#if data.live.errors.length}
        <span class="errors" title={data.live.errors.join("\n")}>⚠ {data.live.errors.length}</span> ·
      {/if}
      {data.live.refreshed_at ? `read ${ago(data.live.refreshed_at, data.now)}` : "not read yet"}
      · <kbd>⌘K</kbd> · <kbd>?</kbd>
    </span>
  </header>

  <div class="body">
    <aside bind:this={list} class:backlog={view === "backlog"}>
      {#if view === "focus"}
        {#each slots as { slot, label }, i (slot)}
          {@const topic = activeOf(slot)}
          <SlotCard
            number={i + 1}
            {label}
            {topic}
            times={topic ? times.get(topic.id) : undefined}
            live={data.live}
            selected={i === slotIndex}
            onSelect={() => {
              slotIndex = i;
              detailRow = -1;
            }}
          />
        {/each}
      {:else}
        {#each backlogRows as item, i (item.key)}
          {@const heading = i === 0 || backlogRows[i - 1]!.kind !== item.kind ||
          (item.kind === "topic" && backlogRows[i - 1]!.kind === "topic" &&
            (backlogRows[i - 1] as { topic: Topic }).topic.stage !== item.topic.stage)}
          {#if heading}
            <h4>
              {item.kind === "issue" ? "Assigned in Linear" : item.topic.stage === "done" ? "Done" : "Parked topics"}
            </h4>
          {/if}
          <button
            class="row"
            data-key={item.key}
            class:selected={item.key === backlogKey}
            onclick={() => {
              backlogKey = item.key;
              detailRow = -1;
            }}
          >
            {#if item.kind === "topic"}
              <span class="tag">{api.slotLabel(item.topic.slot)}</span>
              <span class="text">{item.topic.title}</span>
            {:else}
              <span class="priority" data-priority={item.issue.priority} title={api.priorityLabel(item.issue.priority)}>
                {priorityMark[item.issue.priority] ?? "·"}
              </span>
              <span
                class="progress"
                data-type={item.issue.state_type}
                data-tone={item.issue.tone}
                style:color={item.issue.state_color || null}
                title={item.issue.state}
              >
                {stateMark[item.issue.state_type]}
              </span>
              <span class="fold" style:--depth={item.depth}>{item.children ? item.folded ? "▸" : "▾" : ""}</span>
              <span class="tag">{item.issue.key}</span>
              <span class="text" class:done={item.issue.tone === "done"}>{item.issue.title}</span>
              {#if item.folded}<span class="tag">+{item.children}</span>{/if}
              {#if !item.issue.mine && item.issue.assignee}<span class="tag">{item.issue.assignee}</span>{/if}
            {/if}
          </button>
        {:else}
          {#if loaded}
            <p class="blank">
              The backlog is empty. Parked topics land here{data.has_linear_key
                ? ", with the Linear issues assigned to you that no topic links."
                : "; set a Linear API key (⌘,) to see your assigned issues too."}
            </p>
          {/if}
        {/each}
      {/if}
    </aside>

    <div class="detail">
      {#if selected}
        <Detail
          topic={selected}
          times={times.get(selected.id)}
          live={data.live}
          now={data.now}
          rows={detailRows}
          active={detailRow}
          onRow={(i) => (detailRow = i)}
        />
      {:else if selectedIssue}
        {#key selectedIssue.key}
          <IssueView
            issue={selectedIssue}
            subIssues={subIssues(selectedIssue.key)}
            now={data.now}
            bind:pane={issuePane}
          />
        {/key}
      {:else if loaded}
        <div class="blank">
          {#if view === "focus"}
            <p>
              {slots[slotIndex]!.label} is empty. <kbd>n</kbd> writes a new topic; <kbd>b</kbd> picks one from the
              backlog.
            </p>
          {:else}
            <p>Nothing selected.</p>
          {/if}
        </div>
      {/if}
    </div>
  </div>
</main>

{#if overlay === "palette"}
  <Palette
    items={data.topics}
    commands={commands.filter((c) => c.id !== "palette" && applies(c))}
    onOpen={(id) => {
      const topic = data.topics.find((t) => t.id === id);
      if (topic) reach(topic);
    }}
    onClose={closeOverlay}
  />
{:else if overlay === "settings"}
  <SettingsView
    settings={data.settings}
    hasLinearKey={data.has_linear_key}
    onChange={changeSettings}
    onEdit={editSetting}
    onLinearKey={linearKey}
    onClose={closeOverlay}
  />
{:else if overlay === "help"}
  <Help {commands} onClose={closeOverlay} />
{:else if overlay === "stats"}
  <Stats stats={data.stats} days={data.settings.stats_days} onClose={closeOverlay} />
{:else if overlay === "prompt" && ask}
  {@const current = ask}
  <Prompt
    title={current.title}
    value={current.value}
    placeholder={current.placeholder}
    secret={current.secret}
    allowEmpty={current.allowEmpty}
    action={current.action}
    onSubmit={(text) => {
      overlay = null;
      current.submit(text);
    }}
    onClose={closeOverlay}
  />
{:else if overlay === "pick" && choice}
  {@const current = choice}
  <Pick title={current.title} options={current.options} onPick={current.pick} onClose={closeOverlay} />
{/if}

{#if toast}
  <div class="toast" class:error={toast.error} role="status">{toast.text}</div>
{/if}

<style>
  main {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
    /* Room for the traffic lights of the overlay title bar. */
    padding: 10px 16px 10px 84px;
    border-bottom: 1px solid var(--surface0);
    font-size: 11px;
    color: var(--overlay0);
  }

  h1 {
    margin: 0;
    font-size: 13px;
    color: var(--text);
  }

  nav {
    display: flex;
    gap: 14px;
    flex: 1;
  }

  nav .current {
    color: var(--text);
    font-weight: 600;
  }

  .attention,
  .errors {
    color: var(--red);
  }

  .body {
    flex: 1;
    display: grid;
    grid-template-columns: minmax(260px, 36%) 1fr;
    min-height: 0;
  }

  aside {
    display: grid;
    grid-template-rows: repeat(4, 1fr);
    gap: 8px;
    padding: 10px;
    border-right: 1px solid var(--surface0);
    min-height: 0;
    overflow-y: auto;
  }

  aside.backlog {
    display: block;
  }

  h4 {
    margin: 12px 8px 4px;
    font-size: 11px;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--overlay0);
  }

  h4:first-child {
    margin-top: 2px;
  }

  .row {
    width: 100%;
    display: flex;
    gap: 8px;
    align-items: baseline;
    padding: 6px 8px;
    border: 0;
    border-radius: 6px;
    background: none;
    text-align: left;
    cursor: default;
  }

  .row.selected {
    background: var(--mantle);
    box-shadow: inset 3px 0 0 var(--accent);
  }

  .tag {
    flex: none;
    font-size: 11px;
    color: var(--overlay0);
    font-variant-numeric: tabular-nums;
  }

  .fold {
    flex: none;
    width: calc(var(--depth) * 16px + 10px);
    text-align: right;
    font-size: 11px;
    color: var(--subtext);
  }

  .priority,
  .progress {
    flex: none;
    width: 20px;
    font-size: 9px;
    letter-spacing: -1px;
    text-align: center;
    color: var(--overlay0);
  }

  .progress {
    width: 14px;
    font-size: 12px;
    letter-spacing: 0;
  }

  .priority[data-priority="1"] {
    font-size: 12px;
    font-weight: 700;
    color: var(--red);
  }

  .priority[data-priority="2"] {
    color: var(--yellow);
  }

  .progress[data-type="backlog"],
  .progress[data-type="triage"] {
    color: var(--text);
  }

  .progress[data-tone="progress"] {
    color: var(--blue);
  }

  .progress[data-tone="review"] {
    color: var(--yellow);
  }

  .progress[data-tone="done"] {
    color: var(--green);
  }

  .text.done {
    text-decoration: line-through;
    color: var(--overlay0);
  }

  .text {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .detail {
    min-height: 0;
    min-width: 0;
  }

  .blank {
    height: 100%;
    display: flex;
    justify-content: center;
    align-items: center;
    padding: 20px;
    color: var(--subtext);
    text-align: center;
  }

  .toast {
    position: fixed;
    bottom: 20px;
    left: 50%;
    transform: translateX(-50%);
    z-index: 30;
    max-width: 70vw;
    padding: 8px 14px;
    border-radius: 8px;
    border: 1px solid var(--surface0);
    background: var(--mantle);
    box-shadow: var(--shadow);
    font-size: 13px;
  }

  .toast.error {
    border-color: var(--red);
    color: var(--red);
  }
</style>
