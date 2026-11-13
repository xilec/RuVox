## Why

Words read correctly in prose are read letter by letter inside backticks:
`view` sounds «виев», `force` — «форке», `forge` — «форге», while the same
words outside read «вив», «фоурс», «фоурдж». Inline code and identifier parts
own a separate per-letter transliteration table, so they never reach the
curated English readings. A path in backticks makes it worse: character-by-
character transliteration turns `~/.dsh/cordis.patch.yml` into
«~/.дсх/кордис.паткх.ймл» — vowel-less clusters with markup characters that the
engine skips, so the phrase is not narrated at all.

Curated readings also miss everyday words the spelling rules cannot express:
`cleanup` reads «клинуп» instead of «клинап», `HEAD` is spelled
«эйч и эй ди» instead of «хээд», `Office` reads «оффайс» instead of «офис»,
`Session` — «сесжн» instead of «сэшон», `untracked` — «унтрэкед» instead of
«антрэкед», `view` — «вив» instead of «вью».

## What Changes

- Add a curated `ENGLISH_WORDS` table for everyday vocabulary the spelling
  rules get wrong (`cleanup` → «клинап», `untracked` → «антрэкед», `view` →
  «вью», `force` → «форс», `forge` → «фордж», `office` → «офис», `session` →
  «сэшон», `head` → «хээд»), consulted after `IT_TERMS` in prose, identifiers,
  inline code and path words alike.
- Resolve identifier parts and inline-code words through the same chain as
  prose (user dictionary, `CODE_WORDS`, curated English tables, spelling
  rules) instead of the per-letter table; drop punctuation around a word part
  (`forge*` → «фордж»).
- Read a filesystem path inside inline code with the path normalizer, exactly
  as that path is read in prose.
- Read a flag's word without its leading dashes (`--force` → «форс»).
- Curate `head` in `AS_WORD` so the all-caps git word is not spelled.

## Non-goals

- Which engine reads which language does not change, and neither does the
  Native identifier route: on TeraTTS a prose identifier still stays Latin.
- No general English pronunciation overhaul: the spelling rules stay as they
  are and only words that must not follow them are curated.
- No user-dictionary behavior change.

## Capabilities

### Modified Capabilities

- `text-pipeline`: inline-code words and identifier parts read through the
  prose English chain, and a path in inline code reads as a path.

## Impact

- `tts-normalize/src/pipeline/normalizers/english.rs`: `ENGLISH_WORDS` and the
  shared `curated_reading` chain.
- `tts-normalize/src/pipeline/normalizers/code.rs`: word resolution and
  punctuation handling for identifier parts.
- `tts-normalize/src/pipeline/normalizers/abbreviations.rs`: the `head`
  all-caps guard.
- `tts-normalize/src/pipeline/normalizers/urls.rs`: path words use the shared
  chain.
- `tts-normalize/src/pipeline/mod.rs`: inline-code path and flag handling.
- Golden fixtures `kebab_case`, `markdown_code_block_directive_legacy` and
  `user_dict_identifier_part` change their readings.
