<script lang="ts" generics="T">
  import * as api from "./api";
  import Highlight from "./Highlight.svelte";
  import { listStep } from "./keys";
  import { reveal } from "./scroll";

  // Picks one option with a fuzzy filter: ↑ ↓ move, ↵ picks, esc cancels.
  let { title, options, onPick, onClose }: {
    title: string;
    options: { label: string; detail?: string; value: T }[];
    onPick: (value: T) => void;
    onClose: () => void;
  } = $props();

  let query = $state("");
  let shown = $state<{ index: number; indices: number[] }[]>([]);
  let active = $state(0);
  let input: HTMLInputElement | undefined = $state();
  let list: HTMLElement | undefined = $state();
  let sequence = 0;

  $effect(() => {
    input?.focus();
  });

  $effect(() => {
    const q = query;
    const labels = options.map((o) => o.label);
    const mine = ++sequence;
    active = 0;
    api.fuzzy(q, labels).then((matches) => {
      if (mine === sequence) shown = matches.map((m) => ({ index: m.index, indices: m.indices }));
    });
  });

  $effect(() => {
    const row = list?.querySelector<HTMLElement>(`[data-index="${active}"]`);
    if (list && row) reveal(list, row);
  });

  function pick(i: number) {
    const option = options[shown[i]?.index ?? -1];
    if (!option) return;
    onClose();
    onPick(option.value);
  }

  function onKeydown(event: KeyboardEvent) {
    const step = listStep(event, active, shown.length);
    if (step !== null) {
      event.preventDefault();
      active = step;
    } else if (event.key === "Enter") {
      event.preventDefault();
      pick(active);
    } else if (event.key === "Escape") {
      event.preventDefault();
      onClose();
    }
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="backdrop" onclick={onClose}>
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div class="pick" role="dialog" tabindex="-1" aria-label={title} onclick={(e) => e.stopPropagation()}>
    <h3>{title}</h3>
    <input
      bind:this={input}
      bind:value={query}
      onkeydown={onKeydown}
      placeholder="Filter"
      spellcheck="false"
      autocomplete="off"
    />
    <ul bind:this={list}>
      {#each shown as { index, indices }, i (index)}
        {@const option = options[index]}
        {#if option}
          <li>
            <button
              class:active={i === active}
              data-index={i}
              onmousemove={() => (active = i)}
              onclick={() => pick(i)}
            >
              <span><Highlight text={option.label} {indices} /></span>
              {#if option.detail}<span class="detail">{option.detail}</span>{/if}
            </button>
          </li>
        {/if}
      {:else}
        <li class="none">Nothing to pick.</li>
      {/each}
    </ul>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 20;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding-top: 14vh;
    background: rgb(0 0 0 / 0.22);
  }

  .pick {
    width: min(520px, 90vw);
    max-height: 64vh;
    display: flex;
    flex-direction: column;
    padding: 16px;
    border-radius: 12px;
    border: 1px solid var(--surface0);
    background: var(--base);
    box-shadow: var(--shadow);
  }

  h3 {
    margin: 0 0 10px;
    font-size: 15px;
  }

  input {
    width: 100%;
  }

  ul {
    list-style: none;
    margin: 8px 0 0;
    padding: 0;
    overflow-y: auto;
  }

  button {
    width: 100%;
    display: flex;
    justify-content: space-between;
    gap: 12px;
    padding: 7px 10px;
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

  .detail {
    color: var(--overlay0);
    font-size: 12px;
    white-space: nowrap;
  }

  .none {
    padding: 7px 10px;
    color: var(--overlay0);
  }
</style>
