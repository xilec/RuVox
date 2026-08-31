# Proposal: wire-code-block-mode

## Why

The `code_block_mode` UI config field is accepted, persisted, patchable via
`update_config`, snapshotted on every generation, and displayed in the params
dialog — but the normalization pipeline never reads it: `TTSPipeline::new()`
unconditionally constructs the code block handler in `Full` mode, so the
setting silently does nothing (#89). Users who want code blocks narrated
briefly have no working way to get that.

Two adjacent dead ends are fixed in the same pass: the inline directive
`<!-- ruvox-code: full|brief -->` (the only working mechanism today) is too
obscure to be a real user feature, and `read_operators` is an orphaned schema
field (present since the initial config schema, never requested, never wired)
that lies in every generation snapshot by claiming to be "in effect".

## What Changes

- **Wire the setting into the pipeline.** The pipeline's code block mode is
  driven by `UIConfig.code_block_mode`: set once at app startup and pushed
  live when `update_config` changes it. Synthesis and preview share the same
  pipeline instance, so both pick up the mode without an app restart
  (synthesis already in flight when the setting changes finishes on the old
  mode).
- **Rename the config value `"skip"` → `"brief"`** and make it the default for
  fresh configs. Semantics: the existing `CodeBlockMode::Brief` — a fenced
  block is replaced with "далее следует пример кода на <язык>" (or "далее
  следует блок кода" without a language tag). `"read"` keeps the current
  `Full` behavior. A persisted legacy `"skip"` is accepted as an alias for
  `"brief"`; any other unknown value falls back to `"brief"`.
- **No config migration.** Existing configs with a persisted `"read"` keep
  full reading — identical to today's actual behavior (always `Full`), so no
  existing user experiences a change until they opt into `"brief"`.
- **BREAKING: remove the inline directive** `<!-- ruvox-code: full|brief -->`.
  The code block mode becomes a single global setting; per-document pragmas
  and their parsing, tests, golden fixture, README section, and explainer
  copy go away. The mode is owned by the config, not the text.
- **Add a Settings control.** The Settings dialog gains a segmented control
  «Читать полностью» / «Кратко» for code block narration, submitted via the
  existing `UIConfigPatch` flow. Without it, fresh installs (default
  `"brief"`) would have no way back to full reading.
- **Remove `read_operators`** from `UIConfig`, `UIConfigPatch`, the
  generation snapshot, IPC types, and their specs. Old configs and old
  snapshots carrying the field keep parsing (serde ignores unknown fields).
- Align `TTSPipeline::new()`'s implicit default to `Brief` so it cannot
  contradict the product default; production code always sets the mode
  explicitly from config.
- CHANGELOG `[Unreleased]` entry (user-visible behavior change). Closes #89.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `text-pipeline`: the "Fenced code blocks" requirement is rewritten — the
  handler mode comes from config (`brief` default, `read` optional) instead of
  hardcoded `Full`; the mode-switch directive requirement and its scenario are
  removed.
- `storage`: `UIConfig.code_block_mode` default flips to `"brief"` with the
  value set `"read" | "brief"` (+ legacy `"skip"` alias); `read_operators` is
  removed from `UIConfig` and from the `GenerationParams` snapshot shape.
- `ipc-commands`: the `UIConfigPatch` shape comment updates to the new value
  set; `update_config` gains the live pipeline-mode push; the generation
  snapshot requirement drops `read_operators` and documents that
  `code_block_mode` now reflects the mode actually applied.
- `ui`: the Settings dialog form requirement gains the code block narration
  segmented control; the params dialog display follows the renamed value.
- `preview-dialog`: the normalization explainer popover copy follows the new
  mode semantics (setting-driven brief/read instead of always-full narration
  and the removed directive).

## Impact

- **Rust backend:** `src-tauri/src/pipeline/normalizers/code_blocks.rs`
  (directive parsing removed, `CodeBlockHandler` simplification),
  `src-tauri/src/pipeline/mod.rs` (mode setter on `TTSPipeline`, default
  flip), `src-tauri/src/lib.rs` (startup wiring), `src-tauri/src/commands/mod.rs`
  (`update_config` push, snapshot recording), `src-tauri/src/storage/schema.rs`
  + `service.rs` (field changes, `read_operators` removal).
- **Frontend:** `src/dialogs/Settings.tsx` (new control + form field),
  `src/lib/tauri.ts` (types), `src/dialogs/GenerationParamsDialog.tsx`
  (display), `src/i18n/ru.ts` / `en.ts` (labels, explainer copy rewording),
  Settings/params dialog tests.
- **Fixtures/docs:** golden fixture `markdown_code_block_duplicates` (built
  entirely on directives) is replaced by mode-driven fixtures; README.md
  «Нормализация» bullet rewritten + README.en.md regenerated.
- **Compatibility:** old configs and snapshots parse unchanged (no
  `deny_unknown_fields`); persisted `"read"` behaves as before; persisted
  legacy `"skip"` becomes live as `brief`.
- No engine, protocol, or storage-format dependencies touched.

## Non-goals

- A "silent" code block mode (block dropped with no marker sentence) —
  `brief` keeps the spoken marker.
- Any config migration or coercion of persisted values beyond the `"skip"`
  alias at parse time.
- Reintroducing per-document/per-entry overrides of the mode; the setting is
  global.
- A replacement for `read_operators` (e.g. a "quiet operators" reading mode) —
  the coarse boolean is removed, not redesigned.
