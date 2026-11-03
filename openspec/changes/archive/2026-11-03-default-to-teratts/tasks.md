# Tasks — TeraTTSv2 as the default engine

## 1. OpenSpec

- [x] 1.1 Proposal, design (with the resolved open questions) and the `ui`
      "first-run model bundle prompt" delta (rename + generalization).
- [x] 1.2 `storage` "Config File Schema" delta: `engine` defaults to
      `"teratts"`, persisted engine is never migrated.
- [x] 1.3 `teratts-engine` licensing delta: TeraTTSv2 is the default and the
      README license section stays accurate.

## 2. Default engine

- [x] 2.1 `UIConfig::default_engine()` → `"teratts"` with the schema doc
      comment updated (`storage/schema.rs`, `lib.rs`).
- [x] 2.2 Rust tests: `config_default`, missing-engine-keys defaults to
      `"teratts"`, and a persisted `"silero_native"` is kept on load/save.

## 3. Generalize the first-run prompt

- [x] 3.1 `bundlePromptEngine(config, availability)` — one decision table:
      `teratts` / `silero_native` prompt while their bundle is missing;
      `piper`, `silero` and unknown values never do.
- [x] 3.2 `src/dialogs/SileroBundlePrompt.tsx` → `BundlePrompt.tsx`: the copy,
      download command and activation call come from one per-engine table;
      progress, error/retry and decline stay shared.
- [x] 3.3 TeraTTSv2 copy: «Движок по умолчанию — TeraTTSv2», reason + cost
      (~410 MB, slower first synthesis, ~3.9 GiB peak), «Скачать (~410 МБ)» /
      «Остаться на Piper», one line pointing at Silero Native (~230 MB) in
      Settings.
- [x] 3.4 Silero Native variant keeps ~230 MB and Silero activation for
      existing users; events are filtered by the prompted engine.
- [x] 3.5 `AppShell` stores the prompted engine and renders the prompt from it.

## 4. Settings

- [x] 4.1 Coercion logic unchanged (unavailable engine → Piper, `engine`
      omitted from the patch); factory-reset default engine → `"teratts"`.
- [x] 4.2 Engine selector copy follows the new default: Silero Native is the
      faster/lighter alternative (no "recommended"), the description names
      TeraTTSv2 as the default; the Silero bundle stays in «Модели и голоса».

## 5. i18n and docs

- [x] 5.1 RU/EN catalogs for the per-engine prompt, the alternative line, the
      ready/error notifications and the Settings engine hint.
- [x] 5.2 README / README.en: TeraTTSv2 is the default (no "experimental"),
      Silero Native faster/lighter, Piper zero-download fallback; the licence
      attribution stays.
- [x] 5.3 `AGENTS.md` project overview, `ai/rules/conventions.md` engine
      summary, CHANGELOG `[Unreleased]`.

## 6. Tests

- [x] 6.1 Prompt decision table (teratts missing → prompt; teratts present →
      no; silero_native missing → still prompts; piper/silero/unknown → no).
- [x] 6.2 Settings coercion with the new default and the coerced patch
      omitting `engine`.
- [x] 6.3 Rust default / no-migration tests.
- [x] 6.4 AppShell test: a fresh config opens the TeraTTSv2 prompt; declining
      persists nothing and it returns on the next launch; a successful
      download activates without a restart; another engine's events are
      ignored.

## 7. Validation

- [x] 7.1 `cargo test --manifest-path src-tauri/Cargo.toml`,
      `pnpm typecheck`, `pnpm test:unit`, `just lint`.
- [x] 7.2 `openspec validate default-to-teratts --strict`, archive the change,
      `openspec validate --specs --strict` (27/27).
