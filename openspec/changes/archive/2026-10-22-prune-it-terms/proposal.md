# Proposal: prune-it-terms

## Why

`IT_TERMS` mixes two different things: pronunciation readings (`file` →
"файл", where the digraph rules would say "филе") and Russian semantic
translations (`function` → "функция" instead of the reading "функшн"). It also
carries entries whose curated reading is byte-for-byte what the transliteration
already produces (`commit`, `index`, `docker`), which do no work in lowercase.

The review of #283/#285 decided that `IT_TERMS` carries pronunciation readings
only: a listener should hear the English word, not a translation of it. Of the
228 entries, 7 are semantic translations, 66 look redundant in lowercase, and
155 are genuine pronunciation fixes. 27 of the 66 turn out to be load-bearing
in the all-caps path: without them "TEST" or "DATA" would be spelled out letter
by letter, so they stay as caps guards; `living` stays because it shadows a
wrong inflection composition; the remaining 38 are removed.

## What Changes

- Remove the 7 semantic translations: `function`, `string`, `array`, `object`,
  `repository`, `boolean`, `refactor`. They now read by transliteration
  ("функшн", "стринг", "аррэй", "обджект", "репоситори", "булин",
  "рефактор"). `string` keeps an entry, but as a caps guard: its reading is
  the transliteration "стринг" instead of the translation "строка", because
  "STRING" is vowel-poor enough for the all-caps rule to spell it out.
- Remove 38 redundant no-ops: `abstract`, `admin`, `async`, `checkout`,
  `cobol`, `commit`, `company`, `config`, `controller`, `default`, `delphi`,
  `deploy`, `docker`, `documents`, `fortran`, `golang`, `haskell`, `index`,
  `kotlin`, `laravel`, `listener`, `localhost`, `mongo`, `ocaml`, `postgres`,
  `postman`, `prolog`, `protected`, `query`, `redis`, `reload`, `report`,
  `request`, `static`, `toml`, `twenty`, `virtual`, `webpack`. Their readings
  and the readings of their inflected forms are unchanged: composition and
  transliteration produce the same Cyrillic for all of them, which was
  verified word by word.
- Keep 28 entries whose lowercase reading is the transliteration but which are
  load-bearing in the all-caps path (`app`, `best`, `branch`, `class`, `const`,
  `data`, `doc`, `docs`, `flask`, `hot`, `impl`, `jira`, `mock`, `nat`, `next`,
  `opt`, `png`, `pod`, `pull`, `repo`, `smell`, `spec`, `string`, `sync`,
  `ten`, `test`, `web`, `world`): the curated lookup is the only path that
  keeps an uppercase token such as "TEST", "DATA", or "STRING" from being
  spelled letter by letter. 27 of them are former no-ops; `string` is a former
  semantic translation that returns with the transliteration reading instead of
  the translation.
- Keep `living` even though its curated reading equals the raw transliteration:
  without the entry it would compose the curated `live` stem into "лайвинг",
  while the word is /ˈlɪvɪŋ/. The entry shadows that composition.
- Read `method` by transliteration ("месод"): "th" maps to "с", which is closer
  to the English pronunciation than the curated "метод".

## Impact

- Affected specs: `text-pipeline` (requirement "English words, abbreviations,
  and transliteration").
- Size: 228 → 183 entries; 45 removed (38 no-ops, 6 semantic translations,
  and the `method` pronunciation override), 28 caps guards kept (27 former
  no-ops plus `string`), one composition guard (`living`).
- Affected code: `IT_TERMS` in `normalizers/english.rs`, the readings fixture
  `dictionary_baseline.csv`, `inflected_forms.csv`, one golden fixture, and the
  dictionary/term unit tests.
- User-visible: `function`, `string`, `array`, `object`, `repository`,
  `boolean`, and `refactor` are now read as English words instead of Russian
  translations; `method` reads "месод"; every other reading stays the same.
