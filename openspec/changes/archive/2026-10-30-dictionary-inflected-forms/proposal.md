## Why

The user dictionary matches one exact Russian word form. A user who wants the
same pronunciation for "замок", "замка" and "замке" has to create three entries,
and a missed form silently keeps the engine's default reading. Russian text is
inflected, so the exact-key model is the wrong default for the feature's main
use case: fixing the pronunciation of Russian words.

## What Changes

- Add an optional `forms` list to a dictionary entry: additional Russian source
  forms that are matched exactly and receive the same `to` as the primary word.
- Keep the primary `from` and every listed form case-insensitive and exact;
  an unlisted inflection is still left alone. No stemming or morphology runs in
  the hot path.
- Restrict forms to Russian entries (Cyrillic source and forms); Latin entries
  keep the current exact-token behavior.
- Reject a form that repeats the primary, a sibling form, or a word/form owned by
  another entry, so two entries can never compete for one source word.
- Persist forms in TOML as a table value with `to` and `forms`
  (`"замок" = { to = "зам+ок", forms = ["замка", "замке"] }` is accepted; the
  exporter may write the equivalent nested table); the historical string value
  stays valid, so existing files load unchanged.
- Add a comma-separated "Формы слова" field to the dictionary editor, with
  help text and inline validation, and show listed forms in the entry list.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `user-dictionary`: entry semantics gain optional inflected forms; the storage
  and import/export requirements gain the form-list representation.

## Impact

- Affected code: `src-tauri/src/dictionary/{mod,store}.rs`,
  `src-tauri/src/commands/dictionary.rs`, `src/dialogs/DictionaryModal.tsx`,
  `src/lib/userDictionary.ts`, `src/lib/tauri.ts`, i18n catalogs.
- Affected fixtures: `src-tauri/tests/fixtures/pipeline/native_en/` (a form-list
  dictionary case) and the dictionary unit tests.
- User-visible: one entry can cover the listed inflections of a Russian word;
  existing dictionaries and hand-edited string entries keep working.
