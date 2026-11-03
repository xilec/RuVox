# Tasks — show synthesis progress and cancel on the queue card

## 1. OpenSpec

- [x] 1.1 Proposal, design, and deltas for `queue-lifecycle` (progress
  presentation and inline cancel) and `ui` (synthesis progress on the queue
  card).

## 2. Frontend

- [x] 2.1 i18n: add the RU/EN keys for «Синтез… {time}», «В очереди»,
  «Отменить» and the accessible cancel label; keep both catalogs complete.
- [x] 2.2 `SynthesisProgressLine`: indeterminate bar, per-row start
  timestamp, one-second tick derived from the timestamp, inline cancel button
  with an accessible label, cleanup on terminal status and unmount.
- [x] 2.3 `QueueItem`: render the line under the title for `pending` and
  `processing`, keeping the #351 failure note and retry intact; cancel stays
  outside the hover-only action group.
- [x] 2.4 `QueueList`: track in-flight cancels per entry, disable the button
  while the command runs, reuse the existing localized toasts, keep the
  context-menu cancel working through the same handler.
- [x] 2.5 `QueueList.module.css`: progress track/segment keyframes with
  `prefers-reduced-motion`, line layout, cancel button sizing.

## 3. Tests

- [x] 3.1 Fake timers: the timer starts when the entry becomes `processing`
  (not on `pending` mount) and the elapsed text advances once per second.
- [x] 3.2 Fake timers: the interval stops and the line disappears on
  `ready`, `error` and `cancelled`; unmount clears the timer.
- [x] 3.3 A `pending` row shows «В очереди» and cancel, with no timer.
- [x] 3.4 Cancel invokes `cancel_synthesis` exactly once, is disabled while
  the call is in flight, re-enables on rejection, and shows the existing red
  toast.
- [x] 3.5 Render-count test: a tick updates the elapsed text without
  re-rendering the queue rows (mocked dialog render counter).
- [x] 3.6 The cancel button's accessible label names the entry; the EN
  catalog labels render after a locale switch.

## 4. Validation

- [x] 4.1 `pnpm typecheck`
- [x] 4.2 `pnpm test:unit`
- [x] 4.3 `just lint` (fmt/clippy/deny/eslint/knip/typecheck)
- [x] 4.4 `openspec validate show-synthesis-progress --strict`, then archive
  the change and sync the specs.
- [x] 4.5 CHANGELOG `[Unreleased]` note.
