# Tasks — restrict the viewer display switch to the entry's source format

## 1. OpenSpec

- [x] 1.1 Proposal, design, and deltas for `text-display`, `storage`,
  `ipc-commands`, `preview-dialog` and `text-import`.

## 2. Backend

- [x] 2.1 `DisplayMode` enum (`rendered` | `plain`) and
  `TextEntry.display_mode: Option<DisplayMode>` with `#[serde(default)]`;
  document `format` as the immutable source format.
- [x] 2.2 Replace `set_entry_format` with `set_entry_display_mode`; register
  the new command; keep the entry's source format and audio artifacts
  untouched.
- [x] 2.3 Tests: serde round-trip for `display_mode` and for an old
  `history.json` without the field; command test proving `format` does not
  change after creation.

## 3. Frontend

- [x] 3.1 `DisplayMode` / `display_mode` in the IPC types and
  `setEntryDisplayMode` in the command wrapper.
- [x] 3.2 `viewerDisplay.ts`: source-format derivation, allowed views and
  effective-mode resolution as pure functions.
- [x] 3.3 `TextViewer`: derive the views from the source format, clamp stale
  values, hide the control for a plain source, render the stored
  `html_source`, persist the choice and restore it across entry switches.
- [x] 3.4 Persist `detectFormat(text)` on the dialog-less Add paths (direct
  Add, plain fallback) at the ingest seam.
- [x] 3.5 RU/EN labels: `Markdown`, `HTML`, «Исходный» / "Raw".

## 4. Tests

- [x] 4.1 Unit tests for the derivation table, including `format: null` and
  stale-`format` rows.
- [x] 4.2 TextViewer tests per source format: markdown, HTML, plain, legacy
  null, stale `html` without markup, entry switching, persisted choice and
  the failed-persist revert.

## 5. Validation

- [x] 5.1 `cargo test --manifest-path src-tauri/Cargo.toml`
- [x] 5.2 `pnpm typecheck`
- [x] 5.3 `pnpm test:unit`
- [x] 5.4 `just lint`
- [x] 5.5 `openspec validate restrict-viewer-display-modes --strict`, then
  archive the change and sync the specs.
- [x] 5.6 CHANGELOG `[Unreleased]` note.
