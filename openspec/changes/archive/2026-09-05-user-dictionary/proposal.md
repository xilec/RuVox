# Proposal: User Dictionary

## Why

The normalization pipeline ships hard-coded pronunciation tables (`IT_TERMS`,
`CODE_WORDS`, abbreviation maps) that the user cannot extend: fixing a wrong
pronunciation or adding a personal term (a surname, a domain word, an alnum
token like `IPv6`) requires a patch and a rebuild. Worse, alnum tokens such as
`IPv6` or `mp3` in prose are captured by no phase at all today — Latin reaches
the TTS engine raw, the exact problem RuVox exists to solve.

## What Changes

- New user-editable **User Dictionary**: flat mappings `from → to`, persisted
  as a hand-editable TOML file in the config root, loaded at startup and
  refreshed at runtime after every save.
- A dictionary **pre-pass phase** in the prose pipeline (its own regex,
  letters-and-digits tokens, exact-key replacement only) that also fixes the
  unreadable-alnum-token gap for dictionary entries; per-token lookups in
  code identifiers, URL components, and code blocks read aloud.
- User entries **win over every built-in table** at every lookup site;
  matching is case-insensitive; one entry per word (key: lowercased `from`).
- Four new Tauri commands: `get_user_dictionary`, `save_user_dictionary`
  (full atomic replace), `import_user_dictionary(path, mode: merge|replace)`,
  `export_user_dictionary(path)`.
- Settings section "Словарь" opening a dictionary editor modal: searchable
  list with an "overrides built-in" badge, CRUD with validation, TOML
  import/export via file dialog or drag&drop with an action choice
  (merge vs replace), quiet footer status line instead of save toasts.
- "В словарь" quick-add from the preview dialog: text selection in either
  pane prefills `from` in the add-entry form.
- Changing the dictionary does **not** invalidate already-synthesized audio;
  regeneration stays manual.

## Capabilities

### New Capabilities

- `user-dictionary`: entry semantics (single Latin source token with at least
  one letter, case-insensitive matching, user-wins precedence), TOML file
  format and persistence in the config root, corruption recovery, validation
  rules, import/export semantics with conflict resolution.

### Modified Capabilities

- `text-pipeline`: new requirement for the user-dictionary pre-pass phase
  (position in the fixed phase order, own regex incl. alnum tokens) and
  user-dictionary precedence at the identifier, URL, and code-block lookup
  sites.
- `ipc-commands`: new requirement for the four user-dictionary commands,
  their payload shapes, and error behavior consistent with the shared IPC
  types.
- `ui`: new requirement for the Settings dictionary section, the editor modal
  (list, search, badge, CRUD, import/export with mode choice, status line).
- `preview-dialog`: new requirement for the selection-based quick-add action.

## Impact

- **Rust backend**: `src-tauri/src/pipeline/` (new pre-pass phase, lookup
  hooks in `CodeIdentifierNormalizer`, `URLPathNormalizer`, `EnglishNormalizer`
  custom_terms path), new dictionary storage module, `commands/`, `lib.rs`
  (pipeline refresh on save), `paths.rs` (dictionary path in config root);
  new direct dependency: `toml`.
- **Frontend**: `src/dialogs/` (new `DictionaryModal`, Settings section),
  `src/dialogs/PreviewDialog.tsx` (quick-add), `src/lib/tauri.ts` (command
  wrappers).
- **Tests**: golden pipeline fixtures for the pre-pass and each lookup site;
  unit tests for TOML load/save/dedupe/corruption; TS unit tests for editor
  validation; manual pass for UI.
- **No breaking changes**: empty dictionary (the default) leaves pipeline
  output byte-identical; existing specs' requirements stay true.

## Non-goals

- Cyrillic source words (#277), multi-word phrases (#278), hyphens and other
  punctuation in `from` (#279).
- Regex-based rules, team/shared dictionaries via Gist, frequency-based
  suggestions, dictionary versioning beyond the file-format `version` field.
- Auto-invalidation or re-synthesis of cached audio when entries change.
