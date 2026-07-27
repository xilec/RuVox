/**
 * HTML text extraction for TTS plus word-span annotation (html-ingestion spec).
 *
 * A single walker implementation serves both consumers so offsets can never
 * drift between what the TTS pipeline read and what the viewer renders:
 *
 * - `extractTextForTts` builds the plain text sent to synthesis (stored as
 *   `TextEntry.original_text`);
 * - `annotateHtmlWords` wraps every word of the same document in
 *   `<span data-orig-start data-orig-end>` in place, with codepoint offsets
 *   into that same extracted text (the coordinate space of char_map and
 *   WordTimestamp.original_pos — see the position-mapping spec).
 *
 * The walker mirrors the rules of the deleted Rust extractor: chrome tag
 * subtrees are excluded, block-level elements are separated by newlines,
 * inline whitespace collapses (NBSP counts as a space).
 */

const EXCLUDED_TAGS = new Set([
  'nav', 'footer', 'aside', 'script', 'style', 'head', 'noscript', 'template',
  'svg', 'math', 'button', 'select', 'option', 'optgroup', 'datalist',
]);

const BLOCK_TAGS = new Set([
  'p', 'div', 'section', 'article', 'main', 'header',
  'h1', 'h2', 'h3', 'h4', 'h5', 'h6',
  'blockquote', 'pre', 'ul', 'ol', 'li', 'dt', 'dd', 'dl',
  'figure', 'figcaption',
  'table', 'thead', 'tbody', 'tfoot', 'tr', 'th', 'td',
  'details', 'summary', 'br', 'hr',
]);

const cpLen = (s: string): number => Array.from(s).length;

// JS `\s` is Unicode-aware: it covers NBSP (U+00A0) and other space
// separators, so no extra NBSP clause is needed (unlike the Rust original).
const isSpace = (ch: string): boolean => /\s/.test(ch);

interface WalkCtx {
  /** Extracted text built so far; maintained in both modes — word offsets
   * are codepoint positions in this string. */
  text: string;
  /** Codepoint length of `text`, incremented as we append. Measuring
   * `cpLen(ctx.text)` per word instead would be O(n²) on large documents. */
  cpCount: number;
  lastWasSpace: boolean;
  /** When true, words are wrapped in data-orig spans in the walked DOM. */
  wrap: boolean;
}

/** Extract the TTS text from a sanitized HTML string. */
export function extractTextForTts(sanitizedHtml: string): string {
  const doc = new DOMParser().parseFromString(sanitizedHtml, 'text/html');
  const ctx: WalkCtx = { text: '', cpCount: 0, lastWasSpace: true, wrap: false };
  walkChildren(doc.body, ctx);
  return ctx.text.trim();
}

/**
 * Wrap every word in `container` in a data-orig span (in place) and return
 * the extracted text — identical to what `extractTextForTts` produces for
 * the same document, which is what makes highlight offsets line up.
 */
export function annotateHtmlWords(container: Element): string {
  const ctx: WalkCtx = { text: '', cpCount: 0, lastWasSpace: true, wrap: true };
  walkChildren(container, ctx);
  return ctx.text.trim();
}

function walkChildren(el: Element, ctx: WalkCtx): void {
  // Snapshot: wrap mode replaces text nodes while we iterate.
  for (const child of Array.from(el.childNodes)) {
    if (child.nodeType === Node.TEXT_NODE) {
      emitText(child as Text, ctx);
    } else if (child.nodeType === Node.ELEMENT_NODE) {
      walkElement(child as Element, ctx);
    }
  }
}

function walkElement(el: Element, ctx: WalkCtx): void {
  const tag = el.tagName.toLowerCase();
  if (EXCLUDED_TAGS.has(tag)) return;
  if (tag === 'br' || tag === 'hr') {
    pushNewline(ctx);
    return;
  }
  const block = BLOCK_TAGS.has(tag);
  if (block) pushNewline(ctx);
  walkChildren(el, ctx);
  if (block) pushNewline(ctx);
}

function pushNewline(ctx: WalkCtx): void {
  if (ctx.text.length > 0 && !ctx.text.endsWith('\n')) {
    ctx.text += '\n';
    ctx.cpCount += 1;
  }
  ctx.lastWasSpace = true;
}

interface Token {
  str: string;
  /** Codepoint range in ctx.text; set only for word tokens. */
  origStart?: number;
  origEnd?: number;
}

function emitText(node: Text, ctx: WalkCtx): void {
  const raw = node.data;
  const tokens: { isWord: boolean; token: Token }[] = [];

  let i = 0;
  while (i < raw.length) {
    let j = i;
    if (isSpace(raw[i])) {
      while (j < raw.length && isSpace(raw[j])) j += 1;
      // A whitespace run contributes a single collapsed space, and only when
      // the output is not already after whitespace.
      if (!ctx.lastWasSpace) {
        ctx.text += ' ';
        ctx.cpCount += 1;
        ctx.lastWasSpace = true;
      }
      tokens.push({ isWord: false, token: { str: raw.slice(i, j) } });
    } else {
      while (j < raw.length && !isSpace(raw[j])) j += 1;
      // Astral characters are never whitespace, so UTF-16 iteration here
      // never splits a surrogate pair.
      const word = raw.slice(i, j);
      const origStart = ctx.cpCount;
      ctx.text += word;
      ctx.cpCount += cpLen(word);
      ctx.lastWasSpace = false;
      tokens.push({ isWord: true, token: { str: word, origStart, origEnd: ctx.cpCount } });
    }
    i = j;
  }

  if (ctx.wrap && tokens.some((t) => t.isWord)) {
    const frag = document.createDocumentFragment();
    for (const { isWord, token } of tokens) {
      if (isWord) {
        const span = document.createElement('span');
        span.dataset.origStart = String(token.origStart);
        span.dataset.origEnd = String(token.origEnd);
        span.textContent = token.str;
        frag.appendChild(span);
      } else {
        // Original whitespace is kept verbatim in the DOM (visual layout,
        // especially inside <pre>, is unchanged) — only words get spans.
        frag.appendChild(document.createTextNode(token.str));
      }
    }
    node.replaceWith(frag);
  }
}
