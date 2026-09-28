<script lang="ts">
  import { untrack } from "svelte";
  import Modal from "./Modal.svelte";

  // Asks for one line of text: Enter submits, Escape cancels.
  let { title, value = "", action, onSubmit, onClose }: {
    title: string;
    value?: string;
    action: string;
    onSubmit: (text: string) => void;
    onClose: () => void;
  } = $props();

  let text = $state(untrack(() => value));
  let input: HTMLInputElement | undefined = $state();

  $effect(() => {
    input?.focus();
    input?.select();
  });

  function onKeydown(event: KeyboardEvent) {
    if (event.key === "Enter" && text.trim()) {
      event.preventDefault();
      onSubmit(text.trim());
    }
  }
</script>

<Modal {title} {onClose}>
  <input bind:this={input} bind:value={text} onkeydown={onKeydown} spellcheck="false" autocomplete="off" />
  <p><kbd>↵</kbd> {action} · <kbd>esc</kbd> cancel</p>
</Modal>

<style>
  input {
    width: 100%;
  }

  p {
    margin: 10px 0 0;
    font-size: 11px;
    color: var(--overlay0);
  }
</style>
