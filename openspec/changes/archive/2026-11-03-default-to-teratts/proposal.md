## Why

A fresh install defaults to Silero Native (`UIConfig::default_engine =
"silero_native"`) and the first-run prompt is hard-wired to Silero: it opens
only for `engine === "silero_native"`, calls `download_silero_native_bundle`
and activates Silero. But TeraTTSv2 reads Russian prose, English terms and
code natively and sounds cleaner, so it should be the engine a new user meets
first — with its ~410 MB bundle offered on first run. Silero Native stays
available as the faster, lighter alternative (~230 MB), and Piper stays the
zero-download fallback.

Without this change a fresh config would default to an engine whose prompt
never appears: Settings would be the only way to install the bundle, and the
first run would silently stay on Piper.

## What Changes

- `UIConfig::default_engine()` returns `"teratts"`. Only fresh configs and
  configs without an `engine` field adopt the new default; a persisted engine
  is never migrated or rewritten (a user on Silero Native stays on Silero
  Native).
- The Silero-specific first-run decision becomes "which bundle does the
  persisted engine need": `"teratts"` and `"silero_native"` prompt when their
  bundle is missing, `"piper"`, `"silero"` and unknown values never prompt.
- `SileroBundlePrompt` becomes an engine-parameterized `BundlePrompt`: engine
  name, bundle size, `download_*_bundle` command and the activation
  `update_config` call come from one table. The TeraTTSv2 variant names the
  engine as the default, explains the reason and the cost (larger download,
  slower first synthesis, ~3.9 GiB peak memory), offers «Скачать (~410 МБ)» /
  «Остаться на Piper» and mentions Silero Native (~230 MB) as the alternative
  in Settings — without a second download button.
- The Silero Native variant keeps its existing behavior for users who already
  have that engine saved: ~230 MB, activation of Silero Native on success.
- Both variants keep the shared mechanics: progress from the
  `bundle_download_*` events, inline error with retry, per-run decline that
  persists nothing, green confirmation and activation without a restart.
- Settings keeps its coercion logic (unavailable engine → Piper, `engine`
  omitted from the patch) with the new default; its factory-reset baseline and
  engine copy follow the new default. The Silero Native bundle stays
  downloadable from «Модели и голоса» unchanged.
- RU/EN copy, README/README.en, `AGENTS.md`, CHANGELOG and the `ui`/`storage`
  specs are updated; the "experimental" label is dropped from TeraTTSv2 and
  the licence attribution stays in README/NOTICE.

## Non-goals

- Migrating existing configs to the new default, per-engine first-run flows or
  a multi-bundle download picker.
- Removing Silero Native, `ttsd` or Piper; changing either engine's quality,
  `sample_rate` semantics or the bundle format.
- Bundling models into the installer or writing the user guide (#350) — the
  README is kept free of contradictions instead.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `ui`: the first-run prompt requirement is renamed and generalized from the
  Silero Native bundle to the persisted engine's model bundle, with the
  TeraTTSv2 and Silero Native variants and their scenarios.
- `storage`: the `Config File Schema` requirement's default engine changes to
  `"teratts"` and states explicitly that a persisted engine is not migrated.
- `teratts-engine`: the licensing requirement no longer calls Silero Native
  "the default engine"; TeraTTSv2 is the default and stays the commercially
  usable option under OpenRAIL-M, Silero Native stays non-commercial.

## Impact

- Rust: `src-tauri/src/storage/schema.rs` (default + docs + tests).
- Frontend: `src/lib/bundlePrompt.ts`, `src/dialogs/SileroBundlePrompt.tsx` →
  `src/dialogs/BundlePrompt.tsx`, `src/components/AppShell.tsx`,
  `src/dialogs/Settings.tsx` (factory defaults), `src/i18n/{ru,en}.ts`.
- Tests: Rust default/missing-key tests, prompt decision table, Settings
  coercion with the new default, AppShell first-run prompt test.
- Docs: `README.md`, `README.en.md`, `AGENTS.md`, `CHANGELOG.md`.
