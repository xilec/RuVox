/**
 * Pure helpers for the user dictionary editor (change `user-dictionary`).
 * Mirrors the backend validation contract (src-tauri/src/dictionary): a
 * source token is Latin letters and digits with at least one letter; the
 * spoken form is a non-empty free-form string that may carry one `+` before
 * a Russian vowel to fix the stress. The Rust side stays the source of truth —
 * these checks exist to give the form instant feedback.
 */

const MAX_FROM_LEN = 64;
const MAX_TO_LEN = 256;

/** The explicit stress mark a spoken form may carry: `+` before the vowel. */
const STRESS_MARK = '+';

/** Russian vowels a stress mark may precede (both cases are accepted). */
const STRESSED_VOWELS = 'аеёиоуыэюя';

/** A single valid source token: Latin letters and digits, ≥1 letter — the
 *  same token shape the dictionary pre-pass regex matches in text. */
const SOURCE_TOKEN_RE = /^(?=.*[A-Za-z])[A-Za-z0-9]+$/;

export interface EntryInput {
  from: string;
  to: string;
}

/** Localized validation failure reason; null when the pair is valid. */
export type DictionaryValidationReason =
  | 'from.required'
  | 'from.charset'
  | 'from.too_long'
  | 'to.required'
  | 'to.stress_malformed'
  | 'to.too_long'
  | 'to.latin_warning';

/** A spoken form split into the text the engine reads and the stressed
 *  vowel's index; null when the mark is malformed. Lives here so the editor
 *  and the preview explain the same rule the backend enforces. */
export function splitStress(to: string): { spoken: string; stressIndex: number | null } | null {
  let spoken = '';
  let stressIndex: number | null = null;
  for (let i = 0; i < to.length; i += 1) {
    const ch = to[i];
    if (ch !== STRESS_MARK) {
      spoken += ch;
      continue;
    }
    if (stressIndex !== null) return null;
    const next = to[i + 1];
    if (next === undefined || !STRESSED_VOWELS.includes(next.toLowerCase())) return null;
    stressIndex = spoken.length;
  }
  return { spoken, stressIndex };
}

/** Validate a from/to pair. Returns the first hard failure, or null when
 *  the pair passes; `to.latin_warning` is a soft warning the caller treats
 *  as non-blocking (the replacement is inserted verbatim). */
export function validateEntryInput(
  entry: EntryInput,
): { reason: DictionaryValidationReason; blocking: boolean } | null {
  const from = entry.from;
  const to = entry.to;

  if (from.length === 0) return { reason: 'from.required', blocking: true };
  if (from.length > MAX_FROM_LEN) return { reason: 'from.too_long', blocking: true };
  if (!SOURCE_TOKEN_RE.test(from)) return { reason: 'from.charset', blocking: true };

  if (to.length === 0) return { reason: 'to.required', blocking: true };
  const parsed = splitStress(to);
  if (parsed === null) return { reason: 'to.stress_malformed', blocking: true };
  if ([...parsed.spoken].length > MAX_TO_LEN) return { reason: 'to.too_long', blocking: true };

  // Soft warning: Latin letters or digits inside `to` reach the TTS engine
  // verbatim — later phases do not re-normalize a dictionary replacement.
  if (/[A-Za-z0-9]/.test(parsed.spoken)) return { reason: 'to.latin_warning', blocking: false };

  return null;
}

/** The dictionary key: lowercased `from` (one entry per word). */
export function entryKey(from: string): string {
  return from.toLowerCase();
}

/** True when `from` already exists in the list (case-insensitively). */
export function isDuplicateFrom(entries: EntryInput[], from: string): boolean {
  const key = entryKey(from);
  return entries.some((entry) => entryKey(entry.from) === key);
}

/** Sort by the lowercased `from` and filter by a case-insensitive substring
 *  of either `from` or `to` (matches the backend's sorted listing and the
 *  editor's search box). Generic so DTO fields survive the round-trip. */
export function sortAndFilterEntries<T extends EntryInput>(
  entries: T[],
  query: string,
): T[] {
  const q = query.trim().toLowerCase();
  return [...entries]
    .filter((entry) =>
      q.length === 0
        ? true
        : entry.from.toLowerCase().includes(q) || entry.to.toLowerCase().includes(q),
    )
    .sort((a, b) => entryKey(a.from).localeCompare(entryKey(b.from)));
}

/** True when a text selection is exactly one valid source token (the
 *  preview dialog's "В словарь" quick-add gate). */
export function isSingleSourceToken(selection: string): boolean {
  return SOURCE_TOKEN_RE.test(selection.trim());
}
