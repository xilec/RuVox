# Design: persist-entry-format

## Context

`TextViewer` holds the display format in a local `useState<Format>("markdown")`
(`src/components/TextViewer.tsx:36`), so the choice resets on every entry
selection and app restart. Issue #5 asks to persist it per entry; issue #6
(HTML pipeline) will additionally set the format at ingestion time.

Storage conventions (`src-tauri/src/storage/schema.rs`): optional
`TextEntry` fields use `#[serde(default)]`; enums serialize lowercase like
`EntryStatus`; there are no schema migrations — backwards compatibility is
achieved via serde defaults only. Mutations follow the pattern
`require_entry → mutate → update_entry → emit_entry_updated`
(e.g. `regenerate_entry`).

The viewer's current default (`markdown`) is ALSO the desired display for
the entire existing history — those entries hold technical markdown-ish
clipboard text that users have always seen rendered as markdown.

## Goals / Non-Goals

**Goals:**

- `TextEntry.format` persisted in `history.json`, restored across launches.
- Selecting an entry shows its saved format in the `SegmentedControl`.
- Legacy `history.json` (no `format` key) parses and behaves exactly as
  today (markdown rendering).
- New command `set_entry_format` following existing command conventions.

**Non-Goals:**

- Format detection at ingestion time (belongs to issue #6 change).
- Re-synthesis when the format is toggled (display-only switch).
- Markdown auto-detection, per-entry `html_source` storage (issue #6).
- Removing the `text_format` UI-config default — it stays the fallback for
  entries without a persisted choice.

## Decisions

### D1: `format: Option<TextFormat>` instead of a serde-defaulted enum

Issue #5 proposes `format: TextFormat` defaulting to `Plain`. That would
make every legacy entry deserialize as `plain` and silently switch the
viewer from markdown to plain rendering for the whole existing history —
a visible regression on upgrade.

Instead: `pub format: Option<TextFormat>` with `#[serde(default)]` → `None`.
`None` means "the user never chose" → the viewer falls back to the
`text_format` UI-config default (today: markdown). New behavior only
appears once the user toggles (or issue #6 sets `Html` at ingestion).

- `TextFormat { Plain, Markdown, Html }` — derives like `EntryStatus`,
  `#[serde(rename_all = "lowercase")]`.
- TS: `format: EntryFormat | null` on `TextEntry`,
  `type EntryFormat = "plain" | "markdown" | "html"`.

Alternatives considered: plain enum with `#[serde(default)]` (rejected —
changes legacy rendering); tri-state enum with an `Unset` variant
(rejected — serializes a meaningless value to disk, `Option` is the
idiomatic absence).

### D2: Display-only toggle, no re-synthesis

`set_entry_format` mutates only the format field; `normalized_text`, audio
and timestamps are untouched. A format switch may therefore make the view
inconsistent with already-synthesized audio (e.g. toggling a plain entry
to `html` does not re-extract text). Accepted trade-off for v1: the
primary flow where format matters for synthesis is ingestion (issue #6),
not retroactive toggling.

Alternative considered: trigger `regenerate_entry`-style re-synthesis on
toggle — rejected as surprising (destroys audio), slow, and mostly useless
until #6 lands.

### D3: Command shape follows the existing mutation pattern

```rust
#[tauri::command]
pub async fn set_entry_format(
    state: State<'_, AppState>, id: String, format: TextFormat,
) -> CmdResult<()> {
    let entry_id = parse_entry_id(&id)?;
    let mut entry = require_entry(&state, &entry_id)?;   // storage + not-found error
    entry.format = Some(format);
    state.storage.update_entry(entry.clone())?;
    emit_entry_updated(&state.app_handle, &entry);
    Ok(())
}
```

No `Processing` guard: the change is display-only and cannot race the
synthesis pipeline. Registered in `generate_handler!` (`lib.rs`) so the
mock test harness picks it up. Frontend wrapper per convention:
`setEntryFormat: (id, format) => tauriInvoke('set_entry_format', { id, format })`.

### D4: TextViewer state sourced from the entry

- Effective format = `entry.format ?? "markdown"` (the viewer default; the
  `text_format` config key is not exposed in the UI, so a constant fallback
  preserves today's behavior exactly).
- `useEffect` on `entry?.id` resets local state from the entry (selection
  change restores the saved mode).
- `SegmentedControl.onChange`: update local state immediately, fire
  `commands.setEntryFormat(...)`; on rejection show a notification and
  revert. The `entry_updated` event re-syncs via the existing
  `QueueList` → `useSelectedEntry` path, so no store changes are needed.

## Risks / Trade-offs

- [Viewer default depends on `UIConfig.text_format` being loadable in
  `TextViewer`] → the config is already fetched for the settings UI; reuse
  the same query/store. If wiring proves noisy, constant `"markdown"` is
  the honest fallback (it is the spec'd default).
- [Older app versions reading a newer `history.json` with `format`] →
  serde ignores unknown fields by default; no action needed.
- [User toggles format on an entry with audio and gets view/audio
  mismatch] → documented accepted trade-off (D2); revisit if it confuses
  users in practice.

## Migration Plan

None — `#[serde(default)]` covers legacy files; no version bump, no data
rewrite.

## Open Questions

(none)
