# Tasks: add-regeneration-preview

## 1. Backend: play-when-ready parameter

- [x] 1.1 `src-tauri/src/commands/mod.rs`: `regenerate_entry` takes
      `play_when_ready: bool` and forwards it to `spawn_synthesis` instead of
      the hard-coded `false`.
- [x] 1.2 Update the regenerate integration tests in
      `src-tauri/src/commands/orchestration_tests.rs` for the new signature;
      add a case asserting the fresh audio autoplays with
      `play_when_ready: true`.

## 2. Preview dialog: regeneration mode

- [x] 2.1 `src/dialogs/PreviewDialog.tsx`: add the `mode` prop
      (`'add' | 'regenerate'`, default `'add'`); in regeneration mode force
      the plain preview path (no detection, no HTML extraction) and hide
      «Редактировать», the source-format selector, and «Больше не показывать
      этот диалог»; label the confirm button from the new
      `preview.regenerate` key.
- [x] 2.2 `src/lib/tauri.ts`: `regenerateEntry(id, playWhenReady)`.
- [x] 2.3 i18n: `preview.regenerate` in `src/i18n/ru.ts` / `en.ts`.

## 3. Flow wiring

- [x] 3.1 `src/components/QueueList.tsx`: replace the direct
      `regenerateEntry` invocation with an `onRegenerate(entry)` prop call on
      the context-menu item (the `processing` disabled gate stays).
- [x] 3.2 `src/components/AppShell.tsx`: hold the regeneration preview state
      (entry snapshot), render the second `PreviewDialog` instance in
      regeneration mode, and own the confirm handler —
      `commands.regenerateEntry(id, playWhenReady)` plus the blue/red
      notifications moved from `QueueList`.

## 4. Tests

- [x] 4.1 `PreviewDialog.test.tsx`: regeneration mode renders without Edit /
      selector / opt-out, normalizes the text as-is (plain path), confirms
      with the unchanged text and the switch state, confirm button uses the
      regeneration label.
- [x] 4.2 `QueueList.test.tsx`: the context-menu action calls `onRegenerate`
      with the entry; existing backend-invoke assertions are replaced.

## 5. Docs & validation

- [ ] 5.1 Add the 1–2-line `[Unreleased]` CHANGELOG note (user-visible
      behavior; additive diff per the human-owned-file rule).
- [x] 5.2 Gates green: `nix develop -c just lint`, `nix develop -c just test`
      (cargo with `CARGO_TARGET_DIR=<worktree>/dist` per the espeak long-path
      workaround), `pnpm dlx @fission-ai/openspec validate --specs --strict`.
- [ ] 5.3 Manual pass checklist to the user: regenerate a ready entry —
      preview opens with normalization, cancel keeps audio playable,
      confirm regenerates; «Read Now» on autoplays the fresh audio; an HTML
      entry previews the stored text without re-extraction.
