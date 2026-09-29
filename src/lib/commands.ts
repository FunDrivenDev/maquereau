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
  /** Whether the command applies now; a key shared by several goes to the first that does. */
  when?: () => boolean;
  /** Gets the key that ran it; none from the palette. */
  run: (key?: string) => void;
}

export const applies = (command: Command) => command.when?.() ?? true;

const glyphs: Record<string, string> = {
  ArrowDown: "↓",
  ArrowUp: "↑",
  Backspace: "⌫",
  Enter: "↵",
  Escape: "esc",
  Tab: "⇥",
  " ": "space",
};

/** How a key reads on screen. */
export const glyph = (key: string) => glyphs[key] ?? key;
