import DOMPurify, { type Config as DOMPurifyConfig } from 'dompurify';
import hljs from 'highlight.js';
import {
  annotateHtmlWords,
  extractTextForTts,
  isBlockSeparator,
} from './htmlText';
import type { EntryFormat } from './tauri';

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
  // Keep data- attributes so that word-highlighting spans survive.
  ALLOW_DATA_ATTR: true,
};

/**
 * Sanitize an HTML string with the shared viewer configuration.
 *
 * Used both at render time and at ingestion time (html-ingestion spec): the
 * sanitized output is what gets stored as `TextEntry.html_source`, so the
 * stored markup, the extraction input, and the rendered document are the
 * same content.
 */
export function sanitizeHtml(raw: string): string {
  // DOMPurify.sanitize with RETURN_DOM=false (default) narrows the return
  // type to string via the config overload.
  return DOMPurify.sanitize(raw, { ...PURIFY_CONFIG, RETURN_DOM: false });
}

/**
 * Text that a preview/normalization request should run on for a given
 * source-format choice (preview-dialog spec): for `html` the markup is
 * sanitized and extracted first, so the preview shows what will actually
 * be narrated; other formats preview the text unchanged.
 */
export function previewTextFor(text: string, format: EntryFormat): string {
  return format === 'html' ? extractTextForTts(sanitizeHtml(text)) : text;
}

/**
 * Sanitize an HTML string and return safe markup for insertion via
 * `dangerouslySetInnerHTML`.
 *
 * After sanitization, `<pre><code class="language-*">` blocks are highlighted
 * by highlight.js so code inside HTML documents gets the same treatment as
 * code inside Markdown, and every word is wrapped in a data-orig span with
 * codepoint offsets into the extracted TTS text (annotateHtmlWords), which
 * enables word highlighting in HTML mode.
 */
export function renderHtml(raw: string): string {
  const container = document.createElement('div');
  // DOMPurify already cleaned the HTML — inserting it here is safe.
  container.innerHTML = sanitizeHtml(raw);
  highlightCodeBlocks(container);
  annotateHtmlWords(container);
  wrapTablesForScrolling(container);
  return container.innerHTML;
}

/**
 * Wrap every `<table>` in a horizontal scroll box. Same reason as the
 * markdown renderer: `overflow-x` on the table element itself does not create
 * a scroll container, so a wide table would overflow the reading pane.
 * `tabindex="0"` makes the box reachable and scrollable from the keyboard
 * (#364 review); the viewer adds the localized role and aria-label after the
 * render, where the active translation is available.
 */
function wrapTablesForScrolling(container: HTMLElement): void {
  container.querySelectorAll('table').forEach((table) => {
    const wrapper = document.createElement('div');
    wrapper.setAttribute('data-scroll-x', '');
    wrapper.setAttribute('tabindex', '0');
    table.parentNode?.insertBefore(wrapper, table);
    wrapper.appendChild(table);
  });
}

/**
 * Walk all `<pre><code class="language-*">` elements in the container and
 * apply highlight.js syntax highlighting in place.
 *
 * Markup that already carries element children (Habr/Shiki spans) is
 * highlighted line by line: such sources separate code lines with `<br>`,
 * which highlight.js cannot see in `textContent`. Rebuilding the block from
 * `textContent` would collapse it into one line and shift every later word
 * span off its extracted-text offset, so the `<br>` separators are kept.
 */
function highlightCodeBlocks(container: HTMLElement): void {
  container.querySelectorAll('pre code').forEach((codeEl) => {
    if (codeEl.firstElementChild) {
      highlightPreformattedLines(codeEl);
      return;
    }
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
}

/**
 * Highlight markup that is already wrapped in elements (Habr/Shiki spans).
 *
 * Such sources separate code lines with `<br>` elements. Rebuilding the block
 * from `textContent` would drop them and shift every later word span, so each
 * line is highlighted on its own and the `<br>` separators are kept — the DOM
 * shape `annotateHtmlWords` walks stays identical to the extracted text.
 * Markup with block-level children is left as authored for the same reason.
 */
function highlightPreformattedLines(codeEl: Element): void {
  const lang = extractLanguage(codeEl.className);
  if (!lang || !hljs.getLanguage(lang)) return;
  // Only <br>-separated markup can be rebuilt line by line without changing
  // what the extractor sees; anything with block-level children is left as
  // authored so the word offsets keep matching original_text.
  const hasBlockChild = Array.from(codeEl.querySelectorAll('*')).some(
    (el) => el.tagName !== 'BR' && isBlockSeparator(el.tagName),
  );
  if (hasBlockChild) return;
  const lines = collectLines(codeEl);
  try {
    codeEl.innerHTML = lines
      .map((line) =>
        hljs.highlight(line, { language: lang, ignoreIllegals: true }).value,
      )
      .join('<br>');
    codeEl.classList.add('hljs');
  } catch {
    // Highlighting failure is non-fatal — display unhighlighted code.
  }
}

/** Collect the code text line by line, treating `<br>` as the separator. */
function collectLines(codeEl: Element): string[] {
  const lines = [''];
  const append = (node: Node): void => {
    for (const child of Array.from(node.childNodes)) {
      if (child.nodeType === Node.TEXT_NODE) {
        lines[lines.length - 1] += (child as Text).data;
      } else if (child.nodeType === Node.ELEMENT_NODE) {
        const el = child as Element;
        if (el.tagName === 'BR') lines.push('');
        else append(el);
      }
    }
  };
  append(codeEl);
  return lines;
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
