# Proposal: add-regeneration-preview

## Why

«Перегенерировать аудио» re-synthesizes immediately: the old audio is deleted
before synthesis starts and the user never sees what the text will be
normalized into this time. The preview dialog — the app's single place to
inspect normalization before synthesis — appears only when text is first
added (#253).

## What Changes

- The «Перегенерировать аудио» queue context-menu action opens the preview
  dialog pre-filled with the entry's `original_text` and its live
  normalization before anything is deleted.
- Confirming the dialog runs the regeneration (delete old audio → re-run
  synthesis); cancelling it (Cancel button, ESC, close icon) leaves the entry
  and its audio completely untouched.
- In regeneration mode the dialog hides the controls that do not apply:
  «Редактировать» (entry text is immutable), the source-format selector
  (regeneration normalizes the stored text directly), and «Больше не
  показывать этот диалог» (that gate belongs to the Add flow). «Read Now»
  stays and is honored.
- `regenerate_entry` gains a `play_when_ready` parameter: on success the
  regenerated audio autoplays when the switch is on (same rule as the initial
  synthesis); it previously hard-coded "do not play".

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `preview-dialog` — new requirement: the regeneration flow opens the same
  preview dialog before synthesis, with regeneration-specific control
  visibility and cancel-keeps-audio semantics.
- `ipc-commands` — «Entry Regeneration Command» gains the `play_when_ready`
  parameter and the autoplay-on-success rule.

## Impact

- `src/dialogs/PreviewDialog.tsx` — new `mode` prop (`add` | `regenerate`);
  regeneration mode forces the plain preview path and hides inapplicable
  controls.
- `src/components/QueueList.tsx` — the context-menu action delegates to a new
  `onRegenerate` prop instead of invoking the backend directly.
- `src/components/AppShell.tsx` — owns the regeneration preview state and the
  confirm handler (invoke + notifications move here from `QueueList`).
- `src-tauri/src/commands/mod.rs` — `regenerate_entry(id, play_when_ready)`.
- `src/lib/tauri.ts` — `regenerateEntry(id, playWhenReady)` wrapper.
- i18n catalogs (`src/i18n/ru.ts`, `src/i18n/en.ts`) — confirm-button label
  for regeneration mode.
- Tests: `PreviewDialog.test.tsx`, `QueueList.test.tsx`, Rust
  `orchestration_tests.rs` regenerate cases.
- `CHANGELOG.md` — user-visible behavior note under `[Unreleased]`.
