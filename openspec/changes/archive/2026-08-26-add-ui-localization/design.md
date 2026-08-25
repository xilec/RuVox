# Design: add-ui-localization

## Context

~120 hardcoded strings across ~10 frontend modules; 61 `CommandError` sites
in `commands/mod.rs` sharing ~10 Russian message patterns. The repo already
uses zustand for stores and has no i18n dependency.

## Goals / Non-Goals

**Goals:**

- Zero-dependency localization (catalogs + `t()`), reactive in React,
  callable from plain modules.
- One wire-format change for errors, applied atomically with the frontend
  translation so no toast ever shows a raw code.

**Non-Goals:**

- Pluralization rules beyond simple param interpolation (RU plurals avoided
  by phrasing); engine-internal diagnostics stay untranslated fallbacks.

## Decisions

- **Hand-rolled layer, not i18next.** Two languages, ~150 keys, one consumer
  app: a typed catalog pair (`src/i18n/ru.ts`, `src/i18n/en.ts`) + a 30-line
  helper beats a runtime dependency. Keys are typed: `en.ts` must satisfy
  `Record<keyof typeof ru, string>` (compile-time missing-key check).
- **Locale store mirrors config, not localStorage.** `src/stores/locale.ts`
  (zustand) is seeded once at App start from `getConfig().language`; the
  Settings selector sets the store immediately (instant relabel) and the
  dialog's Save persists it. Single source of truth stays `config.json`.
- **Errors: `code` + `params`, `message` demoted to optional fallback.**
  Each of the 61 sites gets a dotted site id (`image.fetch_failed`). Params
  are positional strings interpolated into `{0}`-placeholders. Raw detail
  (engine/HTTP text) rides in `message` when useful for diagnosis.
  `formatError(err)` becomes the single localization point: catalog lookup →
  `message` → per-`type` generic.
- **Highlight theme via Vite `?inline` CSS injection.** Import both hljs
  themes as inline strings and mount exactly one `<style>` conditioned on
  Mantine's computed color scheme — avoids double-applying global `.hljs`
  rules that a dual static import would cause.

## Risks / Trade-offs

- Wire-format break: any code path reading `err.message` must migrate in the
  same change; tests assert the new shape, CI enforces both sides compile.
- Catalog drift (key added to RU only) is caught by the TS type on `en.ts`.
- 61 error-site edits are mechanical but wide; clippy `-D warnings` plus
  updated Rust tests pin the constructor signatures.
