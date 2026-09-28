<script lang="ts">
  import * as api from "./api";
  import type { Settings } from "./api";
  import Highlight from "./Highlight.svelte";

  // Every setting of the app, opened with ⌘,. A new setting needs its field in
  // src-tauri/src/settings.rs and its entry in `fields` below; nothing else configures the app.
  // The Linear API key is kept in the keychain, not in `settings`: its field only asks for it.
  let { settings, hasLinearKey, onChange, onEdit, onLinearKey, onClose }: {
    settings: Settings;
    hasLinearKey: boolean;
    onChange: (settings: Settings) => void;
    /** Asks for a new value of a text setting. */
    onEdit: (key: keyof Settings, label: string) => void;
    onLinearKey: () => void;
    onClose: () => void;
  } = $props();

  type Field =
    & { key: keyof Settings | "linear_key"; label: string; description: string }
    & (
      | { kind: "choice"; options: { value: string; label: string }[] }
      | { kind: "number"; min: number; max: number; step: number }
      | { kind: "toggle" }
      | { kind: "text" }
      | { kind: "action"; run: () => void }
    );

  const fields: Field[] = [
    {
      key: "theme",
      label: "Theme",
      description: "Follow macOS, or stay light or dark.",
      kind: "choice",
      options: [
        { value: "system", label: "System" },
        { value: "light", label: "Light" },
        { value: "dark", label: "Dark" },
      ],
    },
    {
      key: "linear_key",
      label: "Linear API key",
      description: "Reads your issues and initiatives. Create one in Linear › Settings › Security & access. Kept in the keychain.",
      kind: "action",
      run: () => onLinearKey(),
    },
    {
      key: "terminal",
      label: "Terminal",
      description: "Where a topic's herdr session opens.",
      kind: "choice",
      options: [
        { value: "ghostty", label: "Ghostty" },
        { value: "terminal", label: "Terminal" },
      ],
    },
    {
      key: "folder",
      label: "Folder of new topics",
      description: "Where a new topic's session starts; each topic can change it (c).",
      kind: "text",
    },
    {
      key: "refresh_minutes",
      label: "Linear and GitHub refresh",
      description: "Minutes between two reads of the linked issues and pull requests.",
      kind: "number",
      min: 1,
      max: 60,
      step: 1,
    },
    {
      key: "session_seconds",
      label: "Session refresh",
      description: "Seconds between two reads of the herdr sessions.",
      kind: "number",
      min: 5,
      max: 120,
      step: 5,
    },
    {
      key: "notify_sessions",
      label: "Notify on sessions",
      description: "When an agent of a topic's session needs an answer or has finished.",
      kind: "toggle",
    },
    {
      key: "notify_comments",
      label: "Notify on comments",
      description: "When someone else comments on a linked issue or pull request.",
      kind: "toggle",
    },
    {
      key: "stats_days",
      label: "Flow stats window",
      description: "How many days of done topics the flow stats count.",
      kind: "number",
      min: 7,
      max: 365,
      step: 7,
    },
    {
      key: "search_limit",
      label: "Search results",
      description: "How many results a search shows at most.",
      kind: "number",
      min: 5,
      max: 500,
      step: 5,
    },
  ];

  let query = $state("");
  let shown = $state<{ field: Field; indices: number[] }[]>(fields.map((field) => ({ field, indices: [] })));
  let active = $state(0);
  let input: HTMLInputElement | undefined = $state();

  $effect(() => {
    input?.focus();
  });

  // Fuzzy filter on the labels; a slower, older answer never replaces a newer one.
  let sequence = 0;
  $effect(() => {
    const q = query;
    const mine = ++sequence;
    active = 0;
    api.fuzzy(q, fields.map((f) => f.label)).then((matches) => {
      if (mine !== sequence) return;
      shown = matches.flatMap((m) => {
        const field = fields[m.index];
        return field ? [{ field, indices: m.indices }] : [];
      });
    });
  });

  function display(field: Field): string {
    if (field.key === "linear_key") return hasLinearKey ? "Set" : "Not set";
    const value = settings[field.key];
    if (field.kind === "choice") return field.options.find((o) => o.value === value)?.label ?? String(value);
    if (field.kind === "toggle") return value ? "On" : "Off";
    return String(value);
  }

  /** Moves the field's value one step: to the next option, or by one `step`. */
  function nudge(field: Field | undefined, delta: 1 | -1) {
    if (!field) return;
    if (field.kind === "action" || field.kind === "text") {
      if (delta === 1 && field.kind === "action") field.run();
      else if (delta === 1 && field.key !== "linear_key") onEdit(field.key, field.label);
      return;
    }
    if (field.key === "linear_key") return;
    const value = settings[field.key];
    let next: string | number | boolean;
    if (field.kind === "toggle") {
      next = !value;
    } else if (field.kind === "choice") {
      const i = field.options.findIndex((o) => o.value === value);
      const n = field.options.length;
      next = field.options[(i + delta + n) % n]?.value ?? String(value);
    } else if (field.kind === "number") {
      next = Math.min(field.max, Math.max(field.min, Number(value) + delta * field.step));
    } else {
      return;
    }
    if (next !== value) onChange({ ...settings, [field.key]: next });
  }

  function onKeydown(event: KeyboardEvent) {
    const field = shown[active]?.field;
    const down = event.key === "ArrowDown" || (event.ctrlKey && (event.key === "n" || event.key === "j"));
    const up = event.key === "ArrowUp" || (event.ctrlKey && (event.key === "p" || event.key === "k"));
    if (down || up) {
      event.preventDefault();
      const n = shown.length;
      if (n) active = (active + (down ? 1 : n - 1)) % n;
    } else if (event.key === "ArrowRight" || event.key === "Enter") {
      event.preventDefault();
      nudge(field, 1);
    } else if (event.key === "ArrowLeft") {
      event.preventDefault();
      nudge(field, -1);
    } else if (event.key === "Escape") {
      event.preventDefault();
      onClose();
    }
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="backdrop" onclick={onClose}>
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div class="settings" role="dialog" tabindex="-1" aria-label="Settings" onclick={(e) => e.stopPropagation()}>
    <header>
      <h3>Settings</h3>
      <input
        bind:this={input}
        bind:value={query}
        onkeydown={onKeydown}
        placeholder="Filter settings"
        spellcheck="false"
        autocomplete="off"
      />
    </header>
    <ul>
      {#each shown as { field, indices }, i (field.key)}
        <li class:active={i === active}>
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div class="field" onmousemove={() => (active = i)}>
            <span class="label"><Highlight text={field.label} {indices} /></span>
            <span class="description">{field.description}</span>
          </div>
          <span class="value">
            <button tabindex="-1" onclick={() => nudge(field, -1)} aria-label="Previous value">‹</button>
            {display(field)}
            <button tabindex="-1" onclick={() => nudge(field, 1)} aria-label="Next value">›</button>
          </span>
        </li>
      {:else}
        <li class="none">No such setting.</li>
      {/each}
    </ul>
    <footer>
      <span><kbd>↑</kbd><kbd>↓</kbd> move</span>
      <span><kbd>←</kbd><kbd>→</kbd> change</span>
      <span><kbd>⌘Z</kbd> undo</span>
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
    padding-top: 10vh;
    background: rgb(0 0 0 / 0.18);
  }

  .settings {
    width: min(680px, 92vw);
    max-height: 76vh;
    display: flex;
    flex-direction: column;
    border-radius: 12px;
    border: 1px solid var(--surface0);
    background: var(--base);
    box-shadow: var(--shadow);
    overflow: hidden;
  }

  header {
    padding: 14px 16px 10px;
    border-bottom: 1px solid var(--surface0);
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
    margin: 0;
    padding: 6px;
    overflow-y: auto;
  }

  li {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
    padding: 8px 10px;
    border-radius: 6px;
  }

  li.active {
    background: var(--mantle);
    box-shadow: inset 3px 0 0 var(--accent);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .label {
    font-weight: 600;
  }

  .description {
    font-size: 12px;
    color: var(--subtext);
  }

  .value {
    display: flex;
    align-items: center;
    gap: 6px;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  .value button {
    border: 0;
    background: none;
    color: var(--overlay0);
    cursor: pointer;
    font-size: 16px;
  }

  .none {
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
