import MarkdownIt from 'markdown-it';
import hljs from 'highlight.js';

const md = new MarkdownIt({
  html: false,
  breaks: true,
  linkify: true,
  highlight(str, lang) {
    if (lang && hljs.getLanguage(lang)) {
      try {
        return hljs.highlight(str, { language: lang, ignoreIllegals: true }).value;
      } catch {
        return '';
      }
    }
    return '';
  },
});

// U4 v1: clean markdown rendering.
// U5 will add data-orig-start/data-orig-end span attributes for word highlighting.
export function renderMarkdown(source: string): string {
  return md.render(source);
}
