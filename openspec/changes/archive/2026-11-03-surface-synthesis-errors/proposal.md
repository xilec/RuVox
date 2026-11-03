## Why

Synthesis failures are visible only as a five-second toast and a red status
badge; the persisted `TextEntry.error_message` is rendered nowhere (the backend
itself notes this in `src-tauri/src/commands/synthesis.rs`). Once the toast is
gone the user cannot tell what failed, and a failed *regeneration* of a
playable entry returns it to `ready` — visually indistinguishable from a
successful one. Retrying today requires the right-click menu, which opens the
regeneration preview even though the text has not changed.

## What Changes

- A queue card with `status = error` renders the stored `error_message`: a
  red-tinted row with a 3 px left border, and below the title the localized
  prefix «Ошибка синтеза» followed by the engine message verbatim (no parsing,
  classification or translation), clamped to two lines with «Показать
  полностью» when it overflows, the full text in the `title` tooltip, and
  selectable/copyable text.
- Both failure variants gain a «Повторить» action that calls
  `commands.regenerateEntry(id, false)` directly — no regeneration preview.
  The action is busy until the entry's `entry_updated` clears the retryable
  error state; a rejected command surfaces the existing localized red toast.
- A playable entry (`ready`) whose `error_message` is set — a failed
  regeneration that kept the previous recording — shows an amber inline note
  «Последняя перегенерация не удалась: …» with the hint that the previous
  recording still plays and the same retry action. The audio stays playable
  and the note disappears when the next regeneration starts.
- RU and EN catalogs get the new keys; the `tts_error` toast keeps its current
  behavior and healthy cards keep their current layout.
- Component tests pin rendering from the entry (not from an event),
  expand/collapse, the single retry invocation with its busy state, the
  ready-plus-error variant, and unchanged healthy rows.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `queue-lifecycle`: a new "Persisted failure reason" requirement makes
  `error_message` an explicit user-visible contract: stored verbatim, cleared
  when a regeneration starts, and restart-safe.
- `ui`: a new "Synthesis failure on the queue card" requirement defines the
  red error block, the two-line clamp with expand/collapse, the amber
  failed-regeneration note, and the direct retry action.

## Impact

- Affected code: `src/components/QueueList.tsx` (the `QueueItem` card),
  `src/components/QueueList.module.css`, `src/i18n/ru.ts`, `src/i18n/en.ts`,
  `src/components/QueueList.test.tsx`, `CHANGELOG.md`.
- No IPC, storage format or backend changes: `regenerate_entry` already clears
  `error_message` when it starts and restores `ready` with the message on a
  failed regeneration.
- User-visible: synthesis failures become a persistent, actionable state of the
  queue card.

## Non-goals

- Error classification/taxonomy, automatic retries or backoff, a global error
  center.
- Rewriting, parsing or localizing engine messages; only the prefix and the
  action labels are localized.
- Changing the `tts_error` toast duration or routing.
- Touching the regeneration preview flow from the context menu.
