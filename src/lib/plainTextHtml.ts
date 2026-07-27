import { wrapWordsWithOrigPos } from './wordSpans';

/**
 * Render plain text as verbatim HTML for the viewer's "plain" mode.
 *
 * Markdown-like markup (`#`, `**`, backticks, …) is NOT interpreted — every
 * character is HTML-escaped and shown as-is. Each word is wrapped in a
 * data-orig-* span so word-highlighting works (same approach as markdown),
 * and lines are joined with <br>. Offsets track each line's position within
 * the original source text.
 */
export function plainToWordHtml(s: string): string {
  // Split on newlines so we can insert <br> between lines while still
  // wrapping each word in a data-orig-* span.  Offsets track the position of
  // each line within the original source text, in codepoints (see
  // wrapWordsWithOrigPos).
  const lines = s.split('\n');
  const parts: string[] = [];
  let offset = 0;
  for (let i = 0; i < lines.length; i += 1) {
    parts.push(wrapWordsWithOrigPos(lines[i], offset));
    offset += Array.from(lines[i]).length + 1; // +1 for the consumed \n
    if (i < lines.length - 1) parts.push('<br>');
  }
  return parts.join('');
}
