# Design: all-caps-word-like-tokens

## Decisions

- **Threshold: `len >= 5 && vowels * 4 >= len`, "y" counted as a vowel.**
  The issue proposed `vowels >= len/3`, which misses `WINDOWS` (2/7 = 0.29) and
  `JAVASCRIPT` (3/10 = 0.30) — the two most common tokens of the calibration
  set. One vowel per four letters admits both (0.25) while still excluding
  real initialisms: `NGINX` (1/5 = 0.20, and it is curated anyway), `SPARK`
  (1/5), `TMUX` (1/4 but only four letters). Counting "y" is what lets
  `SYSTEM` (1/6 without it) and `SYNTAX` read as words; consonant-only tokens
  (`HTML`, `SQL`) cannot qualify at all. The length floor of five keeps
  four-letter words such as `JSON` and the whole short-abbreviation class
  (`API`, `GPU`, `JWT`) spelled.
- **Where the heuristic lives: `AbbreviationNormalizer`'s module, applied by the
  pipeline.** `AbbreviationNormalizer::normalize` is the all-caps resolver and
  the only caller is `process_english_tracked`. The heuristic is a free function
  `reads_as_word(token)` next to it; the pipeline decides, because it already
  owns the precedence chain (`IT_TERMS` → composition → this branch) and needs
  to route the word-like token through `EnglishNormalizer::normalize(word, true)`
  so it is transliterated and recorded as unknown. Putting the decision inside
  `normalize` would need a second, differently-tracked code path.
- **Dictionaries keep precedence.** `IT_TERMS` is checked before the all-caps
  branch; `AS_WORD` and `SPECIAL_CASES` are checked explicitly before the
  heuristic. `KUBERNETES` → "кубернетис" and `IOS` → "ай оу эс" therefore keep
  their readings even though both tokens are vowel-rich.
- **Curated exceptions.** `windows` → "виндовс": the transliteration produces
  "виндоус", while the issue's calibration table asks for "виндовс".
  `ascii` → "аски": the transliteration produces "ассии", which is worse than
  the conventional Russian reading. `browser` → "браузэр": the transliteration
  produces "броусер", which the manual pass rejected. All three are ordinary
  `IT_TERMS` entries, so the pipeline never reaches the heuristic for them.
- **Unknown-words tracking.** A word-like all-caps token is resolved by
  transliteration, so it belongs in the unknown-words map (the spec already says
  transliterated words are recorded and composed forms are not). The map key is
  lowercased, matching the existing behaviour for every other word.

## Rejected alternatives

- **Spell nothing, transliterate every unknown all-caps token.** Would turn
  `API` into "апи" and `SQL` into "скл", breaking the abbreviation contract.
- **Curate a larger list of all-caps words instead of a rule.** The issue asks
  for a rule, and a dictionary cannot cover the long tail (`TERRAFORM`,
  `PROMETHEUS`). Curated entries remain for the cases where transliteration is
  clearly wrong (`windows`, `ascii`).
- **Raise the ratio to one vowel per three letters.** Loses `WINDOWS`,
  `JAVASCRIPT`, and `BROWSER`, which is the class the issue exists for.
