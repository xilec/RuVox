# Design — editable title and note header

## Context

`TextViewer` currently starts with a toolbar whose left side is the format
`SegmentedControl` and whose right side is the reading-typography controls from
#354/#357. The queue identifies an entry by `original_text.slice(0, 60)`. The
storage schema has no entry-level identity besides the text itself, and
`history.json` is a versioned document that must keep loading without
migration.

Issue #353 adds the identity: `title`/`note` fields, an editable header in the
viewer, a title-first queue, and search across title/note/text.

## Decisions

### 1. Seed the title in the storage service, not in the callers

Both ingestion paths (`add_text_entry` / `add_clipboard_entry` in
`commands/mod.rs` and the tray's clipboard poller in `lib.rs`) already
converge on `StorageService::add_entry_with_source` (`add_entry` delegates to
it). Seeding there makes the rule impossible to miss for a future ingestion
path and keeps the tray consistent with the UI without touching the tray code.

Rejected alternative: seeding in `ingest_text` and the tray handler. Two call
sites is exactly the drift the shared storage path exists to prevent, and the
tray path has no frontend to derive a title.

### 2. The derivation is duplicated in Rust and TypeScript — deliberately, with a parity lock

The backend must seed a title without a webview round-trip, and the viewer must
derive one for legacy entries whose `title` is `None` (no migration). That
makes two implementations of the same algorithm unavoidable unless the
frontend calls a `derive_title` command per entry and per render.

Risk: implementation drift would make a freshly added entry and an identical
legacy entry render different titles. Mitigation:

- one shared case table (headings, list bullets, ordered lists, whitespace
  collapsing, empty/markup-only input, the 200-character cap with a trailing
  space) duplicated verbatim in
  `src-tauri/src/storage/title.rs` and `src/lib/entryTitle.test.ts`; both
  suites must be edited together;
- the two modules cross-reference each other in their doc comments.

The table is the cheapest lock that fits the repo's test style. A build-time
codegen step (like the Piper catalogue mirror) is not worth it for a ~40-line
pure function; if the algorithm grows, revisit.

### 3. Two commands instead of an `Option<Option<String>>` patch

Tauri serializes `Option<T>` as `null`/missing, so a single
`set_entry_metadata(patch)` command cannot distinguish "leave unchanged" from
"clear" for an optional string field. `set_entry_title(id, title)` and
`set_entry_note(id, note)` treat `null` and an empty/whitespace-only string as
"clear to `None`" — the only meaning the UI needs, because clearing is what
falls back to the derived title / hides the note.

Normalization happens in the command (trim; empty → `None`; cap at 200 / 2000
Unicode scalar values, re-trimming after the cap). The UI also limits input
length, but the backend limit is authoritative: a capped value never reaches
`history.json` longer than the contract.

### 4. Atomic writes through `update_entry_if`

The header edits touch a single field while synthesis may be finishing in the
background. A read-modify-`update_entry` clone would persist a stale status
over a `ready` completion — the lost-update bug fixed in #363. Both commands
therefore mutate the field under the storage write lock via
`StorageService::update_entry_if(&uuid, |_| true, |e| e.title = value)` and
emit `entry_updated` from the entry read back afterwards.

### 5. Header layout: one meta row with a controls slot

```
+--------------------------------------------------------------+
| Заголовок                                              [карандаш] |
| Заметка (2 строки) …                  Показать полностью  [карандаш] |
| 2 нояб. 2026 г., 14:03 · Готово · Silero (нативный) · Xenia  [Plain|MD|HTML] [Узко|Широко] [− 100% +] |
+--------------------------------------------------------------+
| текст, прокручивается                                          |
```

`EntryHeader` owns the identity and metadata and exposes a `controls` slot
rendered at the right end of the metadata row. `TextViewer` passes the format
switch and the reading controls into that slot, so the format switch visibly
moves "to the right of the meta row" as in the issue mockup while #354/#357
controls stay reachable in every display mode. The row wraps (`flex-wrap`) on
narrow panes instead of clipping.

Rejected alternatives:

- **A second toolbar row above the header** for reading controls: costs a row
  of reading height for every entry, and the mockup already puts the format
  switch in the meta row.
- **Moving the reading controls into the Settings dialog only**: they are
  frequent reading adjustments; #354/#357 acceptance expects them in the
  viewer.

The title is a `<button>` styled as a heading, not an `<h1>/<h2>`: the rendered
document owns the heading hierarchy (tests and anchors query `h2` for the
document's own headings), and a real button gives keyboard access to the
click-to-edit affordance for free. The pencil is an `ActionIcon` shown on
hover/focus-within.

### 6. Drafts are local state; optimistic values are overrides

`EntryHeader` keeps `titleDraft`/`noteDraft` only while editing, seeded from
the persisted value when editing starts. An incoming `entry_updated` (status
change, regeneration) re-renders the header but never touches those states, so
an open draft survives — this is the exact bug the issue calls out.

Committed values are shown immediately through `titleOverride`/`noteOverride`
(`undefined` = render the persisted field). A successful `entry_updated` clears
the override once `entry.title` equals it; a rejected IPC call restores the
previous override (usually `undefined`, i.e. the persisted value) and shows the
red localized toast. `skipBlur` refs prevent a second commit when the input
unmounts after Enter/Esc/Save.

### 7. One display vocabulary for engine, voice and dates

`displayEngine`/`displayVoice` and the naive-UTC date formatter were local to
`GenerationParamsDialog`; the header shows the same facts, so they move to
`src/lib/generationDisplay.ts` / `src/lib/datetime.ts`. The voice label becomes
engine-aware:

- Piper → the localized `PIPER_VOICES` label;
- Silero (both engines) → the speaker's proper name (`Xenia`), or the
  localized «Случайный» for `random`;
- TeraTTS → the stored id (`ru_m5`), which is already human-readable;
- unknown id → the stored id, never a guessed name.

The Silero speaker list moves from `Settings.tsx` into `src/lib/sileroVoices.ts`
so the Settings selector and the read-only labels cannot drift. The status
badge mapping moves to `src/lib/entryStatus.ts` for the same reason (queue card
and header).

The header formats `created_at` with the same `toLocaleString`-based helper as
the dialog, tagged by the app language (`ru-RU` / `en-US`), so the two surfaces
cannot disagree about the timestamp.

### 8. Queue identity and search

Queue cards render `entryDisplayTitle(entry)` — stored title, else the derived
fallback, one line with CSS ellipsis and the full title in the native `title`
tooltip. Search keeps the existing case-insensitive substring semantics but
matches any of `title`, `note`, `original_text`. The derived title is built
from `original_text`, so legacy entries remain findable through the text field.

### 9. Notes never reach TTS

The note is metadata only: it is not concatenated into `original_text`, not
part of normalization, and the commands never touch `normalized_text` or
storage audio fields. The orchestration test asserts the normalized text is
unchanged after editing a note on a ready entry.
