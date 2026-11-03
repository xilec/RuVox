# Tasks — add-user-guide

## 1. Discovery

- [x] 1.1 Walk the README feature list and the relevant specs
  (`ui`, `queue-lifecycle`, `playback`, `text-display`, `text-import`,
  `model-bundles`, `viewer-copy-actions`, `tray`, `auto-update`, plus
  `storage`, `logging`, `preview-dialog`, `user-dictionary`,
  `html-ingestion`, `word-highlight`).
- [x] 1.2 Record the include/skip table with a reason per feature in
  `design.md`.

## 2. Remove the unused `player_hotkeys`

- [x] 2.1 Drop the field, `default_player_hotkeys`, the `Default` entry and the
  patch field from `src-tauri/src/storage/schema.rs`.
- [x] 2.2 Drop the patch branch from `src-tauri/src/commands/mod.rs`.
- [x] 2.3 Drop the patch case and the literals from
  `src-tauri/src/commands/tests.rs`.
- [x] 2.4 Drop the field from `Config` in `src/lib/tauri.ts` and from the four
  UI test config mocks.
- [x] 2.5 Add a Rust test that loads a config carrying the legacy
  `player_hotkeys` key, asserts the rest is intact, and asserts the key is gone
  from the serialized config.
- [x] 2.6 Confirm `UIConfig` is not `deny_unknown_fields`.

## 3. Guide

- [x] 3.1 Write `docs/user-guide.md` in Russian with the ten sections from the
  issue, one short section per topic.
- [x] 3.2 Add ASCII mockups for the window map, first-run prompt, add menu,
  preview, queue, queue context menu, player, settings, dictionary and tray;
  keep them ~80 columns and aligned.
- [x] 3.3 Cover the shipped features required by the issue: title/note and
  metadata (#353), reading width/scale (#354/#357), viewer «В словарь» (#348),
  dictionary playback (#349), first-run TeraTTSv2 (#359).
- [x] 3.4 Add the hotkey table with only implemented keys.
- [x] 3.5 Add the troubleshooting section (bundle, sound, encoding,
  pronunciation, synthesis failure, long synthesis).
- [x] 3.6 Link the guide from `README.md` (one click) and mirror the link in
  `README.en.md` and `docs/index.md`.

## 4. Process rule

- [x] 4.1 Add the hard rule to `ai/rules/conventions.md`.
- [x] 4.2 Add the guide and rule pointers to the `AGENTS.md` documentation
  table.

## 5. OpenSpec

- [x] 5.1 Add the `ipc-commands` MODIFIED delta: `UIConfig` without
  `player_hotkeys`, plus the legacy-key scenario.
- [x] 5.2 `openspec validate add-user-guide --strict`.
- [x] 5.3 Archive the change and run `openspec validate --specs --strict`
  (27/27).
- [x] 5.4 Diff the synced requirement against the delta: only the removed
  field, the added `language` field and the new scenario change.

## 6. Changelog

- [x] 6.1 `CHANGELOG.md` `[Unreleased]`: Added — user guide; Removed — the
  unused hotkey setting.

## 7. Validation

- [x] 7.1 `cargo test --manifest-path src-tauri/Cargo.toml`.
- [x] 7.2 `pnpm typecheck`.
- [x] 7.3 `pnpm test:unit`.
- [x] 7.4 `just lint`.
- [x] 7.5 `openspec validate --specs --strict`.
- [x] 7.6 Manual checklist prepared for the PR (guide against the real UI:
  first launch, settings sections, dictionary, viewer, hotkeys).
