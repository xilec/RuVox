import DOMPurify, { type Config as DOMPurifyConfig } from 'dompurify';
import hljs from 'highlight.js';

/**
 * DOMPurify configuration.
 *
 * We allow only a curated subset of HTML elements and attributes so that
 * arbitrary user-supplied HTML (e.g. from clipboard) cannot smuggle dangerous
 * content into the webview.  Script execution is already blocked by the Tauri
 * CSP, but we sanitize defensively anyway.
 */
const PURIFY_CONFIG: DOMPurifyConfig = {
  USE_PROFILES: { html: true },
  // Strip all event handlers (onclick, onload, …) and javascript: URIs.
  FORBID_ATTR: ['style'],
  FORBID_TAGS: ['script', 'style', 'iframe', 'object', 'embed', 'form', 'input'],
  // Keep data- attributes so that future word-highlighting spans survive.
  ALLOW_DATA_ATTR: true,
};

/**
 * Sanitize an HTML string and return safe markup for insertion via
 * `dangerouslySetInnerHTML`.
 *
 * After sanitization, `<pre><code class="language-*">` blocks are highlighted
 * by highlight.js so code inside HTML documents gets the same treatment as
 * code inside Markdown.
 */
export function renderHtml(raw: string): string {
  // DOMPurify.sanitize with RETURN_DOM=false (default) returns a string; the
  // TrustedHTML union type requires an explicit cast through unknown first.
  const clean = DOMPurify.sanitize(raw, { ...PURIFY_CONFIG, RETURN_DOM: false }) as unknown as string;
  return highlightCodeBlocks(clean);
}

/**
 * Walk all `<pre><code class="language-*">` elements in an HTML string and
 * apply highlight.js syntax highlighting in place.
 *
 * We parse, mutate, and re-serialize via a temporary `<div>` so we never
 * insert raw HTML manually.
 */
function highlightCodeBlocks(html: string): string {
  const container = document.createElement('div');
  // DOMPurify already cleaned the HTML — inserting it here is safe.
  container.innerHTML = html;

  container.querySelectorAll('pre code').forEach((codeEl) => {
    const lang = extractLanguage(codeEl.className);
    if (lang && hljs.getLanguage(lang)) {
      try {
        const result = hljs.highlight(codeEl.textContent ?? '', {
          language: lang,
          ignoreIllegals: true,
        });
        codeEl.innerHTML = result.value;
        codeEl.classList.add('hljs');
      } catch {
        // Highlighting failure is non-fatal — display unhighlighted code.
      }
    } else {
      // Auto-detect when no explicit language is given.
      try {
        const result = hljs.highlightAuto(codeEl.textContent ?? '');
        if (result.relevance > 5) {
          codeEl.innerHTML = result.value;
          codeEl.classList.add('hljs');
        }
      } catch {
        // Auto-detect failure is non-fatal.
      }
    }
  });

  return container.innerHTML;
}

/**
 * Extract a highlight.js language identifier from a CSS class string.
 *
 * Handles both `language-rust` and `lang-rust` prefixes used by various
 * HTML-generating tools.
 */
function extractLanguage(className: string): string | null {
  const match = /(?:language|lang)-(\S+)/.exec(className);
  return match ? match[1] : null;
}
