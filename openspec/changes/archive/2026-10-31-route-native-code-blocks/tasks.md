## 1. Contract and profiles

- [x] 1.1 Add `code_block: Reading` to `Route`; include it in `any_native()`,
  rename `ProfileError::NativeLatinWithoutLanguage` to
  `NativeWithoutLanguage`, and update the doc comments/invariant text.
- [x] 1.2 `TextProfile::validate`: require a non-Russian language for any
  native route; unit tests for a native code route without a language and for
  native code with narrated prose.
- [x] 1.3 Set `code_block: Native` on `TERATTS_PROFILE`, `PIPER_PROFILE` and
  the test-only `NATIVE_EN_PROFILE`; add `en` to Piper's languages; keep both
  Silero profiles `Narrate`.
- [x] 1.4 Update profile registry tests: per-engine route matrix (TeraTTS and
  Piper native code, Silero narrated), Piper's language list, and the
  validate-every-engine test.

## 2. Pipeline

- [x] 2.1 Code-block phase: in the read mode with a `Narrate` route keep the
  existing narration; with a `Native` route trim the content, collapse
  internal whitespace runs to one space, and replace the block with a unique
  private-use placeholder; return the native blocks to the caller. Brief and
  mermaid behavior unchanged.
- [x] 2.2 `CharMapping`: add a substitution helper that replaces a placeholder
  with the code text, mapping every inserted codepoint to the placeholder's
  whole source range; unit test length and origin invariants.
- [x] 2.3 Wire the pipeline: pass the route into the code-block phase, record
  a `CodeBlock` span with the profile's native language over each placeholder,
  and restore the code after the punctuation post-processing and before
  `build_mapping`.
- [x] 2.4 `SpanKind::CodeBlock` with language, span priority and overlap rules;
  extend span-invariant fixtures if needed.
- [x] 2.5 Pipeline unit tests: native code is not rewritten by the number,
  operator, symbol, English or dictionary phases; brief mode outranks the
  route; multiple blocks restore independently; the code-block span covers the
  output code text and maps into the source block.

## 3. Adapter

- [x] 3.1 TeraTTS adapter: replace `<`/`>` with spaces inside `CodeBlock`
  spans before segmentation (codepoint count unchanged) and log it; unit test
  that `Vec<T>` becomes `Vec T` in the English segment while a stray `<` in
  Russian text still fails validation.

## 4. Tests and fixtures

- [x] 4.1 Golden harness: register `teratts` and `piper` as known profiles for
  pinned folders while keeping the flat corpus on `silero`/`native_en`; update
  the classifier to use the flat-corpus selection.
- [x] 4.2 Move the now profile-specific flat fixtures `markdown_code_block`
  and `user_dict_code_block` to `silero/`.
- [x] 4.3 Add `teratts/code_block_native` and `piper/code_block_native`
  fixtures (read mode; digits and operators in the block prove no narration)
  and review the generated text and char map by hand.
- [x] 4.4 Confirm the brief and mermaid fixtures stay profile-invariant and the
  full-corpus run is green.

## 5. Documentation and validation

- [x] 5.1 Update README code-block and TeraTTS wording, regenerate
  `README.en.md`, and add a CHANGELOG `[Unreleased]` entry.
- [x] 5.2 `openspec validate route-native-code-blocks --strict`.
- [x] 5.3 `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and
  `cargo test` for `tts-contract` and `src-tauri`; `pnpm typecheck` and
  `pnpm test:unit` (no frontend change expected, but the gate is required).
- [x] 5.4 Automated end-to-end smoke on TeraTTSv2: the `teratts_mixed_app`
  probe fed a read-mode fenced block containing `<`, synthesized it without an
  error, and word timestamps stayed mapped.
- [ ] 5.5 Manual listening pass with TeraTTSv2 and Piper installed: set
  «Читать полностью», listen to an article with a fenced block (code read by
  the engine, Russian prose unchanged, no digits spelled out inside code),
  verify «Кратко» still announces, and that the highlight stays continuous.
