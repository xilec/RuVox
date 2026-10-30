## 1. Storage model

- [x] 1.1 Add `DictionaryScope`, `DictionaryCollection`, `DictionaryBook` and `merged_for` to `dictionary/mod.rs`.
- [x] 1.2 Implement version 2 load/save with version 1 migration and atomic writes in `dictionary/store.rs`.
- [x] 1.3 Add store tests for migration, round-trip, empty-engine omission and merge precedence.

## 2. Commands and pipeline refresh

- [x] 2.1 Add scope-aware DTOs and `get_dictionaries`, `save_dictionary`, `import_dictionary`, `export_dictionary`.
- [x] 2.2 Refresh the merged map at startup, after dictionary changes and after an engine switch.
- [x] 2.3 Gate `builtin_contains` by the scope's profile.
- [x] 2.4 Update command registration and command tests.

## 3. Editor

- [x] 3.1 Add the scope selector and collection-aware load/save/import/export to `DictionaryModal`.
- [x] 3.2 Add Russian/English labels and the precedence hint.
- [x] 3.3 Update `src/lib/tauri.ts` and any tests.

## 4. Verification

- [x] 4.1 `just test`, `just lint`, `openspec validate --all --strict`.
- [x] 4.2 Manual pass: switch scopes, add an engine override, verify it does not leak to another engine.
- [x] 4.3 Archive the change and sync the spec.
