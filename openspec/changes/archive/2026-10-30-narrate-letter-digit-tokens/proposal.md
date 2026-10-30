## Why

Tokens that mix Latin letters and digits (`p95`, `v2`, `x86`, `sha256`) match
no prose phase: the English-word rule only sees pure letter runs, the number
phases only see standalone numbers, and the identifier rules only see
camelCase/snake_case/kebab-case. For a Russian-only engine the Latin letters
are silently dropped by the model alphabet, so latency figures, versions, sizes
and hashes disappear from the audio while remaining in the on-screen text.

## What Changes

- Extend the English-word phase with a letter+digit token pattern and resolve
  such tokens by profile:
  - a `Narrate` Latin route reads them in Russian — a curated whole-token
    reading wins (`mp3` → "эм пэ три"); otherwise the token is split into
    maximal letter and digit runs and each run is read by its existing owner
    (letter names/`IT_TERMS`/`CODE_WORDS`/transliteration, Russian numerals);
  - a `Native` Latin route leaves the token intact and annotates it as a
    code-identifier span, so the adapter sends it to the engine instead of
    letting the number phase split it.
- Keep camelCase/snake_case identifiers with the identifier phase: mixed-case
  tokens are excluded from the new pattern.
- Add unit tests and golden fixtures for both the Silero (narrated) and
  native-English (left alone) profiles.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `text-pipeline`: the English-word requirement gains the letter+digit token
  behavior and the native annotation rule.

## Impact

- Affected code: `src-tauri/src/pipeline/mod.rs` (pattern, English-phase
  resolution, native annotation) and
  `src-tauri/src/pipeline/normalizers/code.rs` (read-only `CODE_WORDS` lookup).
- Affected fixtures: `src-tauri/tests/fixtures/pipeline/{silero,native_en}/`.
- User-visible: `p95`, `v2`, `x86`, `sha256` and similar tokens are spoken
  instead of disappearing; native engines keep them.
