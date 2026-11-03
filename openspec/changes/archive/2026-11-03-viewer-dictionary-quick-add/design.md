# Design — viewer dictionary quick-add

## Context

The dictionary editor is one shared, always-mounted `DictionaryModal` in
`AppShell`; every entry point only seeds `dictInitialFrom` and opens it. The
editor saves immediately per mutation (`save_dictionary` on submit, delete and
import) and never has a "save and close" action — the user closes it when
done. `dictRevision` is bumped on every close so open preview dialogs
re-normalize.

The viewer's `ViewerContextMenu` already receives the click target and the
current selection; `isSingleSourceToken` is the shared gate the preview
quick-add uses. Regeneration already exists as a two-step flow: the queue
context menu sets `regenEntry`, the preview dialog confirms, and
`handleRegenConfirm` runs `regenerate_entry` (delete-then-synthesize; the old
audio is kept when synthesis fails).

## Goals / Non-Goals

**Goals:**

- A word noticed in the viewer reaches the dictionary in one action, in every
  display mode.
- After a save, the app offers to refresh the stale recording without a detour
  through the queue menu.
- The editor never interrupts playback; a confirmed regeneration keeps the
  existing stop-and-replace behavior.
- The prompt appears only when the dictionary actually changed.

**Non-Goals:**

- Multi-word entries, automatic re-synthesis on dictionary changes, inline
  pronunciation editing.
- Changing the queue regeneration flow, the preview quick-add or the player.

## Decisions

**The context menu only gates and dispatches.** `ViewerContextMenu` gains an
optional `onAddToDictionary(word)`; the item renders when the selection is
exactly one valid source token, using the same `isSingleSourceToken` helper the
preview quick-add uses. `TextViewer` forwards the callback to `AppShell`;
nothing else in the viewer changes. Rejected: duplicating the token regex in
the menu (a second source of truth that would drift from the editor's
validation).

The menu label reuses the existing `dictionary.quickadd.button` string
(«В словарь» / "To dictionary"): it is the same action as the preview
quick-add, so one translated string covers both.

**Track writes with an `onSaved` callback, not by diffing state.**
`DictionaryModal` saves immediately and has no save-and-close action, so
"closed on a save" cannot be derived from the close event, and `dictRevision`
bumps on every close regardless. The modal therefore invokes an optional
`onSaved` after each successful write (submit, delete, import). `AppShell`
remembers whether the current editor session was opened from the viewer and
whether it reported a write; only that pair raises the offer. Rejected:
comparing the entry list before/after the session (needs a full re-read, races
with the immediate-save model, and still cannot tell "no save" from "saved the
same bytes").

**A compact confirmation instead of the regeneration preview.** The issue's
sketch suggested reusing `setRegenEntry`, i.e. opening the full regeneration
preview after every quick-add. That window shows the normalized text and a
"Read now" switch; after adding a word the user already knows what is stale,
and a second full preview reads as a detour. The change adds a small prompt
(`RegeneratePrompt`, title «Переозвучить сейчас?») and confirms through the
existing `handleRegenConfirm` with an explicit entry, so the actual
regeneration — delete-then-synthesize, old audio kept on failure, localized
error toast — stays in one place. Rejected: a toast with an action button (the
offer must not disappear before the user decides).

**Regeneration does not auto-play.** The prompt exists to replace the stale
recording, so it calls the existing path with `play_when_ready = false`. The
backend stops the currently playing entry before regenerating (existing
behavior); the fresh audio then waits in the queue. Rejected: starting
playback after a silent editor session (hijacks whatever the user was doing).

**The entry is captured when the editor opens.** The recording that becomes
stale is the one the viewer showed. The editor is modal, but capturing the
entry keeps the offer correct if `entry_updated` events change the selection
state while the editor is open; regeneration is addressed by id, so a stale
snapshot is harmless.

**Playback is untouched by the editor.** Opening/closing issues no player
command; only the confirmed regeneration reaches `regenerate_entry`, which
stops the player when that entry is playing. The integration test asserts no
pause/stop/play command in the open-close path.

## Risks / Trade-offs

- Any write in the session (including a delete or an import) can stale the
  recording, so the offer appears for those too; the user can decline and the
  recording stays as it is.
- Several saves collapse into one offer at close; regenerating once after the
  session is the intended granularity.
- `onSaved` is a new optional prop on the shared editor; the Settings and
  preview quick-add entry points pass nothing and keep their behavior.
