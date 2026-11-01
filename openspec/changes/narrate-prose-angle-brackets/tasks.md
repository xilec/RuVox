## 1. Pipeline

- [x] 1.1 Reproduce first: add the golden fixture `teratts/shell_redirect` with
  the incident line (`grep -r mdrfckr /root/.ssh /home/*/.ssh 2>/dev/null
  crontab -l; …`) and confirm the golden run fails on the stray `>` before any
  production change. (Pinned to the `teratts` profile: the line contains digits
  and Latin tokens whose readings differ per profile, so a flat case cannot
  assert one expected string.)
- [x] 1.2 Special-symbols phase: after the Greek/math/arrow loop, read a bare
  `<`/`>` as "меньше"/"больше" with the same space-surrounded `TrackedText`
  substitution the neighbouring rules use; comment why the phase owns it
  (operator phase already consumed the multi-character forms, native code is
  behind placeholders).
- [x] 1.3 Generate the fixture's `expected.txt`/`char_map.json` with
  `RUVOX_FIXTURE_CASE=… cargo test --test golden_profiles -- --ignored
  write_fixture_case` and review both by hand. The legacy directive fixture
  (`silero/markdown_code_block_directive_legacy`) was re-baked for the same
  reason — its HTML comment is intentionally plain text into the symbol phases.

## 2. Tests

- [x] 2.1 Unit test in `pipeline::tests`: drive the public
  `process_for_engine` over a redirect, a comparison, a pasted tag and a
  Markdown autolink under every shipping profile and assert the output has no
  `<`/`>` left. The assertion is the invariant, not one expected string.
- [x] 2.2 Keep an explicit expectation for the incident text ("два больше
  слэш дев слэш налл") so a reading change is visible in review.
- [x] 2.3 Verify the operator precedence scenario: `>=` still reads "больше
  или равно" and never splits into "больше"/"равно".
- [x] 2.4 Confirm `teratts/code_block_native` and the other native-code
  fixtures are unchanged (bracket narration must not touch restored code);
  update `markdown_blockquote_markers_are_not_read` for the narrated inline
  comparison.

## 3. Documentation and validation

- [x] 3.1 Add a CHANGELOG `[Unreleased]` entry (user-visible: `<`/`>` are now
  read instead of failing on TeraTTS / being dropped elsewhere).
- [x] 3.2 `pnpm dlx @fission-ai/openspec@1.6.0 validate
  narrate-prose-angle-brackets --strict`.
- [x] 3.3 `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo test --manifest-path src-tauri/Cargo.toml` (golden corpus included),
  `pnpm typecheck` and `pnpm test:unit` (no frontend change expected, but the
  gate is required).
- [ ] 3.4 Manual pass on the dev app with TeraTTS active: regenerate the entry
  that failed (SSH-honeypot article) and confirm the synthesis completes and
  the redirect is audible.

## 4. Review follow-up (branch review)

- [x] 4.1 Read the brackets through `SymbolNormalizer`'s shared `SYMBOLS` table
  instead of a local literal pair — one home for the wording, output
  byte-identical (no fixture re-bake).
