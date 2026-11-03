/**
 * Derivation of an entry title from its text (issue #353).
 *
 * Mirrors `src-tauri/src/storage/title.rs`: the backend seeds a new entry's
 * `title` with this algorithm, and the UI re-runs it for legacy entries whose
 * `title` is `null` so they look identical until edited. The two
 * implementations are locked together by the same case table in
 * `entryTitle.test.ts` and the Rust module's tests — change both at once.
 */

/** Maximum title length in Unicode code points. Mirrors the Rust constant. */
export const TITLE_MAX_CHARS = 200;

/** Collapse every run of Unicode whitespace into a single space. */
function collapseWhitespace(text: string): string {
  return text.replace(/\s+/gu, ' ');
}

/**
 * Strip one leading Markdown heading marker or list bullet. Markers must be
 * followed by whitespace (matching Markdown), so `#hashtag` and `-5` are left
 * alone.
 */
function stripOnePrefix(line: string): string {
  const heading = /^#{1,6}(?:\s+|$)/.exec(line);
  if (heading) return line.slice(heading[0].length);

  const bullet = /^(?:[-*+]|\d{1,9}[.)])(?:\s+|$)/.exec(line);
  if (bullet) return line.slice(bullet[0].length);

  return line;
}

/** Strip repeated leading Markdown structure markers (`##`, `-`, `1.`, ...). */
function stripMarkdownPrefix(line: string): string {
  let current = line;
  for (;;) {
    const next = stripOnePrefix(current);
    if (next === current) return current;
    current = next;
  }
}

/**
 * Derive a display title from `text`.
 *
 * Uses the first non-empty line that yields something after stripping
 * Markdown structure, collapses internal whitespace to single spaces, trims,
 * and caps the result at {@link TITLE_MAX_CHARS} characters. Returns `null`
 * when the text has nothing usable (empty, whitespace-only, or only markup
 * marks).
 */
export function deriveTitle(text: string): string | null {
  for (const rawLine of text.split('\n')) {
    const line = rawLine.trim();
    if (line === '') continue;
    const collapsed = collapseWhitespace(stripMarkdownPrefix(line)).trim();
    if (collapsed === '') continue;
    return Array.from(collapsed).slice(0, TITLE_MAX_CHARS).join('').trimEnd();
  }
  return null;
}

/** Minimal entry shape the display helpers need. */
interface EntryTitleSource {
  title: string | null;
  original_text: string;
}

/**
 * Title shown in the header and the queue: the stored title, or the derived
 * fallback for legacy / cleared entries. Only pathological entries whose text
 * yields no title at all fall back to the raw text, so the identity line is
 * never empty.
 */
export function entryDisplayTitle(entry: EntryTitleSource): string {
  const explicit = entry.title?.trim();
  if (explicit) return explicit;
  return deriveTitle(entry.original_text) ?? entry.original_text.trim();
}
