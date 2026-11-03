# Tasks — surface synthesis errors on the queue card

## 1. OpenSpec

- [x] 1.1 Proposal, design, and deltas for `queue-lifecycle` (persisted
  failure reason) and `ui` (synthesis failure on the queue card).

## 2. Frontend

- [x] 2.1 i18n: add the RU/EN keys for the error prefix, «Повторить»,
  «Показать полностью» / «Свернуть», the failed-regeneration note and its
  hint; keep both catalogs complete.
- [x] 2.2 `QueueItem`: red error block for `status = error` (localized prefix +
  verbatim message, two-line clamp, measured overflow, expand/collapse, full
  text in `title`, selectable text).
- [x] 2.3 `QueueItem`: amber note for `ready` with `error_message`, keeping the
  row and its Play action healthy.
- [x] 2.4 `QueueList`: retry action calling `commands.regenerateEntry(id,
  false)` once, busy until the entry's `entry_updated` clears the retryable
  error, red toast on rejection.
- [x] 2.5 `QueueList.module.css`: red row treatment (3 px left border, tint,
  padding compensation) and the note styles; no layout change for healthy
  rows.

## 3. Tests

- [x] 3.1 Error message renders from the entry (not from an event).
- [x] 3.2 Expand/collapse toggles the clamp when the message overflows, and no
  control is shown when it fits.
- [x] 3.3 Retry invokes `regenerateEntry(id, false)` exactly once and is
  disabled while busy; `entry_updated` clears the busy state; a rejection
  clears it and shows the red toast.
- [x] 3.4 `ready` + `error_message` shows the amber note, keeps Play enabled,
  and shows no red treatment.
- [x] 3.5 Healthy rows render no error block/retry/note.

## 4. Validation

- [x] 4.1 `pnpm typecheck`
- [x] 4.2 `pnpm test:unit`
- [x] 4.3 `just lint` (fmt/clippy/deny/eslint/knip/typecheck)
- [x] 4.4 `openspec validate surface-synthesis-errors --strict`, then archive
  the change and sync the specs.
- [x] 4.5 CHANGELOG `[Unreleased]` note.
