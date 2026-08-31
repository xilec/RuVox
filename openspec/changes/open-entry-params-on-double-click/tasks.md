## 1. Implementation

- [x] 1.1 Add `onOpenParams: (entry: TextEntry) => void` to `QueueItemProps`, handle `onDoubleClick` on the item div, and pass a gated opener from `QueueList` (`generation !== null || audio_generated_at !== null`); verify with `pnpm typecheck`
- [x] 1.2 Add QueueList tests: double-click on an entry with a snapshot opens the dialog; double-click on a never-synthesized entry does not; verify with `pnpm test:unit`

## 2. Validation

- [x] 2.1 Run `nix develop -c just test` and `nix develop -c just lint` — all suites and static checks green
- [ ] 2.2 Manual pass: double-click a synthesized entry opens «Параметры записи…»; double-click a pending entry does nothing; single click and context menu unchanged
