import MarkdownIt from 'markdown-it';
import type Token from 'markdown-it/lib/token.mjs';
import type Renderer from 'markdown-it/lib/renderer.mjs';
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
  // The highlight callback returns '' for unknown / missing languages,
  // so a ?? fallback never fires (empty string is not nullish). Use a
  // truthiness check and escape the raw content explicitly when no
  // highlighter result is available.
  const highlightResult = md.options.highlight?.(token.content, info, '');
  const highlighted = highlightResult ? highlightResult : escapeHtml(token.content);
  const langClass = info ? ` class="language-${escapeHtml(info)}"` : '';
  return `<pre><code${langClass}>${highlighted}</code></pre>\n`;
};

/**
 * Render markdown source to HTML with data-orig-start/data-orig-end
 * attributes on inline text spans so that U5 word-highlighting can locate
 * the correct DOM node for each WordTimestamp.
 *
 * We track a searchFrom cursor per render call so that repeated identical
 * text fragments map to distinct, non-overlapping source positions.
 */
export function renderMarkdown(source: string): string {
  // Cursor shared across all text-token renders in this call.
  let searchFrom = 0;

  const origTextRule = md.renderer.rules.text;

  md.renderer.rules.text = (
    tokens: Token[],
    idx: number,
    options: MarkdownIt['options'],
    env: unknown,
    self: Renderer,
  ): string => {
    const content = tokens[idx].content;
    const pos = source.indexOf(content, searchFrom);
    if (pos === -1) {
      // Fallback: text not found at or after cursor (e.g. HTML-decoded entity).
      // Render without position attributes so highlighting gracefully skips it.
      return origTextRule
        ? origTextRule(tokens, idx, options, env, self)
        : escapeHtml(content);
    }
    searchFrom = pos + content.length;
    const escaped = escapeHtml(content);
    return `<span data-orig-start="${pos}" data-orig-end="${pos + content.length}">${escaped}</span>`;
  };

  const html = md.render(source);

  // Restore default rule so other callers / future invocations are not affected.
  md.renderer.rules.text = origTextRule;

  return html;
}
