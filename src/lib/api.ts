// Typed wrappers of the Rust commands; see src-tauri/src/lib.rs.
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface Item {
  id: number;
  title: string;
}

export type Theme = "system" | "light" | "dark";

/** Mirrors src-tauri/src/settings.rs. */
export interface Settings {
  theme: Theme;
  search_limit: number;
}

export interface Snapshot {
  items: Item[];
  settings: Settings;
  /** The label of the action ⌘Z would undo. */
  undo: string | null;
  /** The label of the action ⌘⇧Z would redo. */
  redo: string | null;
}

export interface Match {
  /** The candidate's position in the list searched. */
  index: number;
  score: number;
  /** The code point positions to highlight. */
  indices: number[];
}

export const snapshot = () => invoke<Snapshot>("snapshot");
export const add = (title: string, index: number) => invoke<Snapshot>("add", { title, index });
export const rename = (id: number, title: string) => invoke<Snapshot>("rename", { id, title });
export const remove = (id: number) => invoke<Snapshot>("remove", { id });
export const setSettings = (settings: Settings) => invoke<Snapshot>("set_settings", { settings });
export const undo = () => invoke<Snapshot>("undo");
export const redo = () => invoke<Snapshot>("redo");
export const searchItems = (query: string) => invoke<Match[]>("search_items", { query });
export const fuzzy = (query: string, candidates: string[]) => invoke<Match[]>("fuzzy", { query, candidates });

/** An item of the native menu was chosen: `settings`, `undo` or `redo`. */
export const onMenu = (handler: (id: string) => void): Promise<UnlistenFn> =>
  listen<string>("menu", (event) => handler(event.payload));
