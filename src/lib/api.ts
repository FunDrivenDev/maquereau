// Typed wrappers of the Rust commands; see src-tauri/src/lib.rs.
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type Slot = "feature" | "bug_run" | "exploration" | "tooling";
export type Stage = "queued" | "active" | "done";
export type LinkKind = "linear_issue" | "pull_request" | "github_issue" | "slack" | "web";
export type Tone = "open" | "progress" | "review" | "attention" | "done" | "closed";

export const slots: { slot: Slot; label: string }[] = [
  { slot: "feature", label: "Feature" },
  { slot: "bug_run", label: "Bug / Run" },
  { slot: "exploration", label: "Exploration" },
  { slot: "tooling", label: "Tooling" },
];

export const slotLabel = (slot: Slot) => slots.find((s) => s.slot === slot)?.label ?? slot;

/** Unix time, in seconds. */
export type Time = number;

export interface Step {
  id: number;
  text: string;
  done_at: Time | null;
}

export interface Note {
  id: number;
  text: string;
  at: Time;
}

export interface Link {
  id: number;
  /** The URL, or a bare Linear key such as `BIM-123`. */
  url: string;
  kind: LinkKind;
}

export interface Block {
  id: number;
  reason: string;
  since: Time;
  until: Time | null;
}

export interface Rework {
  id: number;
  reason: string;
  at: Time;
}

export interface Initiative {
  id: string;
  name: string;
  url: string;
}

/** Mirrors `Topic` in src-tauri/src/model.rs. */
export interface Topic {
  id: number;
  title: string;
  slot: Slot;
  stage: Stage;
  created_at: Time;
  started_at: Time | null;
  finished_at: Time | null;
  steps: Step[];
  notes: Note[];
  links: Link[];
  blocks: Block[];
  reworks: Rework[];
  initiative: Initiative | null;
  session: string;
  folder: string;
}

/** Seconds; see src-tauri/src/flow.rs. */
export interface Times {
  lead: number;
  cycle: number | null;
  blocked: number;
}

export interface SlotStats {
  slot: Slot;
  done: number;
  in_progress: number;
  lead_median: number | null;
  cycle_median: number | null;
  cycle_p85: number | null;
  blocked_share: number | null;
  reworks: number;
  reworked: number;
}

export interface LinkStatus {
  title: string;
  state: string;
  tone: Tone;
  url: string;
  comments: number;
  last_comment_at: string | null;
  initiatives: Initiative[];
}

export interface Label {
  name: string;
  color: string;
}

/** Mirrors `Issue` in src-tauri/src/live.rs. */
export interface Issue {
  key: string;
  title: string;
  url: string;
  state: string;
  tone: Tone;
  /** 0 none, 1 urgent … 4 low. */
  priority: number;
  initiatives: Initiative[];
  /** Markdown. */
  description: string;
  project: string | null;
  labels: Label[];
  estimate: number | null;
  /** `YYYY-MM-DD`. */
  due_date: string | null;
  /** ISO 8601. */
  created_at: string;
  /** ISO 8601. */
  updated_at: string;
  parent: { key: string; title: string } | null;
  assignee: string | null;
  /** Assigned to Raphaël. */
  mine: boolean;
}

/** An issue of the backlog's tree, which lists each issue followed by its sub-issues. */
export interface BacklogEntry {
  issue: Issue;
  /** 0 at the top, 1 for a sub-issue… */
  depth: number;
  /** Its sub-issues, listed right after it. */
  children: number;
}

export const priorityLabel = (priority: number) =>
  ["No priority", "Urgent", "High", "Medium", "Low"][priority] ?? "No priority";

export interface Agent {
  name: string;
  /** herdr's state: idle, working, blocked, done or unknown. */
  status: string;
}

export interface Session {
  running: boolean;
  agents: Agent[];
}

export interface Live {
  links: Record<string, LinkStatus>;
  assigned: Issue[];
  sub_issues: Issue[];
  initiatives: Initiative[];
  sessions: Record<string, Session>;
  refreshed_at: Time | null;
  errors: string[];
}

export type Theme = "system" | "light" | "dark";
export type Terminal = "ghostty" | "terminal";

/** Mirrors src-tauri/src/settings.rs. */
export interface Settings {
  theme: Theme;
  search_limit: number;
  terminal: Terminal;
  folder: string;
  refresh_minutes: number;
  session_seconds: number;
  notify_sessions: boolean;
  notify_comments: boolean;
  stats_days: number;
}

export interface Snapshot {
  topics: Topic[];
  /** The flow times of each topic, as `[id, times]` pairs. */
  times: [number, Times][];
  stats: SlotStats[];
  live: Live;
  /** The assigned Linear issues no open topic links to, and their sub-issues. */
  backlog: BacklogEntry[];
  settings: Settings;
  has_linear_key: boolean;
  now: Time;
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

type Id = number;
const change = (command: string, args: Record<string, unknown>) => invoke<Snapshot>(command, args);

export const snapshot = () => invoke<Snapshot>("snapshot");
export const createTopic = (title: string, slot: Slot, activate: boolean) =>
  change("create_topic", { title, slot, activate });
export const topicFromIssue = (key: string, slot: Slot) => change("topic_from_issue", { key, slot });
export const rename = (id: Id, title: string) => change("rename", { id, title });
export const moveTopic = (id: Id, slot: Slot) => change("move_topic", { id, slot });
export const activate = (id: Id) => change("activate", { id });
export const park = (id: Id) => change("park", { id });
export const finish = (id: Id) => change("finish", { id });
export const rework = (id: Id, reason: string) => change("rework", { id, reason });
export const block = (id: Id, reason: string) => change("block", { id, reason });
export const unblock = (id: Id) => change("unblock", { id });
export const addStep = (id: Id, text: string) => change("add_step", { id, text });
export const toggleStep = (id: Id, part: Id) => change("toggle_step", { id, part });
export const moveStep = (id: Id, part: Id, delta: number) => change("move_step", { id, part, delta });
export const addNote = (id: Id, text: string) => change("add_note", { id, text });
export const addLink = (id: Id, text: string) => change("add_link", { id, text });
export const removePart = (id: Id, part: Id) => change("remove_part", { id, part });
export const setInitiative = (id: Id, initiative: Initiative | null) =>
  change("set_initiative", { id, initiative });
export const setSession = (id: Id, session: string) => change("set_session", { id, session });
export const setFolder = (id: Id, folder: string) => change("set_folder", { id, folder });
export const remove = (id: Id) => change("remove", { id });
export const setSettings = (settings: Settings) => change("set_settings", { settings });
export const setLinearKey = (key: string) => change("set_linear_key", { key });
export const undo = () => invoke<Snapshot>("undo");
export const redo = () => invoke<Snapshot>("redo");
export const searchTopics = (query: string) => invoke<Match[]>("search_topics", { query });
export const fuzzy = (query: string, candidates: string[]) => invoke<Match[]>("fuzzy", { query, candidates });
export const refresh = () => invoke<void>("refresh");
export const openSession = (id: Id) => invoke<void>("open_session", { id });
export const openUrl = (url: string) => invoke<void>("open_url", { url });

/** An item of the native menu was chosen: `settings`, `undo` or `redo`. */
export const onMenu = (handler: (id: string) => void): Promise<UnlistenFn> =>
  listen<string>("menu", (event) => handler(event.payload));

/** The background reads of Linear, GitHub and herdr sent a new snapshot. */
export const onSnapshot = (handler: (snapshot: Snapshot) => void): Promise<UnlistenFn> =>
  listen<Snapshot>("snapshot", (event) => handler(event.payload));
