# Proposal: Expose cancel_synthesis in the queue entry menu (#161)

## Summary

The backend `cancel_synthesis(id)` command is fully wired and tested
(#88, #127), but no UI calls it: the queue entry context menu offers only
"Воспроизвести", "Перегенерировать аудио", and "Удалить". While a synthesis
runs, the user cannot abort it from the app.

Add an "Отменить синтез" item to the queue entry context menu, enabled only
while the entry's status is `processing`, calling
`commands.cancelSynthesis(entry.id)`.

## Capabilities

- `queue-lifecycle` (modified — Per-entry actions)

## Non-goals

- No backend changes: abort semantics (entry back to `pending`, late
  completion discarded, ttsd kill) are already settled and tested.
- No confirmation dialog: cancellation is non-destructive (the entry stays
  in the queue and can be regenerated), unlike deletion.
- No local status mutation on click: the backend emits `entry_updated`,
  which the existing subscription applies.

## Approach

`QueueList.tsx`: new `handleCancelSynthesis` (await
`commands.cancelSynthesis`, error notification on failure, mirroring
`handleRegenerate`) and a Menu.Item placed after "Перегенерировать аудио",
`disabled` unless `menu.entry.status === 'processing'`. Component test
(mocked `../lib/tauri`) covers: click on a `processing` entry calls
`cancelSynthesis` with its id; the item is disabled for `ready`/`pending`
entries.
