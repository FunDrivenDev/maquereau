<script lang="ts">
  import { type Issue, openUrl, priorityLabel } from "./api";
  import { ago, date, stateMark } from "./format";
  import { render } from "./markdown";

  // A Linear issue of the backlog, read-only: what it asks, before focusing on it.
  let { issue, subIssues, now, pane = $bindable() }: {
    issue: Issue;
    subIssues: Issue[];
    now: number;
    /** The scrolling element, for the keys that scroll it. */
    pane?: HTMLElement;
  } = $props();

  const seconds = (iso: string) => Math.floor(Date.parse(iso) / 1000);
  const description = $derived(issue.description.trim() ? render(issue.description) : "");

  // A link of the description opens in the browser, never in the app's window.
  function follow(event: MouseEvent) {
    const link = (event.target as Element).closest("a");
    if (!link) return;
    event.preventDefault();
    if (/^https?:/.test(link.href)) void openUrl(link.href);
  }
</script>

<section bind:this={pane}>
  <header>
    <h2><span class="key">{issue.key}</span> {issue.title}</h2>
    <p class="meta">
      <span class="state" data-tone={issue.tone} style:--state={issue.state_color || null}>
        <span class="glyph">{stateMark[issue.state_type]}</span>{issue.state}
      </span>
      · <span class="priority" data-priority={issue.priority}>{priorityLabel(issue.priority)}</span>
      {#if issue.estimate !== null}· {issue.estimate} pt{/if}
      {#if issue.due_date}· due {date(seconds(issue.due_date))}{/if}
      {#if issue.project}· {issue.project}{/if}
      {#if !issue.mine}· {issue.assignee ? `assigned to ${issue.assignee}` : "unassigned"}{/if}
      {#if issue.initiatives[0]}· <span class="initiative">◇ {issue.initiatives[0].name}</span>{/if}
    </p>
    {#if issue.parent}
      <p class="meta">↳ sub-issue of {issue.parent.key} {issue.parent.title}</p>
    {/if}
    {#if issue.labels.length}
      <p class="labels">
        {#each issue.labels as label (label.name)}
          <span class="label" style:--dot={label.color}>
            {#if label.group}<span class="group">{label.group}</span>{/if}{label.name}
          </span>
        {/each}
      </p>
    {/if}
    <p class="meta">
      {#if issue.created_at}written {date(seconds(issue.created_at))}{/if}
      {#if issue.updated_at}· updated {ago(seconds(issue.updated_at), now)}{/if}
    </p>
  </header>

  {#if subIssues.length}
    <h3>Sub-issues</h3>
    <ul>
      {#each subIssues as sub (sub.key)}
        <li>
          <span class="key">{sub.key}</span>
          <span class="grow" class:struck={sub.tone === "done"}>{sub.title}</span>
          {#if !sub.mine && sub.assignee}<span class="meta">{sub.assignee}</span>{/if}
          <span class="state meta" data-tone={sub.tone} style:--state={sub.state_color || null}>
            <span class="glyph">{stateMark[sub.state_type]}</span>{sub.state}
          </span>
        </li>
      {/each}
    </ul>
  {/if}

  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div class="description" onclick={follow}>
    {#if description}
      {@html description}
    {:else}
      <p class="none">No description.</p>
    {/if}
  </div>

  <p class="hint">
    <kbd>1</kbd>–<kbd>4</kbd> or <kbd>↵</kbd> makes it a topic in a slot · <kbd>l</kbd> links it to a topic ·
    <kbd>o</kbd> opens it in Linear · <kbd>space</kbd> <kbd>J</kbd> <kbd>K</kbd> scroll · <kbd>←</kbd>
    <kbd>→</kbd> <kbd>z</kbd> fold its sub-issues
  </p>
</section>

<style>
  section {
    height: 100%;
    overflow-y: auto;
    padding: 14px 18px 40px;
  }

  h2 {
    margin: 0 0 6px;
    font-size: 18px;
    line-height: 1.3;
  }

  .key {
    color: var(--overlay0);
    font-weight: 500;
  }

  .meta {
    margin: 3px 0;
    font-size: 12px;
    color: var(--overlay0);
  }

  /* Linear's own colour for the state, so it reads the same here as there; the tone is the
     fallback until the next read brings the colour. */
  .state {
    --state: var(--overlay0);
  }

  .state[data-tone="review"] {
    --state: var(--yellow);
  }

  .state[data-tone="progress"] {
    --state: var(--blue);
  }

  .state[data-tone="done"] {
    --state: var(--green);
  }

  .glyph {
    margin-right: 4px;
    color: var(--state);
  }

  header .state {
    padding: 1px 8px 1px 6px;
    border-radius: 10px;
    background: color-mix(in srgb, var(--state) 16%, transparent);
    color: var(--text);
    font-weight: 500;
  }

  li .state {
    color: var(--subtext);
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
    margin: 8px 0 4px;
  }

  .label {
    display: inline-flex;
    align-items: center;
    padding: 2px 10px 2px 8px;
    border: 1px solid var(--surface1);
    border-radius: 12px;
    font-size: 13px;
    color: var(--text);
  }

  .label::before {
    content: "";
    width: 8px;
    height: 8px;
    margin-right: 6px;
    border-radius: 50%;
    background: var(--dot);
  }

  .group {
    margin-right: 5px;
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
    margin: 0;
    padding: 0;
    list-style: none;
  }

  li {
    display: flex;
    gap: 8px;
    align-items: baseline;
    padding: 2px 0;
  }

  li .key {
    font-size: 12px;
  }

  .grow {
    flex: 1;
    min-width: 0;
  }

  .struck {
    text-decoration: line-through;
    color: var(--overlay0);
  }

  li .meta {
    margin: 0;
    white-space: nowrap;
  }

  /* The description, as Linear renders it. */
  .description {
    margin-top: 16px;
    padding-top: 12px;
    border-top: 1px solid var(--surface0);
    line-height: 1.6;
    overflow-wrap: anywhere;
    user-select: text;
    -webkit-user-select: text;
    cursor: auto;
  }

  .description :global(:first-child) {
    margin-top: 0;
  }

  .description :global(p),
  .description :global(ul),
  .description :global(ol),
  .description :global(blockquote),
  .description :global(pre),
  .description :global(table) {
    margin: 0 0 10px;
  }

  .description :global(:is(h1, h2, h3, h4, h5, h6)) {
    margin: 18px 0 6px;
    line-height: 1.3;
    font-weight: 650;
  }

  .description :global(h1) {
    font-size: 19px;
  }

  .description :global(h2) {
    font-size: 17px;
  }

  .description :global(h3) {
    font-size: 15px;
  }

  .description :global(:is(h4, h5, h6)) {
    font-size: 14px;
    color: var(--subtext);
  }

  .description :global(:is(ul, ol)) {
    padding-left: 22px;
  }

  .description :global(li) {
    margin: 2px 0;
  }

  .description :global(li > :is(ul, ol)) {
    margin: 2px 0 0;
  }

  .description :global(li::marker) {
    color: var(--overlay0);
  }

  .description :global(li:has(> input[type="checkbox"])) {
    list-style: none;
    margin-left: -20px;
  }

  .description :global(input[type="checkbox"]) {
    margin: 0 6px 0 0;
    vertical-align: -1px;
    accent-color: var(--accent);
  }

  .description :global(a) {
    color: var(--blue);
    text-decoration: none;
    cursor: pointer;
  }

  .description :global(a:hover) {
    text-decoration: underline;
  }

  .description :global(strong) {
    font-weight: 650;
  }

  .description :global(del) {
    color: var(--overlay0);
  }

  .description :global(blockquote) {
    padding: 2px 0 2px 12px;
    border-left: 3px solid var(--surface1);
    color: var(--subtext);
  }

  .description :global(hr) {
    margin: 16px 0;
    border: 0;
    border-top: 1px solid var(--surface0);
  }

  .description :global(code) {
    padding: 1px 5px;
    border-radius: 4px;
    background: var(--mantle);
    font: 12.5px var(--mono);
  }

  .description :global(pre) {
    padding: 10px 12px;
    border: 1px solid var(--surface0);
    border-radius: 8px;
    background: var(--mantle);
    overflow-x: auto;
  }

  .description :global(pre code) {
    padding: 0;
    background: none;
    font: 12px/1.5 var(--mono);
  }

  .description :global(table) {
    display: block;
    max-width: 100%;
    overflow-x: auto;
    border-collapse: collapse;
    font-size: 13px;
  }

  .description :global(:is(th, td)) {
    padding: 5px 10px;
    border: 1px solid var(--surface0);
    text-align: left;
    vertical-align: top;
  }

  .description :global(th) {
    background: var(--mantle);
    font-weight: 600;
  }

  .description :global(tr:nth-child(even) td) {
    background: color-mix(in srgb, var(--mantle) 50%, transparent);
  }

  /* highlight.js tokens, in the theme's colours. */
  .description :global(:is(.hljs-comment, .hljs-quote)) {
    color: var(--overlay0);
    font-style: italic;
  }

  .description :global(:is(.hljs-keyword, .hljs-selector-tag, .hljs-built_in, .hljs-doctag)) {
    color: var(--accent);
  }

  .description :global(:is(.hljs-string, .hljs-regexp, .hljs-addition, .hljs-template-tag)) {
    color: var(--green);
  }

  .description :global(:is(.hljs-number, .hljs-literal, .hljs-symbol, .hljs-bullet)) {
    color: var(--yellow);
  }

  .description :global(:is(.hljs-title, .hljs-section, .hljs-function)) {
    color: var(--blue);
  }

  .description :global(:is(.hljs-type, .hljs-class, .hljs-attr, .hljs-attribute, .hljs-variable)) {
    color: var(--teal);
  }

  .description :global(:is(.hljs-meta, .hljs-tag, .hljs-name, .hljs-selector-class, .hljs-selector-id)) {
    color: var(--subtext);
  }

  .description :global(.hljs-deletion) {
    color: var(--red);
  }

  .description :global(.hljs-emphasis) {
    font-style: italic;
  }

  .description :global(.hljs-strong) {
    font-weight: 700;
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
