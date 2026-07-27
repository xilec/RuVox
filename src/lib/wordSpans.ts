/**
 * Split a text fragment into per-word spans with data-orig-start/data-orig-end
 * attributes so that U5 word-highlighting can target individual words rather
 * than whole tokens/paragraphs.
 *
 * `startOffset` is the position of `text[0]` within the original source
 * document (so the caller can still use a cursor when identical fragments
 * appear more than once).
 *
 * All offsets — `startOffset` and the emitted data-orig-* values — are in
 * Unicode codepoints, matching the Rust pipeline's char_map contract
 * (position-mapping spec). JS string indices are UTF-16 code units, so we
 * track a codepoint cursor alongside the UTF-16 index; astral characters
 * (emoji, …) count as 1, not 2.
 */
export function wrapWordsWithOrigPos(text: string, startOffset: number): string {
  let out = '';
  let i = 0;
  let cp = 0; // codepoint cursor mirroring the UTF-16 index i
  const len = text.length;
  const cpLen = (s: string): number => Array.from(s).length;

  while (i < len) {
    // Run through whitespace as-is (still escaped) — highlighting skips it.
    const wsStart = i;
    while (i < len && /\s/.test(text[i])) i += 1;
    if (i > wsStart) {
      const ws = text.slice(wsStart, i);
      out += escapeHtml(ws);
      cp += cpLen(ws);
    }
    if (i >= len) break;

    const wordStart = i;
    while (i < len && !/\s/.test(text[i])) i += 1;
    const word = text.slice(wordStart, i);
    // Whitespace boundaries are always BMP, so surrogate pairs are never split.
    const origStart = startOffset + cp;
    cp += cpLen(word);
    const origEnd = startOffset + cp;
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
