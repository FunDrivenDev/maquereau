<script lang="ts">
  // Text with the characters at `indices` (code point positions) marked.
  let { text, indices = [] }: { text: string; indices?: number[] } = $props();

  const parts = $derived.by(() => {
    const marked = new Set(indices);
    const out: { text: string; mark: boolean }[] = [];
    Array.from(text).forEach((char, i) => {
      const mark = marked.has(i);
      const last = out.at(-1);
      if (last && last.mark === mark) last.text += char;
      else out.push({ text: char, mark });
    });
    return out;
  });
</script>

{#each parts as part, i (i)}{#if part.mark}<mark>{part.text}</mark>{:else}{part.text}{/if}{/each}
