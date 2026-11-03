## Why

A `processing` entry shows only an 8 px loader inside its status badge, and a
`pending` entry only says «Ожидание»; neither tells how long the synthesis has
been running. On long articles the user cannot tell whether the queue is alive
or stuck, and the only way to cancel is the right-click menu (#161) — an
action that is invisible until discovered.

## What Changes

- A queue card in `processing` gains an indeterminate progress line under the
  title, the elapsed-stage text «Синтез… M:SS», and an inline «Отменить»
  button that is visible without hovering. The elapsed time is counted from
  the moment the frontend observed the entry become `processing`; the entry
  record carries no start timestamp, so after an application restart the timer
  starts again.
- A `pending` card shows «В очереди» and the same cancel button, without a
  timer.
- The inline cancel invokes `cancel_synthesis`, disables its button until the
  command settles, and keeps the existing localized toasts. The context-menu
  «Отменить синтез» action keeps working unchanged.
- A small per-row component owns the one-second tick, so a tick re-renders
  only the ticking line, not the item or the queue list; the interval is
  cleared when the entry reaches any terminal status and on unmount.
- RU and EN catalogs get the new keys; the cancel button's accessible label
  names the entry.
- Component tests with fake timers pin the timer lifecycle, the pending row,
  the single cancel invocation with its busy state, and the per-row render
  isolation.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `queue-lifecycle`: the queue-list rendering requirement gains the
  processing/pending progress presentation and the timer semantics (start at
  the observed `processing` transition, stop at any terminal status, restart
  on application restart), and a new inline-synthesis-cancel requirement
  covers the always-visible cancel action for pending/processing cards.
- `ui`: a new "Synthesis progress on the queue card" requirement defines the
  indeterminate bar, the elapsed text, the inline cancel button, the i18n
  keys, the accessible label and the render isolation of the tick.

## Impact

- Affected code: `src/components/QueueList.tsx`,
  `src/components/QueueList.module.css`, `src/i18n/ru.ts`,
  `src/i18n/en.ts`, `src/components/QueueList.progress.test.tsx`,
  `CHANGELOG.md`.
- No IPC, storage or backend changes: `TextEntry` has no start timestamp and
  the backend emits only status transitions; no percentage/ETA/chunk event is
  introduced.
- User-visible: a running synthesis shows elapsed time and offers a visible
  cancel action; a queued entry says «В очереди».

## Non-goals

- Percentage, ETA or chunk/stage events — they need a new backend progress
  event and are a separate change (#352 is presentation only).
- Backend `started_at`/`updated_at` fields or a storage migration.
- A cancel confirmation dialog, queue-position numbers, streaming audio
  (#347).
- Changing the context-menu cancel behavior beyond sharing its command
  handler.
