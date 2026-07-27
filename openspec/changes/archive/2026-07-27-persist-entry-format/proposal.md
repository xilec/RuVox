# Proposal: persist-entry-format

## Why

The display format (`plain` / `markdown` / `html`) in `TextViewer` is
ephemeral client-side state: it resets to the default (`markdown`) every
time an entry is selected or the app restarts (`src/components/TextViewer.tsx:26-36`,
TODO(B1/F4)). With HTML support coming (GitHub issue #6), the format also
becomes a property of the entry's content — an entry created from browser
HTML must reopen in HTML mode, not in markdown. Persisting the format per
entry (GitHub issue #5) is the prerequisite.

## What Changes

- Add `format: TextFormat` (`"plain" | "markdown" | "html"`, default
  `"plain"` for legacy entries) to the `TextEntry` storage schema with
  `#[serde(default)]` backwards compatibility.
- Add a Tauri command `set_entry_format(id, format)` that persists the
  format and emits `entry_updated`.
- `TextViewer` reads the format from the selected entry (falling back to
  the `text_format` UI-config default when the entry has none persisted yet)
  and calls `set_entry_format` when the user switches the `SegmentedControl`.
- Switching the format changes the display only; existing audio and
  normalized text are left untouched (no re-synthesis).

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `storage`: History File Schema gains the `format` field with a serde
  default; legacy `history.json` files without it must parse as `"plain"`.
- `ipc-commands`: `TextEntry` IPC type gains `format`; new command
  `set_entry_format` with the standard typed-error and `entry_updated`
  event behavior.
- `text-display`: "Display mode switching" — the selected mode is no
  longer ephemeral; it is persisted per entry and restored on selection.

## Impact

- `src-tauri/src/storage/schema.rs` — `TextFormat` enum + `TextEntry.format`.
- `src-tauri/src/commands/mod.rs` + `src-tauri/src/lib.rs` — new command,
  registered in `generate_handler!`.
- `src/lib/tauri.ts` — `TextEntry.format`, `EntryFormat` type, `setEntryFormat`
  wrapper.
- `src/components/TextViewer.tsx` — format state sourced from the entry;
  SegmentedControl persists on change.
- Tests: schema serde tests (round-trip + legacy default), command
  orchestration test, frontend unit tests where applicable.
- GitHub issue #5 closes; unblocks issue #6.
