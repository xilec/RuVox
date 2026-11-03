## Why

The viewer's `SegmentedControl` offers `Plain | Markdown | HTML` for every
entry, although only one rendered view is meaningful — the one matching the
entry's source format. For a markdown entry, HTML mode pushes raw markdown
through the HTML path; for a plain entry, both rendered modes misinterpret the
text. The switch should be reduced to the rendered view of the source format
and a verbatim view, and hidden entirely when the source is plain.

The root cause is that `TextEntry.format` carries two meanings: the source
format written at ingestion and the viewer's display choice overwritten by
`set_entry_format` on every switch. Splitting them makes the source format
immutable and the display choice independent of it.

## What Changes

- `TextFormat` becomes the immutable source format: written once at ingestion,
  never touched by the viewer. Ingestion paths that never asked the user (the
  direct Add path with the preview dialog disabled, the plain-flavour
  fallback) persist `detectFormat(text)` instead of `null`.
- A new persisted field `TextEntry.display_mode: Option<DisplayMode>`
  (`rendered` | `plain`, serde default) records the viewer's display choice,
  and `set_entry_format` is replaced by `set_entry_display_mode`.
- The viewer derives the allowed views from the entry's source format:
  - `markdown` → `Markdown` (default) and «Исходный»;
  - `html` → `HTML` (default) and «Исходный»;
  - `plain` → «Исходный» only, no `SegmentedControl`;
  - `format: null` / legacy → derive the source: `html_source.is_some()` →
    `html`, otherwise `detectFormat(original_text)`. A stale `format` that
    contradicts the derived source (the old toggle wrote the display choice
    into `format`) is clamped at render time — no data migration.
- Labels move to the translation catalog: the rendered option keeps the
  format name (`Markdown` / `HTML`), the verbatim view reads «Исходный»
  (EN: "Raw").
- `display_mode` is display-only: it persists per entry across restarts and
  entry switches, and never touches text, audio or timestamps.
- Tests: Rust serde round-trip plus loading an old `history.json` without the
  field; TS unit tests for the derivation table including legacy rows;
  TextViewer tests per source format; a test that `format` never changes
  after creation.
- OpenSpec deltas for `text-display`, `storage`, `ipc-commands`,
  `preview-dialog` and `text-import`; CHANGELOG `[Unreleased]` note.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `text-display`: "Display mode switching" now derives the allowed views from
  the immutable source format, clamps stale values, hides the control for a
  plain source and persists the choice as `display_mode`; the rendered HTML
  view is backed by the stored `html_source`.
- `storage`: "History File Schema" gains `DisplayMode`, the
  `TextEntry.display_mode` field with a serde default, the immutability of
  `format`, and the legacy-history round-trip.
- `ipc-commands`: "Shared IPC Types" gains `DisplayMode` / `display_mode`;
  "Text Ingestion Commands" states that `format` is the immutable source
  format; "Entry Format Command" is replaced by "Entry Display Mode Command".
- `preview-dialog`: "Add flow gating" persists the detected source format on
  the dialog-less paths, and "Source format selection" records the effective
  format as the immutable source format rather than a display mode.
- `text-import`: "Import format routing" persists the routed format as the
  entry's immutable source format.

## Impact

- Affected code: `src-tauri/src/storage/schema.rs`,
  `src-tauri/src/storage/service.rs`, `src-tauri/src/commands/mod.rs`,
  `src-tauri/src/lib.rs`, `src-tauri/src/commands/orchestration_tests.rs`,
  `src/lib/tauri.ts`, `src/lib/viewerDisplay.ts` (new),
  `src/components/TextViewer.tsx`, `src/components/AppShell.tsx`,
  `src/lib/addFlow.ts`, `src/i18n/ru.ts`, `src/i18n/en.ts`, tests,
  `openspec/`, `CHANGELOG.md`.
- Persisted history gains one optional field; older `history.json` files keep
  parsing, and no migration is required.
- User-visible: a markdown entry shows `Markdown | Исходный`, an HTML entry
  `HTML | Исходный`, a plain entry has no switch; the choice survives
  restarts.

## Non-goals

- Converting markdown to HTML or back, or storing a second rendered
  representation for markdown/plain entries.
- A source-format editor in the viewer: `format` is read-only after
  ingestion.
- Changing the preview dialog's source-format selector or its formatting.
- Rendering/styling work beyond the two labels.
- Migrating existing history data: stale values are clamped at render time.
- Porting content detection to Rust: the tray clipboard path keeps
  `format: null`, and the viewer derives its source at render time.
