<script lang="ts">
  import { type Command, glyph } from "./commands";
  import Modal from "./Modal.svelte";

  let { commands, onClose }: { commands: Command[]; onClose: () => void } = $props();
</script>

<Modal title="Keyboard" {onClose}>
  <dl>
    {#each commands as command (command.id)}
      <dt>
        {#each command.keys as key (key)}<kbd>{glyph(key)}</kbd>{/each}
      </dt>
      <dd>{command.label}</dd>
    {/each}
  </dl>
  <h4>Search</h4>
  <p>
    Every search is fuzzy, and every word must match. As in fzf: <code>'word</code> matches exactly,
    <code>^word</code> at the start, <code>word$</code> at the end, <code>!word</code> excludes.
  </p>
  <h4>Undo</h4>
  <p>Every action can be undone with <kbd>⌘Z</kbd> and redone with <kbd>⌘⇧Z</kbd>, so none asks for confirmation.</p>
</Modal>

<style>
  dl {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 6px 16px;
    margin: 0;
  }

  dt {
    display: flex;
    gap: 4px;
    justify-content: flex-end;
  }

  dd {
    margin: 0;
    color: var(--subtext);
  }

  h4 {
    margin: 18px 0 6px;
    font-size: 13px;
  }

  p {
    margin: 0 0 6px;
    font-size: 12px;
    line-height: 1.5;
    color: var(--subtext);
  }

  code {
    font-family: var(--mono);
    font-size: 11.5px;
  }
</style>
