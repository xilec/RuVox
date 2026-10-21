# Proposal: prune-it-terms

## Why

`IT_TERMS` mixes two different things: pronunciation readings (`file` →
"файл", where the digraph rules would say "филе") and Russian semantic
translations (`function` → "функция" instead of the reading "функшн"). It also
carries entries whose curated reading is byte-for-byte what the transliteration
already produces (`class`, `index`, `app`), which do no work.

The review of #283/#285 decided that `IT_TERMS` carries pronunciation readings
only: a listener should hear the English word, not a translation of it. Of the
228 entries, 7 are semantic translations, 66 are no-ops, and 155 are genuine
pronunciation fixes.

## What Changes

- Remove the 7 semantic translations: `function`, `string`, `array`, `object`,
  `repository`, `boolean`, `refactor`. They now read by transliteration
  ("функшн", "стринг", "аррэй", "обджект", "репоситори", "булин",
  "рефактор").
- Remove 65 redundant no-ops: `abstract`, `admin`, `app`, `async`, `best`,
  `branch`, `checkout`, `class`, `cobol`, `commit`, `company`, `config`,
  `const`, `controller`, `data`, `default`, `delphi`, `deploy`, `doc`,
  `docker`, `docs`, `documents`, `flask`, `fortran`, `golang`, `haskell`,
  `hot`, `impl`, `index`, `jira`, `kotlin`, `laravel`, `listener`,
  `localhost`, `mock`, `mongo`, `nat`, `next`, `ocaml`, `opt`, `png`, `pod`,
  `postgres`, `postman`, `prolog`, `protected`, `pull`, `query`, `redis`,
  `reload`, `repo`, `report`, `request`, `smell`, `spec`, `static`, `sync`,
  `ten`, `test`, `toml`, `twenty`, `virtual`, `web`, `webpack`, `world`.
  Their readings and the readings of their inflected forms are unchanged:
  composition and transliteration produce the same Cyrillic for all of them,
  which was verified word by word.
- Keep `living` even though its curated reading equals the raw transliteration:
  without the entry it would compose the curated `live` stem into "лайвинг",
  while the word is /ˈlɪvɪŋ/. The entry shadows that composition.
- Read `method` by transliteration ("месод"): "th" maps to "с", which is closer
  to the English pronunciation than the curated "метод".

## Impact

- Affected specs: `text-pipeline` (requirement "English words, abbreviations,
  and transliteration").
- Affected code: `IT_TERMS` in `normalizers/english.rs`, the readings fixture
  `dictionary_baseline.csv`, `inflected_forms.csv`, one golden fixture, and the
  dictionary/term unit tests.
- User-visible: `function`, `string`, `array`, `object`, `repository`,
  `boolean`, and `refactor` are now read as English words instead of Russian
  translations; `method` reads "месод"; every other reading stays the same.
