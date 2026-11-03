# Tasks

## 1. Readings

- [ ] 1.1 Cardinal table: "three" → «фри», "thirty" → «фёти», "hundred" →
      «хандрэд».
- [ ] 1.2 Construct table: "percent" → «пёрсэнт», "the" → «зэ», "thirtieth" →
      «фётис», "hundredth" → «хандрэдс».
- [ ] 1.3 `FUNCTION_WORDS` table ("the" → «зэ», "and" → «энд») consulted before
      `IT_TERMS`.

## 2. Tests

- [ ] 2.1 Update the unit and pipeline expectations that pin an old reading.
- [ ] 2.2 Add a pipeline test that reads a phrase with "the" and "and" and gets
      the curated readings, in any case.
- [ ] 2.3 Regenerate the Silero fixtures that contain an affected word.

## 3. Docs and validation

- [ ] 3.1 `CHANGELOG.md` `[Unreleased]` note.
- [ ] 3.2 `cargo test -p tts-normalize`, golden fixtures, perf budgets, clippy
      and fmt.
- [ ] 3.3 Archive the change and sync the delta specs.
