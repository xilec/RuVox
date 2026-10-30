# Change: spelling-rule transliteration for English words

## Why

The fallback transliterator is a digraph table: it knows `tion` and `sh`, but
not silent `e`, vowel digraphs, soft `c`/`g`, or voiced `s`. Every word outside
`IT_TERMS` therefore reads as a letter-by-letter approximation (`file` was
"филе", `page` was "паге"), and the dictionary carries entries whose only job
is to patch those gaps. The #292 review named two visible consequences:
`package` is curated as the translation "пакет", and `feature` as "фича",
although the reading should be phonetic (`пэкэдж`, `фиче`).

## What Changes

- **Rule-based transliteration**: replace the Aho-Corasick digraph table with a
  documented English spelling-rule scanner covering silent/magic `e`, vowel
  digraphs, soft `c`/`g`, voiced `s` between vowels and in `-sion`/`-sure`,
  `-tion`/`-sion`/`-ture`/`-cial`/`-tial`, a `th` decision table,
  `ck`/`qu`/`x`/`-le`, and the common `-age`/`-ive`/`-ate`/`-ade` endings.
- **Prune redundant `IT_TERMS` entries**: entries whose exact reading the rules
  now produce are removed (`file`, `page`, `type`, `module`, ...); the two
  semantic translations named in #292 are replaced by rule readings and kept
  only where they guard inflections. All-caps guards (`TEST`, `DATA`, `APP`),
  composition guards (`living`, `use`, `feature`), and irregulars
  (`kubernetes`, `nginx`) stay.
- **Tests and fixtures**: rule-class unit tests, the regenerated dictionary
  baseline, the curated inflected-form fixture, and a golden pipeline fixture
  for the new readings.
- **Docs and spec**: `docs/contributing.md` explains the rule layer and when an
  `IT_TERMS` entry is still justified; the `text-pipeline` spec describes the
  new fallback.

## Non-goals

- Full English grapheme-to-phoneme conversion, stress placement, or vowel
  reduction; the rules remain an approximation, and irregular words stay in
  `IT_TERMS`.
- Changing the code-identifier path (`CODE_WORDS`) or the user dictionary.
- Changing which engines read Latin natively (that is #311/#307).
