## Why

A number inside English prose is read as a Russian cardinal on every route, so
the language flips at the number even when everything around it stays English.
On the native route the Russian words split the English block into `en` / `ru`
segments and a 200 ms join appears at each flip; on the narrated route a Russian
numeral sits inside transliterated English. The numeric span layer picks the
spoken form by shape only — nothing looks at the language of the surroundings.

## What Changes

- The numeric span layer gains an English-context test for a standalone
  cardinal: the nearest word on each side is English prose, or English prose on
  one side and a sentence edge on the other, and the sentence contains no
  Cyrillic word. Punctuation and digits are transparent when the neighbour is
  found; identifiers, URLs, inline code and user-dictionary tokens are not
  English prose.
- English-context cardinals are read in English by a new English numeral reader
  (Latin words in the cardinal register).
- The narrated route transliterates that reading into Cyrillic through curated
  numeral readings, so the register keeps one language.
- The native route leaves the digits in place and annotates them with a
  prose-Latin span, so the number continues the English block and no `ru`
  segment or join is inserted.
- The context is resolved in the layer's single scan from a sentence-language
  map built over the original text, so a phase-inserted Cyrillic fragment cannot
  make an English sentence look Russian, and the near-linear scaling
  requirement holds.
- Russian context keeps today's readings byte for byte.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `text-pipeline`: the Numbers requirement gains the English-context rule; the
  English reading setting requirement states that numbers follow the resolved
  route; the Span annotation requirement covers a number the pipeline leaves to
  the engine.

## Non-goals

- Numbers that own a unit, percentage, currency, date, time, version, range or
  reference reading — tracked separately per register (#371, #372).
- Spelling the English reading as text on the native route — #370.
- Zero-padded codes, IDs, phone numbers, decimal fractions, signed numbers and
  comma-grouped thousands.
- A year register beyond the cardinal reading.
- Transliteration quality beyond the numeral words.
- The preview dialog and the UI.

## Impact

- `tts-normalize`: `pipeline/numeric/` (context test, English reader),
  `pipeline/mod.rs` (route plumbing at the numeric phase),
  `pipeline/normalizers/english.rs` (curated numeral readings).
- Tests: unit tables in the numeric layer, pipeline route tests, golden
  fixtures under `src-tauri/tests/fixtures/pipeline/`.
- Specs: `text-pipeline`; `CHANGELOG.md`.
