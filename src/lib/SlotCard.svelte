<script lang="ts">
  import type { Live, Times, Topic } from "./api";
  import { duration, isBlocked, kindGlyph, linkStatus, nextStep, sessionLabel, sessionState } from "./format";

  // One of the four slots: its topic, the next step, and what needs a look.
  let { number, label, topic, times, live, selected, onSelect }: {
    number: number;
    label: string;
    topic: Topic | null;
    times: Times | undefined;
    live: Live;
    selected: boolean;
    onSelect: () => void;
  } = $props();

  const session = $derived(topic ? live.sessions[topic.session] : undefined);
  const state = $derived(sessionState(session));
  const step = $derived(topic ? nextStep(topic) : null);
</script>

<button class="card" class:selected onclick={onSelect}>
  <div class="head">
    <span class="slot"><kbd>{number}</kbd> {label}</span>
    {#if topic && times?.cycle != null}<span class="age" title="Cycle time">{duration(times.cycle)}</span>{/if}
  </div>
  {#if topic}
    <div class="title">
      {#if isBlocked(topic)}<span class="blocked" title="Blocked">■</span>{/if}
      {topic.title}
    </div>
    <div class="next" class:none={!step}>{step ? `→ ${step.text}` : "No next step · a to add one"}</div>
    <div class="foot">
      <span class="session" data-state={state ?? "off"}>● {sessionLabel(session)}</span>
      <span class="links">
        {#each topic.links as link (link.id)}
          {@const status = linkStatus(live, link)}
          <span class="link" data-tone={status?.tone ?? "none"} title={status?.state ?? link.url}>
            {kindGlyph[link.kind]}{#if status?.comments}<sup>{status.comments}</sup>{/if}
          </span>
        {/each}
      </span>
    </div>
  {:else}
    <div class="empty">Empty · <kbd>n</kbd> to add a topic, <kbd>b</kbd> for the backlog</div>
  {/if}
</button>

<style>
  .card {
    display: flex;
    flex-direction: column;
    gap: 6px;
    width: 100%;
    min-height: 0;
    padding: 10px 12px;
    border: 1px solid var(--surface0);
    border-radius: 10px;
    background: none;
    text-align: left;
    cursor: default;
    overflow: hidden;
  }

  .card.selected {
    background: var(--mantle);
    border-color: var(--accent);
    box-shadow: inset 3px 0 0 var(--accent);
  }

  .head,
  .foot {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 8px;
    font-size: 11px;
    color: var(--overlay0);
  }

  .slot {
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .age {
    font-variant-numeric: tabular-nums;
  }

  .title {
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .blocked {
    color: var(--red);
  }

  .next {
    font-size: 13px;
    color: var(--subtext);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .next.none,
  .empty {
    color: var(--overlay0);
    font-size: 12px;
  }

  .session[data-state="blocked"] {
    color: var(--red);
    font-weight: 600;
  }

  .session[data-state="done"] {
    color: var(--green);
  }

  .session[data-state="working"] {
    color: var(--blue);
  }

  .links {
    display: flex;
    gap: 6px;
  }

  .link[data-tone="attention"] {
    color: var(--red);
  }

  .link[data-tone="review"] {
    color: var(--yellow);
  }

  .link[data-tone="progress"] {
    color: var(--blue);
  }

  .link[data-tone="done"] {
    color: var(--green);
  }
</style>
