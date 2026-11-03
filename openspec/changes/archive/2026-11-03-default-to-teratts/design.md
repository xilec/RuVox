# Design — TeraTTSv2 as the default engine

## Context

`UIConfig::default_engine()` returns `"silero_native"`; a fresh config (or one
written before the `engine` field existed) starts on Silero Native. The
first-run prompt (`src/lib/bundlePrompt.ts::shouldOfferBundleDownload` +
`src/dialogs/SileroBundlePrompt.tsx`) is hard-wired to that engine: it opens
only when `engine === "silero_native"`, calls
`commands.downloadSileroNativeBundle()` and activates Silero Native through
`update_config`.

TeraTTSv2 (`teratts`) already ships a downloadable bundle
(`download_teratts_bundle`, `bundle_download_*` events with `engine:
"teratts"`, `update_config({ engine: "teratts" })`) and is selectable in
Settings, but nothing offers its bundle on first run. A fresh config moved to
`"teratts"` without a generalized prompt would silently stay on Piper and make
Settings the only install path — hence the default and the prompt change
together.

Settings coerces an unavailable saved engine to Piper and omits `engine` from
the saved patch, so a coerced save cannot overwrite the saved (or default)
engine before its bundle exists. The first-run prompt is what completes the
story after the coercion.

## Goals / Non-Goals

**Goals:**

- A fresh install starts on TeraTTSv2 once its bundle is installed, and is
  offered that bundle on first launch with an honest reason and price.
- Existing configs keep their persisted engine: no silent switch, no
  migration.
- One parameterized prompt and one decision function instead of two
  engine-specific code paths.
- The shared mechanics stay identical: `bundle_download_*` progress, inline
  retry, per-run decline, activation without a restart.

**Non-Goals:**

- Migration of existing configs, per-engine first-run flows, a download
  picker.
- Changing the bundle format, the download commands, `sample_rate` semantics
  or either engine's quality.

## Decisions

**Default engine: `"teratts"` in `UIConfig::default_engine()`.** The field is
serde-defaulted, so only fresh configs and configs without the key adopt it; a
config that already has `"engine": "silero_native"` (or any other value) parses
to that value. Rejected: a migration that rewrites persisted configs — it would
silently switch users who chose an engine deliberately, and the issue's
acceptance criteria forbid it.

**One decision function returning the engine.** `shouldOfferBundleDownload`
becomes `bundlePromptEngine(config, availability): "teratts" |
"silero_native" | null`. The prompt now needs to know *which* bundle to offer
(command, copy, activation), and a boolean plus a second lookup would let the
two halves drift. The table is explicit: `teratts`/`silero_native` prompt when
`availability[engine].available` is false; `piper`, `silero` (needs the Python
stack, not a bundle) and unknown values never prompt. Rejected: keeping the
boolean and deriving the engine from `config.engine` at the call site — an
unchecked cast for a value the decision function already validated.

**Engine table in the dialog, not `if (engine === ...)` branches.** The
`BundlePrompt` holds a `Record<BundlePromptEngine, { title, body, download
label, alternative?, ready title/message, download: () => Promise<void> }>`
with typed `MessageKey`s and the command call. Adding a third bundle engine is
one table entry plus catalog strings; the progress/error/activation code path
stays shared. Rejected: passing every string as a prop from `AppShell` — it
would move the copy and the command choice into the shell for no benefit.

**Activation is `update_config({ engine })` for both engines.** The persisted
engine already equals the prompted one in both cases, but the current session
may be running the Piper fallback; `update_config` is what rebuilds the
`EngineSwitcher` onto the downloaded engine without a restart. On failure the
prompt reuses the existing "bundle downloaded, engine not started" notification
and does not claim activation.

**Events are filtered by the prompted engine.** `bundle_download_started` /
`_progress` / `_finished` carry `engine`, and Settings can start its own
download concurrently. Ignoring events for another engine prevents a Settings
download of one bundle from closing the first-run prompt or activating the
wrong engine.

**Declining persists nothing.** «Остаться на Piper» only closes the modal;
the config is not patched, so next launch re-evaluates the same condition and
the prompt returns. No "don't ask again" state in this revision.

**Silero Native prompt copy stays, wording adjusted to "saved engine".** With
TeraTTSv2 as the default, the Silero prompt can no longer claim to be "the
default engine"; it addresses the users who already have `silero_native`
persisted. Behavior (size, command, activation) is unchanged.

**Settings factory defaults follow the engine default.** `SETTINGS_DEFAULTS`
(the target of «Сброс») changes `engine: "teratts"`, matching the storage
default; the loaded-config baseline still drives dirty tracking. Rejected:
leaving `"silero_native"` in the reset constant — reset would contradict the
storage default and the "TeraTTSv2 is the default" copy.

**Labels: Silero Native drops "(recommended)".** With TeraTTSv2 the default,
the Silero Native option is the faster, lighter alternative, not the
recommendation; the engine selector label and description are updated
accordingly. The Models dialog still offers its ~230 MB bundle unchanged.

## Open questions (resolved)

- **Migrate existing configs?** No. Only fresh configs and configs without
  `engine` get `"teratts"`; a persisted engine is kept as-is.
- **Keep the "experimental" label?** No. TeraTTSv2 is the default, so the
  README/CHANGELOG drop the label.
- **Second download button in the prompt?** No. The prompt mentions
  «Быстрее и легче: Silero Native (~230 МБ) — в Настройках» and nothing more;
  the Models dialog remains the place to install Silero Native.

## Risks / Trade-offs

- The default now costs a larger one-time download (~410 MB), a slower first
  synthesis and more memory (~3.9 GiB peak with parallel lanes). This is the
  deliberate trade recorded in the prompt copy: quality of the first
  impression over synthesis speed.
- A user who declines is served by Piper for the session and sees the prompt
  again next launch; that is the desired per-run semantics, but it means the
  prompt cannot be dismissed permanently in this revision.
- The default engine string is written by the backend, so any future default
  change is a schema-comment/test/README edit rather than a migration.
