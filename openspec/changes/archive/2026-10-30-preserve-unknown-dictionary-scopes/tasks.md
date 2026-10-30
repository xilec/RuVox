## 1. Storage

- [x] 1.1 Keep unknown-scope collections as raw tables in `DictionaryBook`.
- [x] 1.2 Write them back from `serialize_book`.
- [x] 1.3 Add a load/save round-trip test.

## 2. Verification

- [x] 2.1 `cargo test --lib dictionary::`, `openspec validate --all --strict`.
- [x] 2.2 Archive the change and sync the spec.
