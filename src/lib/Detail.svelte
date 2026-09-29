<script lang="ts">
  import { slotLabel, type Live, type Times, type Topic } from "./api";
  import {
    ago,
    date,
    duration,
    kindGlyph,
    linkStatus,
    linkTitle,
    type Row,
    sessionLabel,
    sessionState,
  } from "./format";
  import { reveal } from "./scroll";

  // A topic in full: its flow times, session, initiative, and the rows ↵ walks through.
  let { topic, times, live, now, rows, active, onRow }: {
    topic: Topic;
    times: Times | undefined;
    live: Live;
    now: number;
    rows: Row[];
    /** The row the keys act on, or -1 when the list has the keys. */
    active: number;
    onRow: (index: number) => void;
  } = $props();

  let pane: HTMLElement | undefined = $state();

  const session = $derived(live.sessions[topic.session]);
  const suggested = $derived(
    topic.initiative
      ? null
      : topic.links.flatMap((l) => linkStatus(live, l)?.initiatives ?? [])[0] ?? null,
  );
  const reworks = $derived(topic.reworks.length);

  $effect(() => {
    const row = pane?.querySelector<HTMLElement>(`[data-row="${active}"]`);
    if (pane && row) reveal(pane, row);
  });

  const sections: { kind: Row["kind"]; title: string; empty: string }[] = [
    { kind: "step", title: "Steps", empty: "a adds a step" },
    { kind: "link", title: "Links", empty: "l links an issue, pull request or thread" },
    { kind: "note", title: "Notes", empty: "w writes a note" },
    { kind: "block", title: "Blocks", empty: "" },
    { kind: "rework", title: "Rework", empty: "" },
  ];
</script>

<section bind:this={pane}>
  <header>
    <h2>{topic.title}</h2>
    <p class="meta">
      {slotLabel(topic.slot)} · {topic.stage === "queued" ? "parked" : topic.stage}
      · written {date(topic.created_at)}
      {#if topic.initiative}
        · <span class="initiative">◇ {topic.initiative.name}</span>
      {:else if suggested}
        · <span class="suggested">◇ {suggested.name}? <kbd>i</kbd></span>
      {/if}
    </p>
  </header>

  <dl class="flow">
    <div><dt>Lead</dt><dd>{duration(times?.lead)}</dd></div>
    <div><dt>Cycle</dt><dd>{duration(times?.cycle)}</dd></div>
    <div><dt>Blocked</dt><dd class:red={!!times?.blocked}>{duration(times?.blocked)}</dd></div>
    <div><dt>Rework</dt><dd class:red={reworks > 0}>{reworks}</dd></div>
  </dl>

  <p class="session" data-state={sessionState(session) ?? "off"}>
    <kbd>s</kbd> herdr <code>{topic.session}</code> · {sessionLabel(session)} · <span class="folder">{topic.folder}</span>
  </p>

  {#each sections as section (section.kind)}
    {@const mine = rows.map((row, index) => ({ row, index })).filter(({ row }) => row.kind === section.kind)}
    {#if mine.length || section.empty}
      <h3>{section.title}</h3>
      <ul>
        {#each mine as { row, index } (row.id)}
          <li>
            <button data-row={index} class:active={index === active} onclick={() => onRow(index)}>
              {#if row.kind === "step"}
                <span class="mark" class:done={row.step.done_at !== null}>{row.step.done_at ? "✓" : "○"}</span>
                <span class:struck={row.step.done_at !== null}>{row.step.text}</span>
              {:else if row.kind === "link"}
                {@const status = linkStatus(live, row.link)}
                <span class="mark">{kindGlyph[row.link.kind]}</span>
                <span class="grow">{linkTitle(live, row.link)}</span>
                {#if status}
                  <span class="state" data-tone={status.tone}>{status.state}</span>
                  {#if status.comments}<span class="comments">💬 {status.comments}</span>{/if}
                {/if}
              {:else if row.kind === "note"}
                <span class="mark">✎</span>
                <span class="grow">{row.note.text}</span>
                <span class="when">{ago(row.note.at, now)}</span>
              {:else if row.kind === "block"}
                <span class="mark red">■</span>
                <span class="grow">{row.block.reason}</span>
                <span class="when">
                  {row.block.until ? duration(row.block.until - row.block.since) : `since ${ago(row.block.since, now)}`}
                </span>
              {:else}
                <span class="mark">↺</span>
                <span class="grow">{row.rework.reason}</span>
                <span class="when">{date(row.rework.at)}</span>
              {/if}
            </button>
          </li>
        {:else}
          <li class="none">{section.empty}</li>
        {/each}
      </ul>
    {/if}
  {/each}
</section>

<style>
  section {
    height: 100%;
    overflow-y: auto;
    padding: 14px 18px 40px;
  }

  h2 {
    margin: 0;
    font-size: 17px;
  }

  .meta {
    margin: 4px 0 0;
    font-size: 12px;
    color: var(--overlay0);
  }

  .initiative {
    color: var(--accent);
  }

  .suggested {
    font-style: italic;
  }

  .flow {
    display: flex;
    gap: 24px;
    margin: 14px 0 8px;
  }

  .flow dt {
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--overlay0);
  }

  .flow dd {
    margin: 2px 0 0;
    font-size: 15px;
    font-variant-numeric: tabular-nums;
  }

  .red {
    color: var(--red);
  }

  .session {
    margin: 0 0 6px;
    font-size: 12px;
    color: var(--subtext);
  }

  .session[data-state="blocked"] {
    color: var(--red);
    font-weight: 600;
  }

  .folder {
    color: var(--overlay0);
  }

  h3 {
    margin: 16px 0 4px;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--overlay0);
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  button {
    width: 100%;
    display: flex;
    align-items: baseline;
    gap: 8px;
    padding: 5px 8px;
    border: 0;
    border-radius: 6px;
    background: none;
    text-align: left;
    cursor: default;
  }

  button.active {
    background: var(--mantle);
    box-shadow: inset 3px 0 0 var(--accent);
  }

  .mark {
    width: 1em;
    color: var(--overlay0);
  }

  .mark.done {
    color: var(--green);
  }

  .struck {
    text-decoration: line-through;
    color: var(--overlay0);
  }

  .grow {
    flex: 1;
    min-width: 0;
    overflow-wrap: anywhere;
  }

  .state,
  .comments,
  .when {
    font-size: 12px;
    white-space: nowrap;
    color: var(--overlay0);
  }

  .state[data-tone="attention"] {
    color: var(--red);
  }

  .state[data-tone="review"] {
    color: var(--yellow);
  }

  .state[data-tone="progress"] {
    color: var(--blue);
  }

  .state[data-tone="done"] {
    color: var(--green);
  }

  .none {
    padding: 4px 8px;
    font-size: 12px;
    color: var(--overlay0);
  }
</style>
