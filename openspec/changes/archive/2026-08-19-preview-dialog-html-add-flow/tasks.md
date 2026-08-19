# Tasks: preview-dialog-html-add-flow

## 1. Decision helper

- [x] 1.1 Add `src/lib/addFlow.ts`: pure `resolveAddAction({ html, plain,
  previewEnabled, defaultFormat })` returning `empty | preview(text, format)
  | direct-html | direct-plain`, per the design matrix
- [x] 1.2 Unit tests `src/lib/addFlow.test.ts` covering all six matrix rows

## 2. AppShell wiring

- [x] 2.1 Rework `addEntry()` in `src/components/AppShell.tsx` to: probe the
  `text/html` flavor (best-effort, as today) and read plain text via the
  plugin (both up front, both best-effort), then dispatch on
  `resolveAddAction`
- [x] 2.2 Track the per-opening preview format in state
  (`previewFormat: EntryFormat | null`) and pass
  `defaultFormat={previewFormat ?? toEntryFormat(config?.text_format)}` to
  `PreviewDialog`; clear it on close/cancel
- [x] 2.3 Preserve today's direct-path semantics exactly: HTML extraction
  with no readable text falls back to plain (or no-ops when both empty)

## 3. Gates

- [x] 3.1 `nix develop -c pnpm typecheck` / `pnpm test:unit` / `pnpm lint`
  green
- [ ] 3.2 `nix develop -c just test` green (Rust unaffected, run anyway)
- [ ] 3.3 Verify on the Win10 VM: Add with browser-copied (HTML) content
  opens «Предпросмотр нормализации» with the selector on `html`; Add with
  plain text opens it as before; with the dialog disabled, behavior is
  unchanged (deferred to the epic's final VM pass after #196)

## 4. Archive

- [x] 4.1 `openspec validate` the change, sync delta specs, archive the
  change, commit the archive
