<script lang="ts">
  import { type Issue, priorityLabel } from "./api";
  import { ago, date } from "./format";
  import { blocks } from "./markdown";

  // A Linear issue of the backlog, read-only: what it asks, before focusing on it.
  let { issue, now, pane = $bindable() }: {
    issue: Issue;
    now: number;
    /** The scrolling element, for the keys that scroll it. */
    pane?: HTMLElement;
  } = $props();

  const seconds = (iso: string) => Math.floor(Date.parse(iso) / 1000);
  const description = $derived(blocks(issue.description));
</script>

<section bind:this={pane}>
  <header>
    <h2><span class="key">{issue.key}</span> {issue.title}</h2>
    <p class="meta">
      <span class="state" data-tone={issue.tone}>{issue.state}</span>
      · <span class="priority" data-priority={issue.priority}>{priorityLabel(issue.priority)}</span>
      {#if issue.estimate !== null}· {issue.estimate} pt{/if}
      {#if issue.due_date}· due {date(seconds(issue.due_date))}{/if}
      {#if issue.project}· {issue.project}{/if}
      {#if issue.initiatives[0]}· <span class="initiative">◇ {issue.initiatives[0].name}</span>{/if}
    </p>
    {#if issue.parent}
      <p class="meta">↳ sub-issue of {issue.parent.key} {issue.parent.title}</p>
    {/if}
    {#if issue.labels.length}
      <p class="labels">
        {#each issue.labels as label (label.name)}
          <span class="label" style:--dot={label.color}>{label.name}</span>
        {/each}
      </p>
    {/if}
    <p class="meta">
      {#if issue.created_at}written {date(seconds(issue.created_at))}{/if}
      {#if issue.updated_at}· updated {ago(seconds(issue.updated_at), now)}{/if}
    </p>
  </header>

  <div class="description">
    {#each description as block, i (i)}
      {#if block.kind === "heading"}
        <p class="heading" data-level={Math.min(block.level, 3)}>{block.text}</p>
      {:else if block.kind === "paragraph"}
        <p>{block.text}</p>
      {:else if block.kind === "item"}
        <p class="item" style:--depth={block.depth}><span class="mark">{block.mark}</span>{block.text}</p>
      {:else if block.kind === "quote"}
        <blockquote>{block.text}</blockquote>
      {:else if block.kind === "code"}
        <pre>{block.text}</pre>
      {:else}
        <hr />
      {/if}
    {:else}
      <p class="none">No description.</p>
    {/each}
  </div>

  <p class="hint">
    <kbd>1</kbd>–<kbd>4</kbd> or <kbd>↵</kbd> makes it a topic in a slot · <kbd>l</kbd> links it to a topic ·
    <kbd>o</kbd> opens it in Linear · <kbd>space</kbd> <kbd>J</kbd> <kbd>K</kbd> scroll
  </p>
</section>

<style>
  section {
    height: 100%;
    overflow-y: auto;
    padding: 14px 18px 40px;
  }

  h2 {
    margin: 0 0 4px;
    font-size: 17px;
  }

  .key {
    color: var(--overlay0);
    font-weight: 500;
  }

  .meta {
    margin: 2px 0;
    font-size: 12px;
    color: var(--overlay0);
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

  .priority[data-priority="1"] {
    color: var(--red);
    font-weight: 600;
  }

  .priority[data-priority="2"] {
    color: var(--yellow);
  }

  .initiative {
    color: var(--accent);
  }

  .labels {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin: 6px 0 2px;
  }

  .label {
    padding: 1px 8px 1px 6px;
    border: 1px solid var(--surface0);
    border-radius: 10px;
    font-size: 11px;
    color: var(--subtext);
  }

  .label::before {
    content: "●";
    margin-right: 4px;
    color: var(--dot);
  }

  .description {
    margin-top: 14px;
    padding-top: 10px;
    border-top: 1px solid var(--surface0);
    line-height: 1.5;
    overflow-wrap: anywhere;
  }

  .description p,
  blockquote {
    margin: 0 0 8px;
    white-space: pre-wrap;
  }

  .heading {
    margin-top: 14px;
    font-weight: 600;
  }

  .heading[data-level="1"] {
    font-size: 16px;
  }

  .heading[data-level="2"] {
    font-size: 15px;
  }

  .description .item {
    margin: 0 0 3px;
    padding-left: calc(var(--depth) * 18px + 18px);
    text-indent: -18px;
  }

  .item .mark {
    display: inline-block;
    width: 18px;
    text-indent: 0;
    color: var(--overlay0);
  }

  blockquote {
    padding-left: 10px;
    border-left: 3px solid var(--surface1);
    color: var(--subtext);
  }

  pre {
    margin: 0 0 8px;
    padding: 8px 10px;
    border-radius: 6px;
    background: var(--mantle);
    font: 12px/1.45 var(--mono);
    white-space: pre-wrap;
  }

  hr {
    border: 0;
    border-top: 1px solid var(--surface0);
  }

  .none,
  .hint {
    font-size: 12px;
    color: var(--overlay0);
  }

  .hint {
    margin-top: 18px;
  }
</style>
