## 1. Validation and model

- [x] 1.1 Extend `key_script` in `src-tauri/src/dictionary/mod.rs` with the
  internal-hyphen rule (non-empty alnum segments, at least one letter, one
  script, no leading/trailing/doubled hyphen) and update the `InvalidFrom`
  message to name the hyphen.
- [x] 1.2 Add `UserDictionary::has_hyphenated_keys()` for the pipeline gate.
- [x] 1.3 Cover the shape in `src-tauri/src/dictionary/tests.rs`: accepted
  `UTF-8`, `e-mail`, `по-русски`; rejected `-mail`, `mail-`, `e--mail`, `C:`,
  `a.b`, `IPзамок`, and a hyphenated inflected form.

## 2. Pipeline

- [x] 2.1 Add the hyphenated candidate regex and a whole-token pass in
  `src-tauri/src/pipeline/mod.rs`, running between the abbreviation phase and
  the code-identifier phase, gated by `has_hyphenated_keys()`; record a
  `DictionaryOverride` span like the existing pre-pass.
- [x] 2.2 Look up the whole token in `CodeIdentifierNormalizer::normalize_kebab_case`
  so inline code follows the same rule.
- [x] 2.3 Pipeline tests: whole-token reading with built-in readings for the
  parts, precedence over part entries, part entries still applying, case
  insensitivity, no partial match inside `utf-8-sig`, native-identifier route
  keeps the override span and the neighbouring native span.

## 3. Editor and strings

- [x] 3.1 Mirror the hyphen rule in `src/lib/userDictionary.ts`
  (`LATIN_TOKEN_RE`, `CYRILLIC_TOKEN_RE`) and its doc comment.
- [x] 3.2 Update `dictionary.form.error.from.charset` in `src/i18n/ru.ts` and
  `src/i18n/en.ts` to mention the hyphen.
- [x] 3.3 Update `src/lib/userDictionary.test.ts`: acceptance for `UTF-8`,
  rejection for malformed hyphenation, quick-add gate for a hyphenated
  selection.

## 4. Fixtures, specs, changelog

- [x] 4.1 Add the `native_en/user_dict_hyphenated` golden fixture (input, dict
  sidecar, expected text, char map) and review the generated output by hand.
- [x] 4.2 Add the `[Unreleased]` CHANGELOG note.

## 5. Validation

- [x] 5.1 `cargo fmt --manifest-path src-tauri/Cargo.toml`
- [x] 5.2 `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
- [x] 5.3 `cargo test --manifest-path src-tauri/Cargo.toml`
- [x] 5.4 `pnpm typecheck && pnpm test:unit`
- [x] 5.5 `openspec validate add-hyphenated-dictionary-keys --strict`
