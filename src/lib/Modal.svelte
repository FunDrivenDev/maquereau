<script lang="ts">
  import type { Snippet } from "svelte";

  // A centred dialog over a dimmed window; Escape or a click outside closes it.
  let { title, wide = false, onClose, children }: {
    title: string;
    wide?: boolean;
    onClose: () => void;
    children: Snippet;
  } = $props();

  function onKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      onClose();
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="backdrop" onclick={onClose}>
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div class="modal" class:wide role="dialog" tabindex="-1" aria-label={title} onclick={(e) => e.stopPropagation()}>
    <h3>{title}</h3>
    {@render children()}
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
    padding-top: 16vh;
    background: rgb(0 0 0 / 0.22);
  }

  .modal {
    width: min(560px, 90vw);
    max-height: 72vh;
    overflow-y: auto;
    padding: 18px 20px;
    border-radius: 12px;
    border: 1px solid var(--surface0);
    background: var(--base);
    box-shadow: var(--shadow);
  }

  .modal.wide {
    width: min(760px, 94vw);
  }

  h3 {
    margin: 0 0 12px;
    font-size: 15px;
  }
</style>
