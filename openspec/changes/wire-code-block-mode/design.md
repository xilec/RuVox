# Design: wire-code-block-mode

## Context

See proposal.md for the motivation. The facts that shape the design:

- `TTSPipeline` is a process-lifetime singleton (`Arc<Mutex<TTSPipeline>>`
  in `AppState`, `lib.rs`), shared by synthesis (`run_normalization`) and
  preview (`preview_normalization`), both funneling through
  `run_pipeline_normalization` (`commands/mod.rs`). One wiring point covers
  both consumers.
- `CodeBlockHandler` already has `with_mode` / `set_mode` / `mode()` and both
  modes are unit-tested; the missing piece is plumbing from config to the
  handler (the `code_block_handler` field on `TTSPipeline` is private).
- Config round-trip plumbing (schema, patch, snapshot, params dialog display)
  exists and is tested; the Settings dialog has no control for the field, and
  `read_operators` is an unwired schema orphan.
- Storage serde types do not use `deny_unknown_fields`, so dropping a field
  from `UIConfig` / `GenerationParams` keeps old `config.json` / `history.json`
  parsing (unknown keys ignored).

## Goals / Non-Goals

**Goals:**

- Single source of truth for the code block narration mode: `UIConfig`, read
  at startup and pushed live on `update_config`.
- One home for the string ↔ mode knowledge (legacy `"skip"` alias, unknown
  fallback) shared by storage sanitization and the pipeline.
- Directive removal without behavior change for generic (non-directive) HTML
  comments.

**Non-Goals:** (see proposal Non-goals) — additionally: no markdown-aware
stripping of HTML comments (out of scope; generic comments already pass
through as ordinary text today and keep doing so), no per-entry mode
overrides, no redesign of operator reading.

## Decisions

### D1: Mode plumbing — setter on `TTSPipeline`, not a constructor parameter

Add `TTSPipeline::set_code_block_mode(CodeBlockMode)` and
`TTSPipeline::code_block_mode() -> CodeBlockMode`, delegating to the existing
`CodeBlockHandler` methods. The pipeline is a long-lived singleton mutated at
two moments (startup, `update_config`); a setter expresses both with one API.
A `with_code_block_mode` constructor would force reconstructing the pipeline
(or duplicating construction) for live updates.
`TTSPipeline::new()`'s implicit default flips to `Brief` to match the product
default; production code sets the mode explicitly from config immediately
after construction, so the implicit default is a test-only convenience that
can no longer contradict the config.

### D2: One mapping home — `CodeBlockMode` owns string conversion

`CodeBlockMode` gains `from_config(&str) -> CodeBlockMode`
(`"brief"` | `"skip"` → Brief, `"read"` → Full, anything else → Brief) and
`as_config_str() -> &'static str` (`"brief"` / `"read"` — never `"skip"`).
Storage sanitizes on load and on patch by round-tripping through the enum
(`from_config(raw).as_config_str()`), so `get_config` always returns a
canonical value and the Settings control binds to a clean enum. Rejected:
sanitizing only at the pipeline (leaves raw `"skip"`/garbage visible to the
UI and persisted forever) and a storage-side string table (two homes for one
rule — they drift).

### D3: Wiring points — startup + `update_config` push

- Startup (`lib.rs` setup, where the pipeline is constructed): read
  `storage.load_config()`, `pipeline.set_code_block_mode(from_config(…))`.
- `update_config` (`commands/mod.rs`): after the config is persisted, if the
  patch carried `code_block_mode`, push the new mode into
  `state.pipeline` (brief `Mutex` lock, no `await` held across it — the
  command is synchronous with respect to the pipeline mutex).

Rejected: reading the config inside `run_pipeline_normalization` per request
(threads `storage` into a pure-pipeline helper, adds a file read per
synthesis/preview, and races with in-flight synthesis over what "current"
means). The chosen push model makes the pipeline mode authoritative and
lock-consistent with normalization.

### D4: Snapshot records the mode captured at normalization time

The synthesis task captures `pipeline.code_block_mode().as_config_str()`
right after `run_normalization` returns and threads it into the generation
snapshot. This is honest under the `update_config` race: a synthesis that
normalized under Brief but finished after the user switched to Read records
`"brief"`. Fallback if the plumbing through `synthesize_audio` proves
disproportionate during implementation: read the config at snapshot time and
note the sub-second race in a comment — but prefer the capture, the data
already flows through task locals.

### D5: Directive removal — plain removal, no comment stripping

`CodeBlockHandler` drops `collect_directives` and the per-block mode
switching; `process` scans fenced blocks only. Generic HTML comments were
never special and keep flowing to the symbol phases as ordinary text (status
quo); a pasted `<!-- ruvox-code: … -->` now behaves like any other comment.
Rejected: stripping HTML comments in markdown processing — real behavior
addition beyond the removal, needs its own spec case, and can follow later if
pasted comments annoy in practice. The exact narration of a legacy directive
comment is pinned by a golden fixture.

### D6: Golden fixtures — optional mode sidecar

Fixture pairs gain an optional `<case>.mode.txt` sidecar containing
`"brief"` or `"read"`; absent sidecar = default pipeline (Brief). The
directive-driven `markdown_code_block_duplicates` fixture is replaced by two
mode-driven pairs (`markdown_code_block_brief`, `markdown_code_block_full`)
plus the legacy-directive case from the spec scenario. Rejected: running the
whole suite twice per mode (doubles runtime, most fixtures are
mode-independent) and a filename convention like `<case>@read` (uglier in
diffs, breaks the existing naming).

### D7: Frontend — SegmentedControl in the Settings form

`SettingsFormValues` + `buildSettingsPatch` gain `code_block_mode`;
a Mantine `SegmentedControl` («Кратко» / «Читать полностью») binds to it,
labeled via new `settings.code_block.*` i18n keys. The params dialog's
`displayCodeBlockMode` switches to `'read' | 'brief'` with a
`generation.code_block.brief` label; the `generation.read_operators` row and
keys are deleted. `tauri.ts` drops `read_operators` from both interfaces.
The explainer copy (`preview.explain.details`) is reworded to name the
setting instead of the directive.

## Risks / Trade-offs

- [In-flight synthesis uses the previous mode after a settings change] →
  accepted and specified (`ipc-commands` delta: MAY finish on the previous
  mode); normalization lock makes the switch point exact.
- [Persisted legacy `"skip"` changes audible behavior (Full → Brief)] →
  intended: the user asked for skip and got Full because the setting was
  dead; no UI ever wrote `"skip"` (the editor did not exist), so only
  hand-edited configs are affected.
- [Configs missing `code_block_mode` flip to Brief] → the serde default
  change is the product default change; the field has existed since the
  initial schema, so virtually all persisted configs carry an explicit value.
- [Fixture churn could mask regressions in code reading] → the replacement
  pairs reuse the same block content in both modes, and the full-mode sidecar
  keeps `Full` coverage explicit rather than implicit-in-default.
- [Dangling directive references after removal] → grep `ruvox-code` must
  return only the changelog/archives after the change; tasks include the
  sweep.

## Migration Plan

No data migration: tolerant serde covers old configs and snapshots
(`read_operators` ignored and dropped on next save; `"skip"` aliased on
load). Rollback = revert the merge; persisted values written by the new
build (`"brief"`) are understood by the old build's serde as an opaque
string — the old build ignores the mode entirely (hardcoded Full), so a
rollback restores pre-change narration without parse errors.

## Open Questions

None.
