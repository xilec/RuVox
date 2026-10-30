# Design: spelling-rule transliteration

## Context

`EnglishNormalizer::normalize` resolves a word through `IT_TERMS`, bounded
inflection composition, then `transliterate_simple`. The last step was an
Aho-Corasick table with digraphs (`sh`, `tion`, `ea`, ...) and a soft-`c`
pre-pass. It had no notion of a silent final `e`, no context for `c`/`g`/`s`,
and no way to express `-ture` or `-cial` without more table rows.

## Decisions

### One scanner, ordered rules

`transliterate_word` walks the lowercased word left to right. At each position
it tries, in order:

1. **Whole-suffix rules** — `-tion`, `-sion`, `-ture`, `-cial`/`-tial`,
   `-sure`, `-gine`, `-ule`, and the unstressed `-age`/`-ive`/`-ate`/`-ade`
   endings.
2. **Digraphs** — `ck`, `qu`, `sh`, `ch`, `th`, `ph`, `wh`, `gh`, `ng`, then
   vowel digraphs (`ai/ay`, `ea/ee/ie`, `oa`, `oo`, `ou`, `ow`, `ue`, `ui`,
   `au/aw`, `ew`, `ey`, `oi/oy`).
3. **Context letters** — soft `c`/`g` before `e/i/y`, voiced `s` between
   vowels, `x` as `кс` (or `кз` before a vowel), and the remaining consonants.

The final silent `e` is removed before the scan and marks the nearest preceding
vowel as long (`file` → `файл`, `page` → `пейдж`, `type` → `тайп`). A small
number of contexts suppress or redirect the long reading: `-mple` stays short
(`example`), `-age` after a long word is `эдж` (`package`), `-ive`/`-ate`/`-ade`
are handled as endings, and word-initial `en-/ex-/er-` reads `э`.

### Exceptions stay in `IT_TERMS`

The rules are deliberately approximate. `IT_TERMS` keeps:

- irregular readings (`kubernetes`, `nginx`, `queue`);
- all-caps guards (`TEST`, `DATA`, `APP`, `STRING`) whose removal would make
  the all-caps heuristic spell the token;
- composition guards (`living`, `use`, `feature`) needed by inflected forms.

An entry whose exact reading the rules already produce is removed unless it is
one of those guards. The policy is documented in `docs/contributing.md` and
checked by the dictionary baseline plus the existing all-caps tests.

### `th` decision table

`th` is `с` before a back vowel (`method` → `месод`) and `т` otherwise
(`thin` → `тин`). English function words are irregular and stay in the
dictionary rather than growing a special-case branch.

## Risks

- English orthography is irregular; the baseline moves many rows. The diff is
  reviewed by ear, not auto-accepted, and the manual listening pass is the gate.
- Magic `e` can over-apply to multi-syllable words (`advance`); those readings
  are candidates for `IT_TERMS` if the listening pass rejects them.
- Removing an entry can change an inflected form; composition guards are kept
  for the stems the curated fixture covers.

## Alternatives

- **Extend the digraph table** — does not scale to context rules and keeps the
  redundant entries.
- **Full pronunciation dictionary** — the repository already has `IT_TERMS` for
  irregulars; a second dictionary would duplicate the policy.
- **Vowel reduction** — explicitly out of scope for #306.
