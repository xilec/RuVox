# Proposal: all-caps-word-like-tokens

## Why

Unknown all-caps tokens are always spelled letter by letter
(`AbbreviationNormalizer`), so a long word-like token such as `DEVELOPER` or
`JAVASCRIPT` is read as a string of letter names ("ди-и-ви-и-эл-оу-пи-и-эр"),
which is hard to follow. After the composed-inflections fix (#288) spelling is
the only remaining path for tokens absent from every dictionary, yet it is the
wrong reading for tokens that are really words: `DEVELOPER`, `LINUX`, `UBUNTU`,
`JAVASCRIPT`, `WINDOWS`.

## What Changes

- Add a word-like heuristic for all-uppercase tokens that are absent from
  `IT_TERMS`, `AS_WORD`, and `SPECIAL_CASES`: at least five letters and at
  least one vowel per four letters (counting "y" as a vowel) → read with the
  same digraph transliteration as a lowercase word. Otherwise the token keeps
  the letter-by-letter reading.
- The heuristic is calibrated on a fixed table (below) and locked by unit tests
  and a pipeline test.
- Word-like tokens are recorded in the unknown-words map, because they are
  resolved by transliteration, not by a curated entry.
- Two common tokens that the heuristic would otherwise mispronounce get
  explicit `IT_TERMS` entries, which take precedence: `windows` → "виндовс"
  (the digraph rules would say "виндоус") and `ascii` → "аски" (they would say
  "ассии").

Calibration set (vowels/len, "y" counted):

| token      | vowels/len  | rule        | reading       |
|------------|-------------|-------------|---------------|
| DEVELOPER  | 4/9 (0.44)  | word        | девелопер     |
| LINUX      | 2/5 (0.40)  | word        | линукс        |
| UBUNTU     | 3/6 (0.50)  | word        | убунту        |
| WINDOWS    | 2/7 (0.29)  | curated     | виндовс       |
| JAVASCRIPT | 3/10 (0.30) | word        | джаваскрипт   |
| API        | 2/3 (0.67), len 3 | spelled | эй пи ай |
| HTML       | 0/4         | spelled     | эйч ти эм эль |
| SQL        | 0/3         | spelled     | эс кью эль    |
| JSON       | 1/4, len 4  | spelled     | джейсон (`AS_WORD`) |

## Impact

- Affected specs: `text-pipeline` (requirement "English words, abbreviations,
  and transliteration").
- Affected code: `pipeline/mod.rs` (all-caps branch), `normalizers/abbreviations.rs`
  (the heuristic), `normalizers/english.rs` (two curated tokens).
- User-visible: long all-caps words are now read as words; no change for short
  or vowel-poor abbreviations, or for anything the dictionaries already cover.
