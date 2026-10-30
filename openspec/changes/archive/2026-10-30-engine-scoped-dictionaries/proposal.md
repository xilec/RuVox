## Why

One global user dictionary serves every engine, and the built-in override
markers do not follow the active profile. An override that is useful for one
engine can be wrong for another, and switching engines currently means
rewriting the whole map.

## What Changes

- Persist named dictionary collections, each bound to `global` or one engine;
  an entry belongs to exactly one collection.
- Merge the global collection with the active engine's collection at
  normalization time, with engine-bound entries winning on the same `from`.
- Keep the user dictionary authoritative over built-in tables for every
  profile; gate the built-in override markers by the profile that actually
  rewrites the fragment.
- Add a scope selector to the dictionary editor, import/export the selected
  collection, and migrate today's single dictionary losslessly into the global
  collection.
- Bump the dictionary file format to version 2; version 1 files still load as
  the global collection.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `user-dictionary`: storage becomes a set of named, scoped collections;
  lookup merges global and the active engine; the editor exposes the scope.

## Impact

- Affected code: `src-tauri/src/dictionary/{mod,store}.rs`,
  `src-tauri/src/commands/dictionary.rs`, `src-tauri/src/commands/mod.rs`,
  `src-tauri/src/lib.rs`, `src/dialogs/DictionaryModal.tsx`,
  `src/lib/tauri.ts`, i18n catalogs.
- Storage: `user_dictionary.toml` version 2 with `[[dictionaries]]`; v1 is
  read as the global collection.
- User-visible: a dictionary scope selector; engine-specific overrides no
  longer leak into other engines.
