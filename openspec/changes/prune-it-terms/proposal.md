# Proposal: prune-it-terms

## Why

`IT_TERMS` mixes two different things: pronunciation readings (`file` →
"файл", where the digraph rules would say "филе") and Russian semantic
translations (`function` → "функция" instead of the reading "функшн"). It also
carries entries whose curated reading is byte-for-byte what the transliteration
already produces (`class`, `index`, `app`), which do no work.

The review of #283/#285 decided that `IT_TERMS` carries pronunciation readings
only: a listener should hear the English word, not a translation of it. Of the
228 entries, 7 are semantic translations, 66 look redundant in lowercase, and
155 are genuine pronunciation fixes. 27 of the 66 turn out to be load-bearing
in the all-caps path: without them "TEST" or "DATA" would be spelled out letter
by letter, so they stay as caps guards.

## What Changes

- Remove the 7 semantic translations: `function`, `string`, `array`, `object`,
  `repository`, `boolean`, `refactor`. They now read by transliteration
  ("функшн", "стринг", "аррэй", "обджект", "репоситори", "булин",
  "рефактор").
- Remove 39 redundant no-ops: `abstract`, `admin`, `async`, `checkout`,
  `cobol`, `commit`, `company`, `config`, `controller`, `default`, `delphi`,
  `deploy`, `docker`, `documents`, `fortran`, `golang`, `haskell`, `index`,
  `kotlin`, `laravel`, `listener`, `localhost`, `mongo`, `ocaml`, `postgres`,
  `postman`, `prolog`, `protected`, `query`, `redis`, `reload`, `report`,
  `request`, `static`, `toml`, `twenty`, `virtual`, `webpack`. Their readings
  and the readings of their inflected forms are unchanged: composition and
  transliteration produce the same Cyrillic for all of them, which was
  verified word by word.
- Keep 27 no-op entries that are load-bearing in the all-caps path (`app`,
  `best`, `branch`, `class`, `const`, `data`, `doc`, `docs`, `flask`, `hot`,
  `impl`, `jira`, `mock`, `nat`, `next`, `opt`, `png`, `pod`, `pull`, `repo`,
  `smell`, `spec`, `sync`, `ten`, `test`, `web`, `world`): their lowercase
  reading is the transliteration, but the curated lookup is the only path that
  keeps an uppercase token from being spelled letter by letter.
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
