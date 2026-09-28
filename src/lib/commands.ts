/**
 * An action of the app. The one `commands` list in App.svelte drives the keyboard, the
 * palette (⌘K, then `>`) and the help (`?`), so every action has a key and shows up in
 * all three.
 */
export interface Command {
  id: string;
  label: string;
  /**
   * `KeyboardEvent.key` values that run it. A key starting with ⌘ is only shown: the
   * native menu or App.svelte handles it.
   */
  keys: string[];
  run: () => void;
}

const glyphs: Record<string, string> = {
  ArrowDown: "↓",
  ArrowUp: "↑",
  Backspace: "⌫",
  Enter: "↵",
  Escape: "esc",
  " ": "space",
};

/** How a key reads on screen. */
export const glyph = (key: string) => glyphs[key] ?? key;
