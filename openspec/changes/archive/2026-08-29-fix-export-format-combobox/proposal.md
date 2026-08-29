# Proposal: fix-export-format-combobox

## Why

Follow-up on the #252 manual pass: a single save dialog cannot expose the
chosen file-type filter to the backend (rfd flattens the portal response to
a path, and the portal response has no filter field), so the two-iteration
history of "switch the filter and hope" left WAV effectively unreachable
without typing the extension by hand.

## What Changes

1. **The format choice moves into the dialog itself**: the Linux export
   dialog is opened via the xdg-desktop-portal directly (ashpd, already in
   the tree through rfd) with a «Формат» choice combo — `WAV` by default,
   `Ogg Opus` as the alternative. The portal response reports the combo's
   selected value, so the backend knows the format without guessing.
2. **The returned path is normalized to the chosen format**: matching
   extension kept as typed (any case); mismatched/foreign replaced; missing
   appended. No usable choice in the response → stored format's extension
   as the fallback.
3. **One menu item stays** («Сохранить аудио как…»); no file-type filters
   in the dialog (the combo replaces them). No new error codes: portal
   failures map to the new `export.dialog_failed` wire code, cancellation
   stays a silent no-op.
4. Windows keeps rfd: the native dialog rewrites the typed extension to the
   selected filter's on save, so the extension carries the decision; the
   recognized-extension fallback applies.

## Capabilities

### New Capabilities

<!-- none -->

### Modified Capabilities

- `ipc-commands`: the "Audio Export Commands" requirement — portal dialog
  with the format combo, requested-format normalization, `export.dialog_failed`
  code.

## Impact

- `src-tauri/src/commands/export.rs` — direct ashpd dialog call (Linux),
  normalization with the reported choice, tests; `src-tauri/Cargo.toml` —
  linux-gated `ashpd` dependency.
- Specs: the ipc-commands export requirement is rewritten for the combo
  flow. The ui queue-menu text (synced separately) is unaffected — the
  choice lives in the dialog.
