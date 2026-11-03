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

/**
 * Unicode `White_Space`, the set Rust's `char::is_whitespace` / `str::trim`
 * use. JS `\s` is not the same set: it adds U+FEFF (BOM) and drops U+0085
 * (NEL), so a title with either would derive differently on the two sides.
 * Keep this module on the Rust set — the shared case table pins it.
 */
const WHITESPACE_RUN = /\p{White_Space}+/gu;
const LEADING_WHITESPACE = /^\p{White_Space}+/u;
const TRAILING_WHITESPACE = /\p{White_Space}+$/u;

/** Trim Unicode `White_Space` from both ends, matching Rust `str::trim`. */
export function trimWhiteSpace(text: string): string {
  return text.replace(LEADING_WHITESPACE, '').replace(TRAILING_WHITESPACE, '');
}

/** Drop trailing Unicode `White_Space`, matching Rust `str::trim_end`. */
function trimEndWhiteSpace(text: string): string {
  return text.replace(TRAILING_WHITESPACE, '');
}

/**
 * Cap a value at `max` Unicode code points, then drop trailing whitespace the
 * cut may expose. Rust does the same (`chars().take(n)` + `trim_end`), so an
 * astral character straddling the limit stays whole on both sides instead of
 * being split into a lone surrogate (which the IPC layer rejects).
 */
export function capCodePoints(value: string, max: number): string {
  return trimEndWhiteSpace(Array.from(value).slice(0, max).join(''));
}

/** Collapse every run of Unicode whitespace into a single space. */
function collapseWhitespace(text: string): string {
  return text.replace(WHITESPACE_RUN, ' ');
}

/**
 * Strip one leading Markdown heading marker or list bullet. Markers must be
 * followed by whitespace (matching Markdown), so `#hashtag` and `-5` are left
 * alone.
 */
function stripOnePrefix(line: string): string {
  const heading = /^#{1,6}(?:\p{White_Space}+|$)/u.exec(line);
  if (heading) return line.slice(heading[0].length);

  const bullet = /^(?:[-*+]|\d{1,9}[.)])(?:\p{White_Space}+|$)/u.exec(line);
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
    const line = trimWhiteSpace(rawLine);
    if (line === '') continue;
    const collapsed = trimWhiteSpace(collapseWhitespace(stripMarkdownPrefix(line)));
    if (collapsed === '') continue;
    return capCodePoints(collapsed, TITLE_MAX_CHARS);
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
  const explicit = entry.title === null ? null : trimWhiteSpace(entry.title);
  if (explicit) return explicit;
  return deriveTitle(entry.original_text) ?? trimWhiteSpace(entry.original_text);
}
