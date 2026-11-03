# Design — show synthesis progress and cancel on the queue card

## Context

The backend exposes only status transitions (`pending → processing →
ready/error/cancelled`) through `entry_updated`; there is no chunk-level
progress event. `TextEntry` also has no `started_at`/`updated_at`, so the
frontend cannot recover when a synthesis actually started.

`QueueItem` in `src/components/QueueList.tsx` already renders the persisted
failure note and retry action from #351; the progress line must integrate with
that layout without changing error rows. Cancel already exists in the
context-menu (#161), and `QueueList` owns its command handler and toasts.

## Goals / Non-Goals

**Goals:**

- A running synthesis shows elapsed time and a cancel action visible without
  hover.
- A queued synthesis says «В очереди» and offers the same cancel action.
- One tick re-renders only the ticking line, not the card or the list.
- Terminal statuses remove the line and stop the interval; unmount clears it.

**Non-Goals:**

- Percentage, ETA or chunk/stage progress.
- Backend start timestamps or a storage change.
- Cancel confirmation, queue positions, streaming audio.

## Decisions

**The frontend owns the start timestamp.** `TextEntry` has no
`started_at`/`updated_at`, and adding backend fields is outside this change.
The timer therefore starts when the row observes `processing` — either on
mount for an entry already processing, or on the `pending → processing`
transition delivered by `entry_updated`. The known limitation is that after
an application restart the timer starts over at 0:00 because the frontend
lost the original start; the row still shows that the synthesis is running.
If backend start timestamps are ever added, they replace the local ref and
the restart limitation disappears.

**One small component owns the tick.** `SynthesisProgressLine` keeps the start
timestamp in a ref and the elapsed seconds in local state; the interval calls
`setElapsedSec` once per second. Because the state lives in this child, React
re-renders only the line on a tick; `QueueItem` and `QueueList` do not
re-render. The effect is keyed on `processing`: it records the start and
starts the interval when the entry becomes processing, and its cleanup clears
the interval (and the ref) on any terminal status and on unmount. The elapsed
value is recomputed from `Date.now()` on every tick rather than incremented,
so a throttled background webview catches up instead of drifting.

**Cancel state lives in `QueueList`.** The list keeps a set of entry ids whose
cancel command is in flight, mirroring the existing `retryingIds` pattern.
The inline button is disabled while its id is in the set, which makes the
"exactly one call" rule hold even if the row re-renders; the context-menu
cancel goes through the same handler and keeps the existing localized
success/error toasts. Clearing in `finally` means a rejection re-enables the
button; a success unmounts the line through the `cancelled` entry update.

**The indeterminate bar is custom, not `Mantine Progress`.** Mantine's
`Progress` requires a numeric `value` and exposes `aria-valuenow`, so an
animated full bar would report 100% to assistive technology. The line renders
a decorative track (`aria-hidden`) with a CSS keyframe sliding a short
segment; the visible text carries the state. The animation respects
`prefers-reduced-motion`.

**Rejected alternatives.** Timer state in `QueueList` (a tick would re-render
every row). Incrementing a counter per tick (drifts after throttling or
suspend). A backend `started_at` field (storage migration, out of scope).
Percentage from an audio-duration estimate (there is no reliable total; it
would be a fake progress bar). Removing the status badge for
pending/processing (the badge is the existing status contract and the #351
card layout depends on it). Keeping cancel only in the context menu (that is
the problem this change solves).

## Risks / Trade-offs

- After an application restart the elapsed time restarts at 0:00; acceptable
  because the backend exposes no start time, and it is recorded here as the
  explicit limitation of this change.
- The start ref lives in the row, so a remount (for example the queue filter
  hides and then shows the entry) restarts the timer, like an application
  restart. Keeping it in the row is what makes a tick re-render only that
  line; the entry record offers no timestamp to restore it from.
- A system clock change can move the elapsed value; the timer is display-only
  and a corrected value on the next tick is harmless.
- A background window throttles timers; because the elapsed value is derived
  from the timestamp, the next tick shows the correct value.
- `jsdom` does not run CSS animations, so the bar's appearance stays a manual
  test item; the interval lifecycle is covered with fake timers.
