# Proposal: read-single-latin-letters

## Why

Single Latin letters are never transliterated by the text pipeline:
`re_english_words` (`\b([A-Za-z][A-Za-z]+)\b`) requires 2+ letters, so `a`,
`I`, `x` pass through to Silero as Latin and silently vanish from the audio
(`Переменная x равна 5` → `Переменная x равна пять`). This violates the
text-pipeline requirement that every remaining English word is replaced with
speakable Cyrillic (Silero cannot read Latin). Found during the epic #109
investigation of text loss on the way to TTS.

## What Changes

- Extend the English-words phase to also match single Latin letters
  (`[A-Za-z]` runs of length 1) and read them by their English letter names
  (`a` → `эй`, `I` → `ай`, `x` → `икс`), reusing the existing letter-name
  spelling table instead of digraph transliteration.
- Unknown-word tracking: single letters resolved via letter-name spelling are
  NOT recorded in the unknown-words map (they are not transliteration
  fallbacks).

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `text-pipeline`: the "English words, abbreviations, and transliteration"
  requirement gains single-letter handling — single Latin letters SHALL be
  read by letter name; the current spec text only covers multi-letter words.

## Non-goals

- Changing pronunciation of multi-letter words, abbreviations, or code
  identifiers (single letters inside identifiers are already handled by the
  code-identifier phase, which runs first).
- Greek/math single symbols (handled by the symbols phase already).
- User-configurable letter pronunciations (covered by the dictionary
  feature, issue #10).

## Impact

- `src-tauri/src/pipeline/mod.rs` — `re_english_words` pattern and the
  single-letter branch in `process_english_tracked`.
- `src-tauri/src/pipeline/normalizers/code.rs` — letter-name table reuse
  (`spell_abbreviation`), possibly made shared.
- Golden fixtures + unit tests pinning single-letter behavior.
- No UI, protocol, or storage changes. Not breaking: output only changes for
  texts that previously emitted raw Latin letters Silero could not read.
