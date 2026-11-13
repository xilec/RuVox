## Context

The pipeline reads English words in prose through a chain the identifier and
inline-code phases never used: `EnglishNormalizer` consults `IT_TERMS`,
`FUNCTION_WORDS`, inflection composition and the context-aware spelling rules,
while `CodeIdentifierNormalizer` resolves a part through `CODE_WORDS` and then
transliterates it character by character from its own table. The two paths
therefore pronounce the same word differently (`view`: «вив» vs «виев»), and
the per-letter table cannot express digraphs at all (`force` → «форке»,
`forge` → «форге»).

The inline-code phase runs before the URL and path phases and always assumes
an identifier, so `` `~/.dsh/cordis.patch.yml` `` is transliterated character by
character into «~/.дсх/кордис.паткх.ймл». The result keeps the markup
characters and creates vowel-less consonant clusters; the engine skips that
fragment instead of narrating it. A path in prose does not have this problem
because the path normalizer splits it into segments and words.

## Decisions

### One curated chain, two entry points

`curated_reading(lower)` in `english.rs` is the single lookup for `IT_TERMS`,
`ENGLISH_WORDS` and `FUNCTION_WORDS`. `EnglishNormalizer::normalize` and the
code normalizer both call it, so a curated word reads the same in prose, in an
identifier part, in inline code and in a path segment. The code normalizer
keeps `CODE_WORDS` ahead of the curated chain because those readings are
technical jargon conventions ("get" → «гет»), not English pronunciation.

### `ENGLISH_WORDS`, not more `IT_TERMS`

`IT_TERMS` is the technical-terms table and the spec constrains it (no
semantic translations, no duplicates of the spelling rules). Everyday words
such as "view", "office" or "session" are not terms. A separate table keeps the
constraints of each table honest, and the spec makes the "one word, one home"
rule explicit so a word never lives in both.

### Identifier parts no longer use the letter table

The per-letter table was the original fallback for any unknown identifier
part, but it is not a pronunciation: it cannot see `ch`, `sh`, silent `e` or
soft `c`. The fallback for an alphabetic part is now the same
`transliterate_simple` the prose phase uses; the letter table remains only for
parts that are not purely alphabetic. This changes existing readings
(`width` → «видт», `apply` → «аппли», `code` → «код», which the golden
fixtures record.

### Punctuation around a word part carries no sound

`forge*` and `worktrees:` reach the identifier normalizer with punctuation
glued to the word, which used to hide the curated reading. The part is trimmed
to its alphanumeric core before resolution when the core is purely alphabetic.
A token with an internal separator (`isolate.worktrees`) is untouched, because
trimming only leading/trailing characters cannot read it as words.

### Paths and flags

An inline-code span whose content starts with `/`, `~`, `./` or `../` and
contains `/` is a filesystem path: it goes through `URLPathNormalizer`, the
same normalizer phase 7 uses for prose, so `~/.dsh/cordis.patch.yml` reads
«тильда слэш точка дш слэш кордис точка патч точка имл». A flag token
(`--force`) drops its leading dashes before the word lookup; the dashes have
no sound and keeping them would only add separator noise.

### `HEAD` and the all-caps guard

`head` is four letters, below the five-letter word-like threshold, so the
all-caps token would be spelled «эйч и эй ди». `AS_WORD` already holds the
guards that keep an all-caps token from spelling (`json`, `yaml`); `head`
joins it and mirrors the `ENGLISH_WORDS` reading.

## Risks and trade-offs

- Identifier readings change wherever the letter table disagreed with the
  spelling rules. The change is visible in golden fixtures and is the point:
  a word part should sound like the word, not like its letters.
- `ENGLISH_WORDS` grows by hand; the "rules read it correctly" constraint
  keeps it from becoming a full dictionary.
- Path detection is heuristic: a path with a URL scheme or a Windows drive
  letter is still read by the identifier path, not the path normalizer. This
  keeps the change narrow; expanding the detector is a separate change.

## Migration

No data migration. A user-dictionary override still wins: the pre-pass runs
before every built-in table on both routes.
