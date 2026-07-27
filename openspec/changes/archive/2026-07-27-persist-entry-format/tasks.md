# Tasks: persist-entry-format

## 1. Storage schema

- [x] 1.1 Add `TextFormat { Plain, Markdown, Html }` enum to `src-tauri/src/storage/schema.rs` (derives like `EntryStatus`, `#[serde(rename_all = "lowercase")]`)
- [x] 1.2 Add `#[serde(default)] pub format: Option<TextFormat>` to `TextEntry`
- [x] 1.3 Schema tests: round-trip with `format` set; legacy entry JSON without `format` parses as `None` (follow `entry_missing_optional_fields` template)

## 2. Backend command

- [x] 2.1 Implement `set_entry_format(id, format)` in `src-tauri/src/commands/mod.rs` (require_entry → mutate → update_entry → emit_entry_updated; no Processing guard)
- [x] 2.2 Register the command in `tauri::generate_handler!` in `src-tauri/src/lib.rs`
- [x] 2.3 Orchestration tests in `commands/orchestration_tests.rs`: persist + `entry_updated` emitted; audio/normalized fields untouched; unknown id → typed error, no event

## 3. Frontend wiring

- [x] 3.1 `src/lib/tauri.ts`: `EntryFormat` type, `format: EntryFormat | null` on `TextEntry`, `setEntryFormat` wrapper
- [x] 3.2 `src/components/TextViewer.tsx`: effective format = `entry.format ?? "markdown"` (viewer default; `text_format` config key is not UI-editable); reset local state on `entry?.id` change; SegmentedControl persists via `setEntryFormat` (optimistic local update, notification + revert on rejection)
- [x] 3.3 Remove the resolved `TODO(B1/F4)` comment
- [x] 3.4 Fallback is an inline `?? DEFAULT_FORMAT` one-liner — not extracted, no dedicated unit test (no component-test harness exists)

## 4. Gates

- [x] 4.1 `nix develop -c just test` green
- [x] 4.2 `nix develop -c just lint` green
- [ ] 4.3 Manual check: toggle format, restart app, verify the mode is restored; verify legacy `history.json` loads and renders in the default mode (deferred to the pre-PR manual pass together with the html-view-support checklist)
