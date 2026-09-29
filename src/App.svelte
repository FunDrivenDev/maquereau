<script lang="ts">
  import * as api from "./lib/api";
  import type { Settings, Snapshot } from "./lib/api";
  import type { Command } from "./lib/commands";
  import Help from "./lib/Help.svelte";
  import Palette from "./lib/Palette.svelte";
  import Prompt from "./lib/Prompt.svelte";
  import { reveal } from "./lib/scroll";
  import SettingsView from "./lib/Settings.svelte";

  type Overlay = "palette" | "settings" | "help" | "add" | "rename";

  let data = $state<Snapshot>({
    items: [],
    settings: { theme: "system", search_limit: 50 },
    undo: null,
    redo: null,
  });
  let loaded = $state(false);
  let selectedId = $state<number | null>(null);
  let overlay = $state<Overlay | null>(null);
  let toast = $state<{ text: string; error: boolean } | null>(null);
  let list: HTMLElement | undefined = $state();
  let toastTimer: ReturnType<typeof setTimeout> | undefined;

  const selectedIndex = $derived(data.items.findIndex((i) => i.id === selectedId));
  const selected = $derived(data.items[selectedIndex] ?? null);

  function show(text: string, error = false) {
    toast = { text, error };
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => (toast = null), error ? 6000 : 4000);
  }

  /** Takes a new snapshot, keeping the selection or, when its item left, the item now in its place. */
  function apply(next: Snapshot) {
    const before = selectedIndex;
    data = next;
    loaded = true;
    if (selectedId !== null && data.items.some((i) => i.id === selectedId)) return;
    selectedId = data.items[Math.max(0, Math.min(before, data.items.length - 1))]?.id ?? null;
  }

  /** Runs a change on the Rust side and says what it did, with how to take it back. */
  async function run(change: Promise<Snapshot>, done: (next: Snapshot) => string | null) {
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

  function undo() {
    const label = data.undo;
    if (!label) return show("Nothing to undo");
    run(api.undo(), () => `Undid ${label} · ⌘⇧Z to redo`);
  }

  function redo() {
    const label = data.redo;
    if (!label) return show("Nothing to redo");
    run(api.redo(), undoable);
  }

  function changeSettings(settings: Settings) {
    run(api.setSettings(settings), () => null);
  }

  $effect(() => {
    document.documentElement.dataset.theme = data.settings.theme;
  });

  $effect(() => {
    api.snapshot().then(apply, (e) => show(String(e), true));
    const unlisten = api.onMenu((id) => {
      if (id === "settings") overlay = overlay === "settings" ? null : "settings";
      else if (id === "undo") undo();
      else if (id === "redo") redo();
    });
    return () => void unlisten.then((stop) => stop());
  });

  $effect(() => {
    const row = list?.querySelector<HTMLElement>(`[data-id="${selectedId}"]`);
    if (list && row) reveal(list, row);
  });

  function move(delta: number) {
    const n = data.items.length;
    if (!n) return;
    const i = selectedIndex < 0 ? 0 : Math.max(0, Math.min(n - 1, selectedIndex + delta));
    selectedId = data.items[i]?.id ?? null;
  }

  function withSelected(action: (id: number) => void) {
    return () => {
      if (selected) action(selected.id);
    };
  }

  const commands: Command[] = [
    { id: "palette", label: "Search and commands", keys: ["⌘K", "/"], run: () => (overlay = "palette") },
    { id: "next", label: "Next item", keys: ["j", "ArrowDown"], run: () => move(1) },
    { id: "previous", label: "Previous item", keys: ["k", "ArrowUp"], run: () => move(-1) },
    { id: "first", label: "First item", keys: ["g"], run: () => move(-Infinity) },
    { id: "last", label: "Last item", keys: ["G"], run: () => move(Infinity) },
    { id: "add", label: "Add an item", keys: ["n"], run: () => (overlay = "add") },
    { id: "rename", label: "Rename the item", keys: ["r", "Enter"], run: withSelected(() => (overlay = "rename")) },
    {
      id: "delete",
      label: "Delete the item",
      keys: ["d", "Backspace"],
      run: withSelected((id) => run(api.remove(id), undoable)),
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
    if (overlay || event.metaKey || event.ctrlKey || event.altKey || target.closest("input, textarea")) return;
    const command = commands.find((c) => c.keys.includes(event.key));
    if (command) {
      event.preventDefault();
      command.run();
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

<main>
  <header data-tauri-drag-region>
    <h1 data-tauri-drag-region>maquereau</h1>
    <span data-tauri-drag-region><kbd>⌘K</kbd> search · <kbd>?</kbd> keys</span>
  </header>

  {#if data.items.length}
    <ul bind:this={list}>
      {#each data.items as item (item.id)}
        <li>
          <button data-id={item.id} class:selected={item.id === selectedId} onclick={() => (selectedId = item.id)}>
            {item.title}
          </button>
        </li>
      {/each}
    </ul>
  {:else if loaded}
    <div class="blank" data-tauri-drag-region>
      <p>Nothing here yet. Press <kbd>n</kbd> to add an item.</p>
    </div>
  {/if}
</main>

{#if overlay === "palette"}
  <Palette
    items={data.items}
    commands={commands.filter((c) => c.id !== "palette")}
    onOpen={(id) => (selectedId = id)}
    onClose={() => (overlay = null)}
  />
{:else if overlay === "settings"}
  <SettingsView settings={data.settings} onChange={changeSettings} onClose={() => (overlay = null)} />
{:else if overlay === "help"}
  <Help {commands} onClose={() => (overlay = null)} />
{:else if overlay === "add"}
  <Prompt
    title="Add an item"
    action="add"
    onSubmit={(title) => {
      overlay = null;
      const index = selectedIndex + 1;
      run(api.add(title, index), (next) => {
        selectedId = next.items[index]?.id ?? selectedId;
        return undoable(next);
      });
    }}
    onClose={() => (overlay = null)}
  />
{:else if overlay === "rename" && selected}
  {@const item = selected}
  <Prompt
    title="Rename"
    value={item.title}
    action="rename"
    onSubmit={(title) => {
      overlay = null;
      run(api.rename(item.id, title), undoable);
    }}
    onClose={() => (overlay = null)}
  />
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

  ul {
    list-style: none;
    margin: 0;
    padding: 8px;
    overflow-y: auto;
  }

  li button {
    width: 100%;
    padding: 8px 12px;
    border: 0;
    border-radius: 6px;
    background: none;
    text-align: left;
    cursor: default;
  }

  li button.selected {
    background: var(--mantle);
    box-shadow: inset 3px 0 0 var(--accent);
  }

  .blank {
    flex: 1;
    display: flex;
    justify-content: center;
    align-items: center;
    color: var(--subtext);
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
