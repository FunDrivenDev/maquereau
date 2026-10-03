/**
 * The index `event` moves a list's highlight to: ↓, ⌃N or ⌃J one down, ↑, ⌃P or ⌃K one up,
 * wrapping around the `length` rows. `null` when the key does not move through a list.
 */
export function listStep(event: KeyboardEvent, active: number, length: number): number | null {
  const down = event.key === "ArrowDown" || (event.ctrlKey && (event.key === "n" || event.key === "j"));
  const up = event.key === "ArrowUp" || (event.ctrlKey && (event.key === "p" || event.key === "k"));
  if (!down && !up) return null;
  if (!length) return active;
  return (active + (down ? 1 : length - 1)) % length;
}
