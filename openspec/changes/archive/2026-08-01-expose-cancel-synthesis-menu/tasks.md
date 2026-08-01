# Tasks: Expose cancel_synthesis in the queue entry menu

## Implementation

- [x] `src/components/QueueList.tsx` — `handleCancelSynthesis` callback
  (await `commands.cancelSynthesis(id)`, error notification on failure).
- [x] `src/components/QueueList.tsx` — "Отменить синтез" Menu.Item after
  "Перегенерировать аудио", disabled unless status is `processing`.

## Tests

- [x] Component test: click on a `processing` entry's menu item calls
  `commands.cancelSynthesis` with the entry id.
- [x] Component test: the item is disabled for `ready` and `pending`
  entries.

## Validation

- [x] `nix develop -c pnpm test:unit` green.
- [x] `nix develop -c just lint` green.
- [x] openspec validate expose-cancel-synthesis-menu --strict green.
