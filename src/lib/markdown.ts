/**
 * A Linear description as blocks to render as text: no HTML is ever built from it, so
 * nothing written in an issue can run in the app.
 */
export type Block =
  | { kind: "heading"; level: number; text: string }
  | { kind: "paragraph"; text: string }
  | { kind: "item"; depth: number; mark: string; text: string }
  | { kind: "quote"; text: string }
  | { kind: "code"; text: string }
  | { kind: "rule" };

const heading = /^(#{1,6})\s+(.*)$/;
const item = /^(\s*)([-*+]|\d+[.)])\s+(?:\[([ xX])\]\s+)?(.*)$/;
const fence = /^\s*(```|~~~)/;
const rule = /^\s*([-*_])(\s*\1){2,}\s*$/;

/** Markdown's inline syntax, reduced to the text it shows. */
export function inline(text: string): string {
  return text
    .replace(/!\[([^\]]*)\]\([^)]*\)/g, (_, alt: string) => `[image${alt ? `: ${alt}` : ""}]`)
    .replace(/\[([^\]]+)\]\([^)]*\)/g, "$1")
    .replace(/(\*\*|__)(.+?)\1/g, "$2")
    .replace(/`([^`]+)`/g, "$1")
    .replace(/\\([\\`*_{}[\]()#+\-.!>])/g, "$1");
}

export function blocks(markdown: string): Block[] {
  const out: Block[] = [];
  const lines = markdown.replace(/\r\n?/g, "\n").split("\n");
  let i = 0;
  while (i < lines.length) {
    const line = lines[i]!;
    if (!line.trim()) {
      i++;
      continue;
    }
    if (fence.test(line)) {
      const close = fence.exec(line)![1]!;
      const code: string[] = [];
      for (i++; i < lines.length && !lines[i]!.trimStart().startsWith(close); i++) code.push(lines[i]!);
      i++;
      out.push({ kind: "code", text: code.join("\n") });
      continue;
    }
    if (rule.test(line)) {
      out.push({ kind: "rule" });
      i++;
      continue;
    }
    const h = heading.exec(line);
    if (h) {
      out.push({ kind: "heading", level: h[1]!.length, text: inline(h[2]!) });
      i++;
      continue;
    }
    const it = item.exec(line);
    if (it) {
      const [, indent, bullet, check, text] = it;
      const mark = check ? (check === " " ? "☐" : "☑") : /\d/.test(bullet!) ? bullet! : "•";
      out.push({
        kind: "item",
        depth: Math.floor(indent!.replace(/\t/g, "  ").length / 2),
        mark,
        text: inline(text!),
      });
      i++;
      continue;
    }
    if (line.trimStart().startsWith(">")) {
      const quote: string[] = [];
      for (; i < lines.length && lines[i]!.trimStart().startsWith(">"); i++) {
        quote.push(lines[i]!.trimStart().replace(/^>\s?/, ""));
      }
      out.push({ kind: "quote", text: inline(quote.join("\n")) });
      continue;
    }
    const paragraph: string[] = [];
    for (; i < lines.length; i++) {
      const next = lines[i]!;
      if (!next.trim() || fence.test(next) || heading.test(next) || item.test(next) || rule.test(next)) break;
      if (next.trimStart().startsWith(">")) break;
      paragraph.push(next.trim());
    }
    out.push({ kind: "paragraph", text: inline(paragraph.join("\n")) });
  }
  return out;
}
