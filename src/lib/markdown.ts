import MarkdownIt from 'markdown-it';
import hljs from 'highlight.js';

function escapeMermaidCode(s: string): string {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

function escapeHtml(s: string): string {
  return s
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}

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

// Override fence renderer to emit plain <div class="mermaid"> for mermaid blocks.
// Default markdown-it wraps highlight() output in <pre><code> which breaks mermaid.run().
md.renderer.rules.fence = (tokens, idx) => {
  const token = tokens[idx];
  const info = token.info.trim();
  if (info === 'mermaid') {
    return `<div class="mermaid">${escapeMermaidCode(token.content)}</div>\n`;
  }
  const highlighted =
    md.options.highlight?.(token.content, info, '') ?? escapeHtml(token.content);
  const langClass = info ? ` class="language-${escapeHtml(info)}"` : '';
  return `<pre><code${langClass}>${highlighted}</code></pre>\n`;
};

export function renderMarkdown(source: string): string {
  return md.render(source);
}
