# Proposal: Cyrillic dictionary keys

## Why

The dictionary accepts only Latin letters and digits, so it can fix the reading
of `IPv6` but not the stress of `замок`. Now that an entry can carry a stress
mark (#312) and the pipeline renders it for engines that support stress, the
remaining obstacle is the key: a Russian word cannot be entered at all, and that
is exactly the word whose stress an engine gets wrong.

## What Changes

- **`from` accepts a Russian Cyrillic token** — one word of letters and digits
  in a single script (Latin or Russian Cyrillic), at least one letter, up to 64
  characters. Mixed-script tokens and non-Russian Cyrillic letters stay
  rejected, because the pipeline only normalizes Russian.
- **The pre-pass matches Cyrillic words** — a second candidate pattern, run only
  when the dictionary actually contains Cyrillic keys, so a Latin-only
  dictionary pays nothing and a document with no Cyrillic entries keeps the
  current cost. Matching stays exact and case-insensitive.
- **Stress marks work the same for both scripts** — an entry `замок = "зам+ок"`
  renders the mark for an engine with stress support and strips it otherwise;
  the native accentor honors a `+` on any word, not just on its exception list.
- **Editor and docs** — the field error and placeholder mention Russian words,
  and the spec says matching is by exact word form (inflections are separate).

## Capabilities

- Modified: `user-dictionary` (entry charset, matching form), `text-pipeline`
  (dictionary pre-pass scope: Latin and Cyrillic candidate tokens).

## Non-goals

- Inflection-aware matching (`замка`, `замке` need their own entries): stemming
  or ending tolerance is a different design with its own failure modes, and it
  is tracked separately.
- Folding `е` and `ё`: they are different words (`все` / `всё`), and folding
  would make a stress entry rewrite the wrong one.
- Ukrainian, Serbian, or Belarusian letters: RuVox narrates Russian; other
  Cyrillic letters would match text the rest of the pipeline does not handle.
- Multi-word phrases: an entry is still one token.

## Impact

- `src-tauri/src/dictionary/` (validation, key-script queries),
  `src-tauri/src/pipeline/` (second candidate pattern + gating), the editor's
  validation mirror and i18n strings, golden fixtures for both profiles, and the
  performance lock (a Cyrillic-entry run over 1 MB).
