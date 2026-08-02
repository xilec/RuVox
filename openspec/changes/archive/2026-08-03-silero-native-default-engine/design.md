# Design: silero-native-default-engine

## Context

Config defaults live in `src-tauri/src/storage/schema.rs` (`UIConfig::default_*`
functions, applied via `#[serde(default)]` — so they affect only absent keys,
never values already persisted in a user's `config.json`). The Settings dialog
(`src/dialogs/Settings.tsx`) mirrors those defaults in its `useForm`
`initialValues` and labels the Piper option «(по умолчанию, без Python)».

Startup engine selection (`build_engine` in `src-tauri/src/lib.rs`) already
probes the Silero Native bundle and silently serves Piper for the run when the
bundle is missing, leaving the on-disk `engine` value untouched — so a fresh
install without the bundle degrades gracefully and the Settings dialog offers
the download. The frontend coercion (`computeEngineFormState` in
`src/lib/engineSelection.ts`) likewise falls back to the first available
engine (Piper) when the saved one is unavailable.

## Goals / Non-Goals

**Goals:**

- Fresh installs (or configs missing the keys) default to Silero Native with
  speaker `aidar` at 24000 Hz.
- Every place that names the default engine/speaker/rate (code comments, UI
  label, rules doc, specs) tells the same story.

**Non-Goals:**

- Migrating existing configs (they carry explicit values; nothing to do).
- Auto-downloading the bundle on first run.
- Changing the fallback order or the download UX.

## Decisions

- **Change the serde defaults, not the resolution logic.** The fallback chain
  (config value → probe → Piper) already handles an unavailable default
  engine; touching only the `default_*` functions keeps the diff minimal.
- **`sample_rate` default follows the default engine.** The field is shared
  across engines, but Piper ignores it (output rate is fixed by the voice
  model — `piper/engine.rs` comments that mpv handles any mismatch) and ttsd
  Silero supports 24000 natively, so defaulting the shared field to 24000 is
  safe for all three engines. The Settings dialog's "follow 24000 when picking
  Silero (нативный) unless the user touched the rate" logic stays — it now
  merely coincides with the global default.
- **`speaker` default becomes `aidar`.** The field is shared between ttsd
  Silero and Silero Native; `aidar` is valid for both.

## Risks / Trade-offs

- A fresh install without the bundle starts on Piper (silent fallback), so the
  "default engine" is effectively "Silero Native once the bundle is present".
  This is the intended graceful degradation, already covered by existing
  behavior and specs.
- Existing users see no change — by design (serde defaults only fill absent
  keys).
