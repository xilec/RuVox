# Tasks — editable title and note header

## 1. OpenSpec

- [x] 1.1 Proposal, design, and deltas for `storage`, `ipc-commands`,
  `queue-lifecycle`, `text-display` and `ui` (there is no separate
  `localization` capability; localization is a `ui` requirement).

## 2. Storage

- [x] 2.1 `TextEntry.title: Option<String>` and `TextEntry.note: Option<String>`,
  both `#[serde(default)]`; `history.json` written by earlier builds parses
  unchanged.
- [x] 2.2 `src-tauri/src/storage/title.rs`: `derive_title` (first usable line,
  Markdown heading/list markers stripped, whitespace collapsed, trimmed,
  200-character cap, `None` when nothing usable) and its case-table tests.
- [x] 2.3 Seed `title` in `add_entry_with_source` — the single storage path used
  by both the UI commands and the tray; `note` starts as `None`.
- [x] 2.4 Rust tests: schema round-trip, legacy default, seeding cases, seeded
  title survives a reload.

## 3. Commands

- [x] 3.1 `set_entry_title(id, title)` and `set_entry_note(id, note)` in
  `commands/mod.rs`: trim, empty → `None`, cap 200 / 2000, atomic
  `update_entry_if` write, `entry_updated` emission, typed `not_found` for an
  unknown id.
- [x] 3.2 Register both in `lib.rs`; add `commands.setEntryTitle` /
  `commands.setEntryNote` wrappers and the `title` / `note` fields to
  `TextEntry` in `src/lib/tauri.ts`.
- [x] 3.3 Rust orchestration tests: trim and persist, clear to `None`, 200/2000
  cap in characters, unknown id, note does not touch `normalized_text`.

## 4. Shared display helpers

- [x] 4.1 `src/lib/entryTitle.ts`: `deriveTitle`, `entryDisplayTitle`,
  `TITLE_MAX_CHARS`, with the case table mirrored from the Rust tests.
- [x] 4.2 `src/lib/generationDisplay.ts`: `displayEngine`, `displayVoice`
  (engine-specific voice names, stored-id fallback), `ABSENT`.
- [x] 4.3 `src/lib/datetime.ts`: naive-UTC parsing and localized date-time
  formatting; `GenerationParamsDialog` uses both shared modules.
- [x] 4.4 `src/lib/sileroVoices.ts` shared by Settings and the display labels;
  `src/lib/entryStatus.ts` shared by the queue card and the header.

## 5. Viewer header

- [x] 5.1 `EntryHeader` + `EntryHeader.module.css` above the scroll area:
  click-to-edit title (Enter/blur save, Esc cancel, empty → derived fallback),
  note with two-line clamp, expand, empty state and autosize textarea
  (Ctrl+Enter/Save, Esc/Cancel), meta row (date, status, engine, voice; no
  snapshot → «не озвучено» / «—»), controls slot at the right of the meta row.
- [x] 5.2 Optimistic edits with rollback and a red toast; incoming
  `entry_updated` does not reset an open draft.
- [x] 5.3 `TextViewer` renders the header above the scroll area and passes the
  format switch plus the reading width/scale controls into the meta row.

## 6. Queue

- [x] 6.1 Queue cards show `entryDisplayTitle` (stored or derived), one line
  with ellipsis and the full title in the tooltip.
- [x] 6.2 Queue search matches title, note and `original_text`,
  case-insensitively.

## 7. i18n

- [x] 7.1 RU/EN catalog entries for the header edit affordances, note states
  and accessible names.

## 8. Tests

- [x] 8.1 Rust: storage round-trip, seeding, command trim/limits/persist,
  unknown id, note not in normalization.
- [x] 8.2 TS: derivation parity table, `entryDisplayTitle` fallbacks, shared
  engine/voice labels.
- [x] 8.3 TS: `EntryHeader` edit flows (Enter/blur/Esc, clear-to-null, note
  empty state, Ctrl+Enter, Escape), rollback + toast, drafts survive
  `entry_updated`, expand/collapse, meta placeholders.
- [x] 8.4 TS: queue title rendering, tooltip and search by title/note/text;
  viewer header placement and reading-control availability.

## 9. Validation

- [x] 9.1 `cargo test --manifest-path src-tauri/Cargo.toml`
- [x] 9.2 `pnpm typecheck`
- [x] 9.3 `pnpm test:unit`
- [x] 9.4 `just lint`
- [x] 9.5 `openspec validate add-entry-metadata --strict`, then archive the
  change and sync the specs; `openspec validate --specs --strict`.
- [x] 9.6 CHANGELOG `[Unreleased]` note.
