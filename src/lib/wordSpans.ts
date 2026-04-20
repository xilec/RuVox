/**
 * Split a text fragment into per-word spans with data-orig-start/data-orig-end
 * attributes so that U5 word-highlighting can target individual words rather
 * than whole tokens/paragraphs.
 *
 * `startOffset` is the position of `text[0]` within the original source
 * document (so the caller can still use a cursor when identical fragments
 * appear more than once).
 */
export function wrapWordsWithOrigPos(text: string, startOffset: number): string {
  let out = '';
  let i = 0;
  const len = text.length;

  while (i < len) {
    // Run through whitespace as-is (still escaped) — highlighting skips it.
    const wsStart = i;
    while (i < len && /\s/.test(text[i])) i += 1;
    if (i > wsStart) {
      out += escapeHtml(text.slice(wsStart, i));
    }
    if (i >= len) break;

    const wordStart = i;
    while (i < len && !/\s/.test(text[i])) i += 1;
    const word = text.slice(wordStart, i);
    const origStart = startOffset + wordStart;
    const origEnd = startOffset + i;
    out += `<span data-orig-start="${origStart}" data-orig-end="${origEnd}">${escapeHtml(word)}</span>`;
  }

  return out;
}

export function escapeHtml(s: string): string {
  return s
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}
