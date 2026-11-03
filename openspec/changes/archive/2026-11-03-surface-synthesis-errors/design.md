# Design — surface synthesis errors on the queue card

## Context

The backend already does the hard part: a failed synthesis persists the engine
message in `TextEntry.error_message`, a failed regeneration restores `ready`
with the previous recording and the same message, and `regenerate_entry`
clears the message when it starts. The frontend only surfaces failures as a
five-second `tts_error` toast (`src/lib/notificationBridge.ts`) and a red badge,
so the stored reason is invisible after the toast closes. The queue card
(`QueueItem` in `src/components/QueueList.tsx`) is the natural owner of the
persistent failure state: it already renders the entry from `TextEntry` state
fed by `getEntries` and `entry_updated`, which makes the display restart-safe
by construction.

## Goals / Non-Goals

**Goals:**

- The stored failure reason is visible on the card whenever the entry is in
  `error`, including after a restart, without depending on any event.
- The failure is actionable: one click starts a new synthesis without the
  regeneration preview (the text has not changed).
- A failed regeneration of a playable entry is visibly distinct from a
  successful one while the old audio keeps playing.
- Healthy rows keep their current layout and the toast keeps its current
  behavior.

**Non-Goals:**

- Error taxonomy, automatic retry/backoff, a global error center.
- Parsing, classifying or localizing engine messages.
- Changing the context-menu regeneration preview flow or the toast duration.

## Decisions

**Render from `TextEntry`, not from events.** The card reads `entry.status` and
`entry.error_message`; `tts_error` keeps feeding the toast only. A component
that rendered from the event would lose the reason on restart and on re-render,
which is the exact failure this change fixes.

**Retry calls the command from the queue, not through `AppShell`.** The
existing `onRegenerate` prop hands the entry to `AppShell`, which opens the
regeneration preview. Retry is failure recovery for unchanged text, so
`QueueList` calls `commands.regenerateEntry(id, false)` itself. The busy state
is a set of entry ids in `QueueList`, cleared by that entry's `entry_updated`
when it leaves the retryable error state (the command emits one with
`pending` + `error_message: null` before synthesis starts) and by the command's
rejection path, which also shows the existing localized red toast. Using the
event rather than the promise keeps the button busy through the actual state
change, matching the user's mental model of "until the retry starts".

**Overflow is measured, not guessed.** The two-line clamp hides the true height
from layout, so the expand control appears only when
`scrollHeight > clientHeight` on the clamped element. A character-count
threshold would show the control for short messages that happen to contain
wide glyphs and hide it for long ones at narrow widths. The queue navbar is
user-resizable, so a `ResizeObserver` re-measures on width changes; the
observer is disconnected while expanded (where the element is not clamped and
would always report no overflow) and the measured flag is kept until the
element is clamped again. `-webkit-line-clamp` is supported by both webview
backends (WebKitGTK and WebView2).

**One note component for both tones.** The red `error` block and the amber
`ready`-with-error note share the message/clamp/expand/retry structure and
differ only in tone, prefix and the extra hint line, so they render through one
internal component. The red tone also tints the row and adds the 3 px left
border; the amber tone leaves the row healthy so the recording reads as
playable.

**Rejected alternatives.** Showing «Показать полностью» unconditionally: noise
on the common short engine messages. Keeping retry only in the context menu:
that path opens the preview dialog, an extra decision for unchanged text.
Storing the expanded state per entry outside the component: the card is
unmounted on restart anyway and the entry's message identifies the state; a
local `useState` reset on message change is enough. Rendering the message in a
tooltip/popover only: not selectable or copyable, and not persistent.

## Risks / Trade-offs

- Busy state depends on the entry's `entry_updated` event; if it were ever
  dropped the button would stay busy. `regenerate_entry` emits it
  synchronously before spawning synthesis, and the rejection path clears the
  state, so the window for a stuck button is a lost in-process event.
- `jsdom` reports zero layout heights, so the overflow tests stub
  `scrollHeight`/`clientHeight`; the real measurement stays a manual-pass item.
- The amber note keys off `error_message`, not a separate "last regeneration
  failed" flag: a `ready` entry with a message always means a failed
  regeneration by the backend contract, so no new storage field is needed.
