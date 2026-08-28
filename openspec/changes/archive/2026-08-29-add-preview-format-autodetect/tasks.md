## 1. Detector

- [x] 1.1 Add `src/lib/detectFormat.ts` with the pure `detectFormat(text)` classifier (html prefix / ≥3 tag fragments; markdown heading, fence, ≥3 list lines, ≥2 inline links; else plain) and exported threshold constants per design.md
- [x] 1.2 Add `src/lib/detectFormat.test.ts` covering the classification matrix from the spec scenarios: full document, multi-tag fragment, `a < b` prose, `<T>` generics, single stray tag, each markdown signal, sub-threshold list/link counts, empty/whitespace text

## 2. Preview dialog and import wiring

- [x] 2.1 Switch the dialog selector state to `EntryFormat | 'auto'` with the effective format resolved for the preview effect and `onSynthesize`; the dialog takes an optional `initialFormat` (import preselection) instead of `defaultFormat`
- [x] 2.2 Add the «Авто» option to the selector with a label showing the detected format; update `AppShell` call sites (clipboard previews open auto, imports preselect the routed format)
- [x] 2.3 Add RU/EN i18n keys for the auto label and the per-format option names to `src/i18n/ru.ts` and `src/i18n/en.ts`
- [x] 2.4 Route the URL `text/*` branch through `detectFormat` in `src/lib/importFlow.ts` and cover it with a test (markdown served as text/plain imports as markdown)

## 3. Verification

- [x] 3.1 `pnpm test:unit` green (detector matrix + updated addFlow/importFlow suites)
- [x] 3.2 `pnpm typecheck` and `just lint` clean (knip verified the removed `toEntryFormat` has no stale references)
- [x] 3.3 `nix develop -c pnpm dlx @fission-ai/openspec validate add-preview-format-autodetect --strict` passes
