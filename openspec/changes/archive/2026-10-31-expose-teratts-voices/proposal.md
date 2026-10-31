## Why

The TeraTTSv2 bundle ships ten voice styles — four Russian and six English —
and the engine treats every voice as cross-lingual, yet Settings hardcodes the
four Russian ids in `TERATTS_VOICES`. The other six are unusable from the UI
even though the engine, the bundle and the `teratts-engine` spec already
support them.

## What Changes

- Add a `get_teratts_voices()` command that reads the installed bundle manifest
  and returns its voice ids in manifest order (`[]` when the bundle is missing
  or unreadable).
- Settings fetches that catalogue on open and after a bundle install, and the
  TeraTTS voice selector lists every returned id instead of the hardcoded four;
  `ru_m5` remains the default.
- Keep the currently saved id selectable even if the manifest does not list it
  (stale config, bundle replaced), so opening Settings never silently drops the
  value the user is about to save.
- Reword the selector description: the voices are not only Russian.
- Sync the `ui` and `storage` specs, which still enumerate four voices, and add
  the new command to `ipc-commands`.
- Cover the catalogue on both sides: a Rust test that the command reports
  English manifest voices, and a Settings test that every reported voice is
  listed and the saved one is preselected.

## Capabilities

### New Capabilities

<!-- none: the engine-side catalogue already exists -->

### Modified Capabilities

- `ui`: the Settings TeraTTS voice selector lists the voices reported by the
  backend rather than a hardcoded four-entry catalogue.
- `storage`: `teratts_voice` accepts any id shipped by the installed bundle
  manifest; the four Russian ids stop being the documented domain.
- `ipc-commands`: new `get_teratts_voices` command exposing the bundle
  catalogue.

## Non-goals

- Per-language voice routing: a segment's language still selects the text
  frontend, never the voice (see `multi-language-synthesis`).
- Validating `teratts_voice` on save, coercing an unknown id, or warning about
  one; synthesis keeps failing with `voice_unknown`.
- Friendlier labels or grouping (Russian/English) in the selector; ids remain
  the labels.
- Listing voices when the bundle is absent or corrupt — the engine is
  unavailable in that state and the selector is not shown.

## Impact

- `src-tauri/src/tts/teratts/mod.rs` — manifest-to-ids helper with tests.
- `src-tauri/src/commands/mod.rs`, `src-tauri/src/lib.rs` — command and
  registration.
- `src/lib/tauri.ts`, `src/dialogs/Settings.tsx`, `src/dialogs/Settings.test.tsx`
  — binding, selector, drift test.
- `src/i18n/ru.ts`, `src/i18n/en.ts` — selector description.
- `openspec/specs/ui`, `openspec/specs/storage`, `openspec/specs/ipc-commands`
  — synced through this change's deltas.
- `CHANGELOG.md` — `[Unreleased]` note.
