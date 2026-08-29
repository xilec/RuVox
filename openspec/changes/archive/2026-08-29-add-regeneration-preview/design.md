# Design: add-regeneration-preview

## Context

`regenerate_entry` (src-tauri/src/commands/mod.rs) deletes the audio before
spawning synthesis; `QueueList.handleRegenerate` invokes it straight from the
context menu. The preview dialog (`src/dialogs/PreviewDialog.tsx`) is a
controlled component driven entirely by props (`text`, `opened`,
`initialFormat`, `onSynthesize`, `onCancel`), already renders a live
normalization preview via `previewNormalize`, and is mounted from
`AppShell`. Synthesis always normalizes `entry.original_text` directly — the
entry `format` affects viewer rendering and ingest-time extraction only, and
for HTML-ingested entries `original_text` already stores the extracted TTS
text.

## Goals / Non-Goals

Goals:

- The regeneration path goes through the same preview dialog before any
  deletion happens.
- Cancel is a true no-op for the entry.
- The dialog never lies: every visible control in regeneration mode does
  what it says.

Non-Goals:

- Editing an entry's text (a "regenerate with edited text" feature — needs a
  text-update backend command and revisits `original_text` immutability).
- Changing the Add-flow preview gate (`preview_dialog_enabled`) or letting
  the in-dialog opt-out checkbox affect the regeneration preview.
- Previewing normalization *differences* (old audio vs. would-be audio) —
  the dialog shows the plain normalization, as in the Add flow.

## Decisions

### D1: Reuse `PreviewDialog` with a `mode: 'add' | 'regenerate'` prop (default `'add'`)

A second instance of the component is rendered by `AppShell` for the
regeneration flow, with its own state. Alternatives:

- One shared instance keyed by a discriminated flow state — would fold
  `previewOpen/previewText/previewFormat/previewPlainFallback/previewSource`
  into a union, touching every Add/paste/import call site in `AppShell` for
  no behavioral gain.
- A separate lighter dialog component — duplicates the panes/normalization
  machinery for no benefit; the issue explicitly asks for "the same preview
  dialog".

`mode` is a single union prop, not boolean soup; per-mode UI differences are
enumerated in one place (the dialog component).

### D2: Regeneration mode forces the plain preview path

In regeneration mode the effective format is hard-wired to `plain`:
`previewTextFor(text, 'plain')` returns the text unchanged, so the right pane
shows exactly the normalization `spawn_synthesis` will run over
`original_text`. Format detection and HTML extraction stay off — re-extracting
`html_source` could legitimately differ from the stored `original_text`, and
that mismatch, not the markup, is what regeneration will narrate. The
selector that could switch formats is hidden (D3), so the internal state
stays consistent.

### D3: Hide «Редактировать», the source-format selector, and «Больше не показывать этот диалог» in regeneration mode

Each hidden control would otherwise mislead:

- Edit — the entry text is immutable; edits cannot be applied without a new
  backend command (Non-Goal).
- Format selector — regeneration does not consult it (D2).
- Opt-out checkbox — its persisted effect (`preview_dialog_enabled: false`)
  gates the Add flow only; silently weakening the regeneration preview (the
  whole point of #253) would be wrong.

«Read Now» stays and is honored (D4). The confirm button gets its own label
«Перегенерировать» (`preview.regenerate` i18n key) so the dialog states which
action will run.

### D4: Wire `play_when_ready` through `regenerate_entry`

The switch must not be a no-op. The backend change is mechanical: the command
takes `play_when_ready: bool` and forwards it to `spawn_synthesis` (which
already implements the autoplay-on-ready rule for the initial synthesis) —
previously hard-coded `false`. The Tauri command has exactly one caller (the
frontend wrapper), so the signature change is non-breaking in practice; the
Rust integration tests are updated in the same change.

### D5: Confirm-side ownership moves to `AppShell`; `QueueList` delegates via an `onRegenerate(entry)` prop

The dialog must be opened by the component that mounts it (`AppShell`), so
the context-menu handler passes the whole `TextEntry` (it already carries
`original_text`) up through a new required prop, and the confirm handler —
invoke + blue/red notifications, moved from `QueueList.handleRegenerate` —
lives next to the dialog state. `QueueList` keeps the `processing`
disabled-gate on the menu item; the backend rejection remains the second
line of defense. No new store: a prop matches how `AppShell` already owns the
Add-flow dialog state, and the queue list renders inside `AppShell` directly.

## Risks / Trade-offs

- [Stale entry snapshot while the dialog is open (entry_updated events land
  under it)] → regeneration works on the entry id; the backend re-reads the
  entry, so only the *preview* can be stale, and the user can reopen it.
- [Playback stop happens only inside the backend confirm path] → unchanged
  from today: a playing entry keeps playing while the preview is open and is
  stopped by `regenerate_entry` only after confirmation — cancel keeps audio
  *and* playback, which is the desired semantics.
- [Two mounted dialog instances] → only one is ever open; both are inert
  (`return null`) while closed, matching the existing single-instance
  behavior.

## Migration Plan

Single PR; no data migration. Rollback = revert the commit (the backend
parameter defaults are supplied by the only caller).

## Open Questions

None.
