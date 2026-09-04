## Context

The pipeline (`src-tauri/src/pipeline/mod.rs::process_with_char_mapping`) runs
a strictly ordered set of phases over `TrackedText`; pronunciation knowledge
lives in built-in tables consulted at four sites: the prose English-word
chain, `CodeIdentifierNormalizer` parts, `URLPathNormalizer::transliterate_word`,
and code-block narration through the identifier path. `EnglishNormalizer`
already has an unused `custom_terms` hook (tests only). Storage is split
between a data root (history, audio) and a config root (`config.json`,
atomic writes, `.bak` recovery). The Settings UI is a single modal with
nested-modal precedent (`CleanupCacheModal`); frontend state is
props/`useState` + the typed `invoke` wrappers in `src/lib/tauri.ts`.

## Goals / Non-Goals

**Goals:**

- Dictionary entries win over built-ins at every lookup site, uniformly.
- Close the alnum-token gap (`IPv6`, `mp3` in prose are captured by no phase
  today) for dictionary entries without changing behavior for non-entries.
- Hand-editable persistence; zero effect when the dictionary is empty.

**Non-Goals:**

- Changing how non-dictionary words are normalized (golden fixtures for
  built-in behavior must not change).
- Cyrillic `from` (#277), phrases (#278), hyphens/punctuation in `from`
  (#279) — filed as follow-ups.

## Decisions

### D1. TOML file format (rejected: JSON)

The dictionary is the one RuVox data file meant for hand editing. TOML maps
the flat entries to a plain key-value table where keys preserve the typed
case (display for free), duplicate keys are a parse error (JSON arrays can
carry silent duplicates), and `#` comments let users annotate. JSON was
rejected despite being the format of every other RuVox file: no comments,
quoted-noise for hand editing, and import/export would still need a
dedicated schema. Cost accepted: new direct dependency `toml` (serde-based,
MIT — passes `cargo deny`). Structure: `version = 1` + `[entries]` table;
loader dedupes keys that differ only by case (last wins, `tracing` warning).

### D2. A dedicated pre-pass phase, not an extended English regex (rejected:
widening `re_english_words`)

Widening `\b([A-Za-z]+)\b` to alnum tokens would change matching for *every*
word and force every branch of the prose resolution chain to handle digit
tails. Instead the dictionary gets its own pass with
`\b[A-Za-z0-9]*[A-Za-z][A-Za-z0-9]*\b` (≥1 letter, so pure numbers stay with
the number phase), placed **between code-identifier splitting and the
English-word phase**: after splitting, entries apply to identifier *parts*,
never whole identifiers; before English resolution, entries beat `IT_TERMS`,
letter spelling, and transliteration. Only exact key hits are replaced —
everything else flows on untouched, so built-in behavior and golden fixtures
are unaffected. The pass is skipped entirely when the map is empty
(byte-identical output, no per-run cost). Sites that split tokens themselves
(identifiers, URLs) consult the map on their existing parts/words before
their built-in tables.

### D3. One home for the dictionary map (rejected: snapshots inside each
normalizer)

`TTSPipeline` owns the single `UserDictionary`; the pre-pass uses it directly
and the identifier/URL paths receive `&UserDictionary` through their existing
signatures. Cloning snapshots into each normalizer was rejected — two homes
for one rule drift apart (craft rule: dictionaries have one home). The
existing `EnglishNormalizer::custom_terms`/`add_custom_terms` hook is
superseded by the pre-pass and gets removed (it is unused in production; its
test coverage moves to the pre-pass).

### D4. Runtime refresh via a pipeline setter (rejected: per-request
pipeline rebuild)

`save`/`import` commands persist the file, then call
`TTSPipeline::set_user_dictionary` under the existing `Arc<Mutex<..>>` — the
same pattern as `set_code_block_mode` after `update_config`. Startup loads
the file once in `lib.rs` next to pipeline construction.

### D5. New `src-tauri/src/dictionary/` module (rejected: extending
`storage/`)

`dictionary/` owns the entry type, validation, and the TOML store. The seam
is lifecycle, not location: history/audio (`storage/`) are generated data
with eviction; the dictionary is user-authored config-state with
merge/replace import. `StorageService` stays untouched; the dictionary path
hangs off the same config root.

### D6. Save is full-replace, all-or-nothing; merge happens in memory

`save_user_dictionary` validates the whole list, then writes the file
atomically — one invalid entry rejects the save, file unchanged. Import
`merge` builds the merged map in memory (imported wins on key collision,
invalid skipped and counted) and performs one atomic write + one pipeline
refresh. No partial states on disk, ever.

### D7. Frontend: modal-local state, Tauri drag-drop events

The editor holds the entry list in `useState` inside the modal (no zustand
store — nothing outside the modal needs it). Styling via CSS Modules +
`--mantine-*`/`--ruvox-*` tokens. Drag&drop uses Tauri's drag-drop events
(the webview `dragDropEnabled` default intercepts HTML5 drops; the Tauri
events yield file paths directly), with the file dialog as the parallel
path.

## Risks / Trade-offs

- [Pre-pass regex matches inside already-replaced regions] → it cannot:
  `TrackedText` skips replaced regions, and earlier phases emit Cyrillic,
  which the ASCII-only regex cannot match. Pinned by the empty-dictionary
  fixture scenario.
- [Extra regex pass costs normalization time] → skipped when the dictionary
  is empty; otherwise one linear pass comparable to the existing phases —
  the near-linear scaling requirement still holds.
- [Deliberate "user wins everywhere" edges] → a single-letter entry (e.g.
  "C") can break special terms like "C++"; an entry ("UTF") can replace a
  part of a hyphenated token ("UTF-8"). Accepted per the explicit
  user-wins-everywhere decision; the general fix is #279.
- [Structured readings (sizes, dates, versions) consume tokens before the
  dictionary] → an entry like "GB" will not override the size-unit reading.
  Accepted: the dictionary targets words, not structured units.
- [Alnum entries inside URL hosts] → URL reading splits digit runs in some
  paths, so an alnum key may not match there; prose and identifiers are the
  guaranteed sites. Pinned by fixtures, not promised in specs.

## Migration Plan

No migration: the dictionary file is new; absence means empty. Rollback is
reverting the change — a leftover `user_dictionary.toml` is harmless.
