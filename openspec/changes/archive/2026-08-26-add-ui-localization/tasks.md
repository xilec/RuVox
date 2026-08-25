# Tasks: add-ui-localization

## 1. Backend

- [x] 1.1 Add `language: String` (default `"ru"`) to `UIConfig` in
      `storage/schema.rs` (+ patch field + schema tests).
- [x] 1.2 Refactor `CommandError` to `{ type, code, params, message? }`;
      assign site codes across all `commands/mod.rs` error sites; update
      Rust tests.

## 2. Frontend core

- [x] 2.1 Create `src/i18n/ru.ts`, `src/i18n/en.ts` (typed against RU),
      `src/lib/i18n.ts` (`t`, interpolation), `src/stores/locale.ts`.
- [x] 2.2 Localize error rendering in `src/lib/errors.ts`
      (`formatError`: code → message → per-type generic).
- [x] 2.3 Seed the locale store from `getConfig()` at App start.

## 3. Migration

- [x] 3.1 Migrate components: AppShell, Player, QueueList, TextViewer,
      ViewerContextMenu.
- [x] 3.2 Migrate dialogs: Settings (incl. language selector wired to
      updateConfig), PreviewDialog, SileroBundlePrompt, CleanupCacheModal.
- [x] 3.3 Migrate non-React modules: notificationBridge, updater,
      viewerCopy, ingest/AppShell helpers.

## 4. Highlight theme

- [x] 4.1 Replace the static hljs import with scheme-driven inline style
      injection; verify both schemes without reload.

## 5. Verification

- [x] 5.1 Unit tests: `t` interpolation/fallback, `formatError` priority
      chain, locale store seeding; Rust schema round-trip for `language`.
- [x] 5.2 Gates green: cargo test/clippy, pnpm typecheck/lint/test:unit.
- [x] 5.3 Manual pass checklist: RU default on fresh config, EN switch
      relabels everything incl. a triggered error toast, highlight theme
      follows light/dark, language persists across relaunch.
