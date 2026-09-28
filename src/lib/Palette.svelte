<script lang="ts">
  import * as api from "./api";
  import { type Match, slotLabel, type Topic } from "./api";
  import { type Command, glyph } from "./commands";
  import Highlight from "./Highlight.svelte";
  import { reveal } from "./scroll";

  // Fuzzy search over the topics; a query starting with `>` searches the commands instead.
  let { items, commands, onOpen, onClose }: {
    items: Topic[];
    commands: Command[];
    onOpen: (id: number) => void;
    onClose: () => void;
  } = $props();

  type Row =
    | { kind: "item"; item: Topic; indices: number[] }
    | { kind: "command"; command: Command; indices: number[] };

  let query = $state("");
  let matches = $state<Match[]>([]);
  let active = $state(0);
  let input: HTMLInputElement | undefined = $state();
  let list: HTMLElement | undefined = $state();
  let sequence = 0;

  const forCommands = $derived(query.startsWith(">"));

  const rows: Row[] = $derived(
    matches.flatMap((m): Row[] => {
      if (forCommands) {
        const command = commands[m.index];
        return command ? [{ kind: "command", command, indices: m.indices }] : [];
      }
      const item = items[m.index];
      return item ? [{ kind: "item", item, indices: m.indices }] : [];
    }),
  );

  // Search as the query changes; a slower, older answer never replaces a newer one.
  $effect(() => {
    const q = query;
    const commandsMode = forCommands;
    const labels = commands.map((c) => c.label);
    void items.length;
    active = 0;
    const mine = ++sequence;
    const timer = setTimeout(() => {
      const search = commandsMode ? api.fuzzy(q.slice(1), labels) : api.searchTopics(q);
      search.then((result) => {
        if (mine === sequence) matches = result;
      });
    }, 40);
    return () => clearTimeout(timer);
  });

  $effect(() => {
    input?.focus();
  });

  $effect(() => {
    const row = list?.querySelector<HTMLElement>(`[data-index="${active}"]`);
    if (list && row) reveal(list, row);
  });

  function choose(row: Row | undefined) {
    if (!row) return;
    onClose();
    if (row.kind === "command") row.command.run();
    else onOpen(row.item.id);
  }

  function onKeydown(event: KeyboardEvent) {
    const next = event.key === "ArrowDown" || (event.ctrlKey && (event.key === "n" || event.key === "j"));
    const prev = event.key === "ArrowUp" || (event.ctrlKey && (event.key === "p" || event.key === "k"));
    if (next || prev) {
      event.preventDefault();
      const n = rows.length;
      if (n) active = (active + (next ? 1 : n - 1)) % n;
    } else if (event.key === "Enter") {
      event.preventDefault();
      choose(rows[active]);
    } else if (event.key === "Escape") {
      event.preventDefault();
      onClose();
    }
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="backdrop" onclick={onClose}>
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div class="palette" onclick={(e) => e.stopPropagation()}>
    <input
      bind:this={input}
      bind:value={query}
      onkeydown={onKeydown}
      placeholder="Search — type > for commands"
      spellcheck="false"
      autocomplete="off"
    />
    <ul bind:this={list}>
      {#each rows as row, i (row.kind === "item" ? `i${row.item.id}` : row.command.id)}
        <li>
          <button
            class:active={i === active}
            data-index={i}
            onmousemove={() => (active = i)}
            onclick={() => choose(row)}
          >
            {#if row.kind === "command"}
              <span class="title"><Highlight text={row.command.label} indices={row.indices} /></span>
              <span class="keys">{#each row.command.keys as key (key)}<kbd>{glyph(key)}</kbd>{/each}</span>
            {:else}
              <span class="title"><Highlight text={row.item.title} indices={row.indices} /></span>
              <span class="keys">{slotLabel(row.item.slot)} · {row.item.stage}</span>
            {/if}
          </button>
        </li>
      {:else}
        <li class="none">{forCommands ? "No such command." : "No match."}</li>
      {/each}
    </ul>
    <footer>
      <span><kbd>↑</kbd><kbd>↓</kbd> move</span>
      <span><kbd>↵</kbd> open</span>
      <span><kbd>&gt;</kbd> commands</span>
      <span><kbd>esc</kbd> close</span>
    </footer>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 10;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding-top: 12vh;
    background: rgb(0 0 0 / 0.18);
  }

  .palette {
    width: min(620px, 90vw);
    max-height: 70vh;
    display: flex;
    flex-direction: column;
    border-radius: 12px;
    border: 1px solid var(--surface0);
    background: var(--base);
    box-shadow: var(--shadow);
    overflow: hidden;
  }

  input {
    border: 0;
    border-bottom: 1px solid var(--surface0);
    border-radius: 0;
    padding: 14px 16px;
    font-size: 16px;
    background: transparent;
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 6px;
    overflow-y: auto;
  }

  li button {
    width: 100%;
    display: flex;
    justify-content: space-between;
    gap: 12px;
    padding: 8px 10px;
    border: 0;
    border-radius: 6px;
    background: none;
    text-align: left;
    cursor: pointer;
  }

  li button.active {
    background: var(--mantle);
    box-shadow: inset 3px 0 0 var(--accent);
  }

  .title {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .keys {
    display: flex;
    gap: 3px;
  }

  .none {
    padding: 14px;
    color: var(--overlay0);
  }

  footer {
    display: flex;
    gap: 16px;
    padding: 8px 14px;
    border-top: 1px solid var(--surface0);
    font-size: 11px;
    color: var(--overlay0);
  }

  footer kbd {
    margin-right: 3px;
  }
</style>
