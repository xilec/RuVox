# Design: audit-canonical-readings

## Methodology

The audit compared dictionaries programmatically rather than by ear:

1. Dumped `IT_TERMS` (183 entries), `AS_WORD` (20), `SPECIAL_CASES` (4), and
   the 328 `code_words` entries, then joined them on the shared key.
2. Every shared key whose readings differed was inspected: 11 keys are shared
   between `IT_TERMS` and the abbreviation tables, 44 between `IT_TERMS` and
   `code_words`.
3. Each divergent reading was checked against the English pronunciation and
   against the path that produces it (spelling uses `letter_name`, the
   transliteration fallback uses the digraph table).

## Decisions

### The letter name is "эл", not "эль"

`letter_name('l')` returns "эл", so every spelled-out token already says "эл"
("LOL" → "эл оу эл"), and `SPECIAL_CASES` follows it. `IT_TERMS` was the only
place using the full Russian letter name "эль". Rather than change
`letter_name` (which would alter every spelled abbreviation and the all-caps
suite with it), the four curated readings move to "эл": the curated and the
spelled paths now agree, and "эйч ти эм эл" is how the abbreviation is
pronounced in Russian anyway.

### `code_words` follows prose readings

`code_words` is the identifier table (`getUserData`, `imageUrl`), and `IT_TERMS`
is the prose table. Inside an identifier the same word should sound the same as
in prose, so `hello`, `image`, and `node` adopt the prose readings. `node` keeps
"нода" — the accepted borrowing — rather than the phonetic "ноуд", and `image`
keeps "имидж", the standard adaptation of /ˈɪmɪdʒ/.

### `png` is spelled out

"пнг" contains no vowels and no engine pronounces it as a word; a curated entry
that makes text worse is a defect. "пи эн джи" is consistent with `csv`
"си эс ви", `svg` "эс ви джи", and `pdf` "пэ дэ эф". The entry stays in
`IT_TERMS` both for the reading and as an all-caps guard (`PNG` must not fall
through to the letter spelling).

### What the audit deliberately leaves alone

- Semantic translations (`package` → "пакет", `feature` → "фича", `module` →
  "модуль"): known and documented, but the raw fallback is clearly worse
  ("пакаге", "фитуре", "модюле"). Replacing them requires the transliteration
  rules (silent e, soft c/g, diphthongs), which is a separate change with its
  own listening pass.
- Russian borrowings that are standard in the domain ("интерфейс", "сервис",
  "клиент", "контейнер", "кластер", "промис", "роутер", "кэш"): these are not
  translations the listener has to undo — they are the words a Russian
  developer says.
- `etc` "етс", `pdf` "пэ дэ эф", `mp3` "эм пэ три": idiomatic readings that do
  not follow `letter_name`, kept as intentional adaptations.
- The fallback does not override curated entries: `IT_TERMS` is consulted
  before transliteration, and the remaining overlaps are all caps guards or
  compositions (covered by the prune-it-terms invariants). No duplicate
  readings were removed by this change.

## Verification

- Unit tests for every corrected word in both the prose and the identifier
  path, plus a cross-dictionary consistency test.
- Golden fixture updates (`abbreviation_upper`, `user_dict_empty_noop`,
  `markdown_code_block`) and a regenerated transliteration baseline.
- Manual listening pass over the changed words.
