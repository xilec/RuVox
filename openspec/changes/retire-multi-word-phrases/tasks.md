# Tasks: retire-multi-word-phrases

## 1. Curate the words the phrase table was fixing

- [x] Add the five `IT_TERMS` entries the four diverging rows needed:
      `stack` → "стэк", `live` → "лайв", `dry` → "драй", `run` → "ран",
      `edge` → "эдж", with a comment naming the retired table.
- [x] Curate `living` → "ливинг" separately, so the verb form does not compose
      the adjective stem `live` → "лайв", and test it.
- [x] Unit-test the five words, so the readings come from the dictionary rather
      than from a phrase hit.

## 2. Retire the phrase table and its lookup

- [x] Delete `MULTI_WORD_PHRASES` and `phrase_reading` from `english.rs`.
- [x] Drop the exact-phrase step from `EnglishNormalizer::normalize` and
      renumber the lookup order in its doc comment (IT_TERMS → inflection
      composition → transliteration).
- [x] Replace the `multiword_phrases` test and the two phrase cases of
      `case_insensitive` with one table over all fifteen former phrases,
      asserted through the per-word path the pipeline actually uses.
- [x] Add pipeline tests for the former phrases: space, doubled space, tab, line
      break, and a user-dictionary entry that still outranks the built-in
      reading.
- [x] Remove the fifteen multi-word rows from
      `tests/fixtures/transliteration/baseline.csv` (it is a word-level fixture)
      and refresh the four word rows the new entries change (`dry`, `edge`,
      `living`, `run`).

## 3. Behavior lock

- [x] Add the `english_phrases_words` golden fixture (`input.txt` /
      `expected.txt` / `char_map.json`): six former phrases in prose,
      `stack`/`trace` separated by a line break, and the doubled-space form.

## 4. Spec + docs

- [x] Rewrite the `text-pipeline` delta: the `MULTI_WORD_PHRASES` clause of the
      resolution order and the phrase-first scenario are removed; the
      English-words requirement now states that multi-word input has no reading
      of its own, and the scenarios pin the curated words, the separator
      independence, and the user-dictionary precedence. The phase order is
      unchanged, so it is not part of the delta.
- [x] Rewrite the `CHANGELOG.md` `[Unreleased]` note for the retired table.

## 5. Validation

- [x] `nix develop -c just test`
- [x] `nix develop -c just lint`
- [x] `nix develop -c just validate` (`openspec validate --specs --strict`)
- [x] `nix develop -c pnpm dlx @fission-ai/openspec@1.6.0 validate retire-multi-word-phrases --strict`
- [ ] Archive the change (`openspec archive`) before the PR.
