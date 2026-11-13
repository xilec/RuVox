## 1. Curated everyday words

- [x] Add the `ENGLISH_WORDS` table (`cleanup`, `head`, `office`, `session`,
      `untracked`, `view`, `force`, `forge`) and consult it in
      `EnglishNormalizer::normalize` after `IT_TERMS`.
- [x] Expose `curated_reading` as the shared lookup for `IT_TERMS`,
      `ENGLISH_WORDS` and `FUNCTION_WORDS`.
- [x] Curate `head` in `AS_WORD` so the all-caps git word is not spelled.
- [x] Cover the table, the all-caps guard and the duplicate rule with unit
      tests.

## 2. One reading for code and prose

- [x] Resolve identifier parts through the user dictionary, `CODE_WORDS`, the
      curated chain and the spelling rules; use the letter table only for
      non-alphabetic parts.
- [x] Drop punctuation around an alphabetic part before resolution
      (`forge*` → «фордж»).
- [x] Resolve path words through the curated chain in `URLPathNormalizer`.
- [x] Add unit tests for identifier words (`view`, `cleanup`, `HEAD`,
      `force`, `forge`).

## 3. Inline code

- [x] Read a filesystem path inside inline code with `URLPathNormalizer`.
- [x] Drop a flag's leading dashes before the word lookup (`--force` →
      «форс»).
- [x] Add pipeline tests: the six words, backticked words, a backticked path,
      and quotation marks that do not change a reading.

## 4. Verification

- [x] `cargo test -p tts-normalize`
- [x] `cargo test -p ruvox-tauri --test golden` (fixtures `kebab_case`,
      `markdown_code_block_directive_legacy` and `user_dict_identifier_part`
      regenerated)
- [ ] `cargo test --workspace`
- [ ] `just lint`

## 5. Manual pass

- [ ] Narrate the last history entry and confirm `--force` reads «форс»,
      `forge` reads «фордж», `~/.dsh/cordis.patch.yml` is narrated (not
      skipped), and the six words read as listed in the proposal.
