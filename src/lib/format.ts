import type { Link, LinkStatus, Live, Session, Topic } from "./api.ts";

/** A duration in seconds, in its two largest units: `3d 4h`, `5h 10m`, `12m`. */
export function duration(seconds: number | null | undefined): string {
  if (seconds == null) return "—";
  const m = Math.floor(seconds / 60);
  const h = Math.floor(m / 60);
  const d = Math.floor(h / 24);
  if (d) return h % 24 ? `${d}d ${h % 24}h` : `${d}d`;
  if (h) return m % 60 ? `${h}h ${m % 60}m` : `${h}h`;
  return `${m}m`;
}

export const ago = (at: number, now: number) => (now - at < 60 ? "just now" : `${duration(now - at)} ago`);

export const date = (at: number) =>
  new Date(at * 1000).toLocaleDateString(undefined, { day: "numeric", month: "short" });

/** How urgent herdr's states are, the most first. */
const urgency = ["blocked", "done", "working", "unknown", "idle"];

/** The state that matters most among a session's agents, or `null` when it runs none. */
export function sessionState(session: Session | undefined): string | null {
  if (!session?.running) return null;
  const states = session.agents.map((a) => a.status);
  return urgency.find((s) => states.includes(s)) ?? (states.length ? states[0]! : "empty");
}

export const sessionLabel = (session: Session | undefined): string => {
  const state = sessionState(session);
  if (state === null) return "not started";
  if (state === "empty") return "no agent";
  const agent = session!.agents.find((a) => a.status === state);
  return agent ? `${agent.name} · ${state}` : state;
};

export const linkStatus = (live: Live, link: Link): LinkStatus | undefined => live.links[link.url];

/** A link's text: its title once read, else its URL without the scheme. */
export function linkTitle(live: Live, link: Link): string {
  const status = linkStatus(live, link);
  if (status?.title) return status.title;
  return link.url.replace(/^https?:\/\//, "");
}

/** Where a link opens: the address its source gave, else the one typed. */
export function linkUrl(live: Live, link: Link): string | null {
  const url = linkStatus(live, link)?.url || link.url;
  return /^https?:\/\//.test(url) ? url : null;
}

export const kindGlyph: Record<Link["kind"], string> = {
  linear_issue: "◆",
  pull_request: "⇅",
  github_issue: "◎",
  slack: "#",
  web: "↗",
};

export const nextStep = (topic: Topic) => topic.steps.find((s) => s.done_at === null) ?? null;

export const isBlocked = (topic: Topic) => topic.blocks.some((b) => b.until === null);

export type Row =
  | { kind: "step"; id: number; step: Topic["steps"][number] }
  | { kind: "link"; id: number; link: Link }
  | { kind: "note"; id: number; note: Topic["notes"][number] }
  | { kind: "block"; id: number; block: Topic["blocks"][number] }
  | { kind: "rework"; id: number; rework: Topic["reworks"][number] };

/** The parts of a topic its detail lists, in order: steps, links, notes (newest first), blocks, rework. */
export function rows(topic: Topic): Row[] {
  return [
    ...topic.steps.map((step): Row => ({ kind: "step", id: step.id, step })),
    ...topic.links.map((link): Row => ({ kind: "link", id: link.id, link })),
    ...[...topic.notes].reverse().map((note): Row => ({ kind: "note", id: note.id, note })),
    ...[...topic.blocks].reverse().map((block): Row => ({ kind: "block", id: block.id, block })),
    ...[...topic.reworks].reverse().map((rework): Row => ({ kind: "rework", id: rework.id, rework })),
  ];
}
