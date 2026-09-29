import DOMPurify from "dompurify";
import hljs from "highlight.js/lib/common";
import { Marked } from "marked";
import { markedHighlight } from "marked-highlight";

// Linear's markdown (GitHub's flavour: tables, task lists, strikethrough), its code blocks
// highlighted. What an issue says is not trusted: the HTML goes through DOMPurify, and the
// CSP lets no script run anyway.
const marked = new Marked(
  { gfm: true, breaks: true },
  markedHighlight({
    emptyLangClass: "hljs",
    langPrefix: "hljs language-",
    highlight: (code, lang) =>
      hljs.getLanguage(lang)
        ? hljs.highlight(code, { language: lang }).value
        : hljs.highlightAuto(code).value,
  }),
  {
    renderer: {
      // The CSP loads no image from outside, and Linear's uploads need its login: an image
      // is a link to it.
      image: ({ href, text }) => `<a href="${escape(href)}">🖼 ${escape(text || "image")}</a>`,
    },
  },
);

const escape = (text: string) => text.replace(/[&<>"']/g, (c) => `&#${c.charCodeAt(0)};`);

/** `markdown` as sanitized HTML. */
export function render(markdown: string): string {
  const html = marked.parse(markdown, { async: false });
  return DOMPurify.sanitize(html, { FORBID_TAGS: ["style", "form"] });
}
