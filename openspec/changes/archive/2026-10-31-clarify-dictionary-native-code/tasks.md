## 1. Spec

- [x] 1.1 Add the `user-dictionary` delta: scope the native-fragment
  dictionary guarantee to prose runs and code identifiers, state the native
  code-block exception, and add its scenario.
- [x] 1.2 Confirm the existing `dictionary_does_not_rewrite_native_code`
  pipeline test covers the clarified behavior; no code change is needed.

## 2. Validation

- [x] 2.1 `openspec validate clarify-dictionary-native-code --strict`.
- [x] 2.2 Re-run `cargo test --manifest-path src-tauri/Cargo.toml --lib
  pipeline::tests` (the dictionary/native tests) — behavior unchanged.
- [x] 2.3 Archive the change and commit it with the #325 branch.
