# Tasks — align dictionary token contract

## 1. Spec

- [x] 1.1 Add the `user-dictionary` MODIFIED delta: point "Viewer quick-add"
  at "Dictionary entry semantics", state the global prefill scope, reference
  the menu placement in `viewer-copy-actions`, and add the `UTF-8` and
  `IPзамок` scenarios.
- [x] 1.2 Add the `preview-dialog` MODIFIED delta: replace the restated
  "punctuation" predicate with a reference to "Dictionary entry semantics".
- [x] 1.3 Confirm the implemented predicate (`isSingleSourceToken` and the
  Rust `tts-normalize` validator) already matches the gloss; no code change.

## 2. Validation

- [x] 2.1 `openspec validate align-dictionary-token-contract --strict`.
- [x] 2.2 Archive the change, then `openspec validate --specs --strict`
  (27/27).
- [x] 2.3 Commit the archive with the synced specs on the
  `feat/add-to-dictionary` branch.
