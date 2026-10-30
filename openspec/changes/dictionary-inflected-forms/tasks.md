## 1. Specification

- [x] 1.1 Proposal, design, delta spec and tasks for
  `dictionary-inflected-forms`.

## 2. Implementation

- [x] 2.1 Add `forms` to `DictionaryEntry` and the lowercased form→primary
  alias index to `UserDictionary`.
- [x] 2.2 Validate Russian-only forms, duplicates and cross-entry collisions on
  save/import; skip invalid entries on load.
- [x] 2.3 Read and write the inline-table TOML shape while keeping string
  entries compatible.
- [x] 2.4 Add the comma-separated forms field, help text, inline errors and the
  entry-list display.

## 3. Tests and fixtures

- [x] 3.1 Unit tests for form validation, alias resolution, collisions and
  store round-trips.
- [x] 3.2 Frontend validation, search and collision tests; editor help/error
  smoke test.
- [x] 3.3 Golden fixture under `native_en/` with a form-list dictionary.
- [x] 3.4 Performance lock keeps the 1 MB/256 KB budgets with an alias entry.

## 4. Documentation and gates

- [x] 4.1 `CHANGELOG.md` entry (user-visible dictionary feature).
- [x] 4.2 `just test`, `just lint`, `openspec validate --all --strict`.
- [ ] 4.3 Archive the change and sync the `user-dictionary` spec.
