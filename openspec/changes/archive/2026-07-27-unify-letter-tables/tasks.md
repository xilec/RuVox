# Tasks: unify-letter-tables

## 1. Shared table

- [x] 1.1 Add `pub(crate) static LETTER_NAMES: [(char, &str); 26]` in
  `src-tauri/src/pipeline/normalizers/english.rs` with the canonical
  readings (x → "икс", y → "вай", z → "зет") plus a lookup helper

## 2. Migrate consumers

- [x] 2.1 `abbreviations.rs`: delete `LETTER_MAP`, use the shared table in
  the spelling path
- [x] 2.2 `code.rs`: `letter_name` delegates to the shared table (behavior
  unchanged)

## 3. Tests

- [x] 3.1 Update `abbreviations.rs` expectations: XML → "икс эм эл",
  XSS → "икс эс эс", UX → "ю икс", X/Y/Z → "икс"/"вай"/"зет",
  XYZ → "икс вай зет", WXYZ → "дабл ю икс вай зет"
- [x] 3.2 Add a test pinning table identity: abbreviation spelling and
  `spell_abbreviation` return the same reading for the same letter

## 4. Gates

- [x] 4.1 `cargo fmt`, `cargo clippy --no-deps -- -D warnings`,
  `cargo test --manifest-path src-tauri/Cargo.toml` — all green
- [x] 4.2 `pnpm dlx @fission-ai/openspec validate unify-letter-tables --strict`
