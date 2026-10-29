# Tasks: Cyrillic dictionary keys

## 1. Validation

- [x] 1.1 Accept a Russian Cyrillic token in `from` (letters and digits, one
  script, at least one letter, 64 characters), keep rejecting mixed-script,
  non-Russian Cyrillic, and digit-only tokens. Verify: unit tests for each
  accept/reject class, including `А4` and `ё`.
- [x] 1.2 Mirror the rule in the editor's `SOURCE_TOKEN_RE` and its error text.
  Verify: TS tests share the Rust cases.

## 2. Pre-pass

- [x] 2.1 Add the Cyrillic candidate pattern and run each pattern only when the
  dictionary has keys of that script. Verify: a Cyrillic entry hits a word in
  prose, a non-hit Cyrillic word is untouched, and the Latin path is unchanged.
- [x] 2.2 Extend the performance lock with a 1 MB run of a dictionary holding
  one Cyrillic entry. Verify: the test stays inside the existing budget.

## 3. Rendering and fixtures

- [x] 3.1 A Cyrillic entry with a mark renders it for `Stress::Auto`/`Manual`
  and strips it for `Stress::None`. Verify: pipeline tests for both branches.
- [x] 3.2 Profile-pinned golden fixtures for the Cyrillic entry, both profiles,
  with regenerated char maps.

## 4. Docs

- [x] 4.1 Spec deltas for `user-dictionary` (charset, exact form) and
  `text-pipeline` (pre-pass scope).
- [x] 4.2 CHANGELOG entry and the README line, plus the editor hint that
  matching is by exact word form.

## 5. Verification

- [x] 5.1 `just test` and `just lint` green; `openspec validate --all --strict`.
- [ ] 5.2 Manual pass: add a Russian word with a mark, preview it, hear the
  stress on the native engine, and check the same word without support.
