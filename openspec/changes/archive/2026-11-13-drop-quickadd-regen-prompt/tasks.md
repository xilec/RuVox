## 1. Artifacts

- [x] 1.1 Proposal, design and the `REMOVED` delta for the viewer quick-add requirement.
- [x] 1.2 The paired change `viewer-quickadd-no-offer` re-adds the requirement without the offer scenarios.
- [x] 1.3 `openspec validate drop-quickadd-regen-prompt --strict` passes.

## 2. Frontend

- [x] 2.1 Remove `RegeneratePrompt` and its state/wiring from `AppShell.tsx`; `closeDictionary` keeps only the Settings restore and the revision bump.
- [x] 2.2 Drop the viewer quick-add bookkeeping (`dictQuickAddEntry`, `dictQuickAddSaved`) and the `onSaved` marker on `DictionaryModal`.
- [x] 2.3 Remove the `onSaved` prop from `DictionaryModal` and its call sites; delete the prop's test.
- [x] 2.4 Delete `src/dialogs/RegeneratePrompt.tsx` and the `dictionary.quickadd.regen.*` strings from both locales.

## 3. Tests and docs

- [x] 3.1 Update `AppShell.dictionary.test.tsx`: drop the offer/confirm/decline/failure flows; assert that closing after a save raises nothing and issues no regeneration or player command.
- [x] 3.2 Update `docs/user-guide.md`: the two places promising the offer point at the entry's context menu instead.

## 4. Verification

- [x] 4.1 `pnpm typecheck`, `pnpm lint`, `pnpm knip`, `pnpm vitest run --dir src`.
- [x] 4.2 `cargo fmt --check` and the workspace clippy gate (no Rust code changes).
- [x] 4.3 Archive this change first, then `viewer-quickadd-no-offer`; `openspec validate --specs --strict` passes afterwards.
