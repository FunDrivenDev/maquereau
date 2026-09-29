/**
 * Scrolls `container` so `el` shows, touching no other ancestor (unlike `scrollIntoView`,
 * which also shifts the fixed layout around it).
 */
export function reveal(container: HTMLElement, el: HTMLElement, block: "nearest" | "center" = "nearest") {
  const box = container.getBoundingClientRect();
  const rect = el.getBoundingClientRect();
  const top = rect.top - box.top + container.scrollTop;
  if (block === "center") {
    container.scrollTo({ top: top - (container.clientHeight - rect.height) / 2 });
  } else if (rect.top < box.top) {
    container.scrollTo({ top: top - 40 });
  } else if (rect.bottom > box.bottom) {
    container.scrollTo({ top: top + rect.height - container.clientHeight + 8 });
  }
}
