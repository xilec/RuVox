# Tasks

## 1. Rule engine

- [x] 1.1 Replace the Aho-Corasick digraph table with `transliterate_word` and
  its ordered suffix/digraph/context rules.
- [x] 1.2 Cover silent/magic `e`, vowel digraphs, soft `c`/`g`, voiced `s`,
  `-tion`/`-sion`/`-ture`/`-cial`/`-tial`, `th`, `ck`/`qu`/`x`/`-le`, and the
  `-age`/`-ive`/`-ate`/`-ade` endings.
- [x] 1.3 Add rule-class unit tests, including `package` → `пэкэдж` and
  `feature` → `фиче`.
- [x] 1.4 Keep irregulars, all-caps guards, and composition guards in
  `IT_TERMS`; remove entries whose exact reading the rules now produce.

## 2. Fixtures and docs

- [x] 2.1 Regenerate the dictionary baseline and review the changed rows.
- [x] 2.2 Update the curated inflected-form fixture explicitly.
- [x] 2.3 Add a golden pipeline fixture for the rule classes.
- [x] 2.4 Update `docs/contributing.md` and the `text-pipeline` spec.
- [x] 2.5 Add a `CHANGELOG.md` `[Unreleased]` note.

## 3. Validation

- [x] 3.1 `cargo fmt`, `cargo clippy`, `cargo test` (src-tauri), `pnpm
  typecheck`, `pnpm test:unit`, `pnpm lint`, `openspec validate --strict`.
- [ ] 3.2 Manual listening pass over the rule-class sample and the changed
  readings; report the verdict on #306.
- [ ] 3.3 Archive the change after the manual verdict.
