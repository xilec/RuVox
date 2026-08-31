## 1. Pipeline: mode API and directive removal

- [x] 1.1 Add `CodeBlockMode::from_config(&str)` (`"brief"` / legacy `"skip"` → Brief, `"read"` → Full, unknown → Brief) and `CodeBlockMode::as_config_str()` (`"brief"` / `"read"`, never `"skip"`) with unit tests for all four input classes; verify with `cargo test --manifest-path src-tauri/Cargo.toml code_blocks`
- [x] 1.2 Add `TTSPipeline::set_code_block_mode` / `TTSPipeline::code_block_mode` delegating to the handler; flip `TTSPipeline::new()` implicit default to Brief; update pipeline unit tests that assumed Full reading to construct with an explicit mode; verify with `cargo test --manifest-path src-tauri/Cargo.toml pipeline`
- [x] 1.3 Remove the `ruvox-code` directive machinery (`collect_directives`, per-block mode switching) and the `mode_switch_via_process` test; add a unit test pinning that a legacy directive comment no longer changes the mode and is normalized as ordinary text; verify with `cargo test --manifest-path src-tauri/Cargo.toml`
- [x] 1.4 Teach the golden harness an optional `<case>.mode.txt` sidecar (`"brief"` / `"read"`; absent = default Brief pipeline); replace `markdown_code_block_duplicates` with `markdown_code_block_brief`, `markdown_code_block_full` (same block content in both modes), and a legacy-directive fixture; verify with `cargo test --manifest-path src-tauri/Cargo.toml --test golden`

## 2. Storage schema

- [x] 2.1 In `schema.rs`: default `code_block_mode` → `"brief"`; sanitize the loaded value and `update_config` patch values through the `CodeBlockMode` enum round-trip (design D2); remove `read_operators` from `UIConfig`, `UIConfigPatch`, and defaults; extend schema unit tests with the serde scenarios (missing key → `"brief"`, persisted `"read"` stays `"read"`, `"skip"` aliased, unknown → `"brief"`, config with `read_operators` parses and drops it); verify with `cargo test --manifest-path src-tauri/Cargo.toml schema`
- [x] 2.2 Remove `read_operators` from `GenerationParams` and storage test fixtures; add a test that a legacy snapshot carrying `read_operators` parses, keeps its other fields, and drops it on re-save; verify with `cargo test --manifest-path src-tauri/Cargo.toml storage`

## 3. Config-to-pipeline wiring

- [x] 3.1 Wire startup: after `TTSPipeline::new()` in `lib.rs` setup, set the mode from `storage.load_config()` (via a small testable helper); verify with a unit test constructing the pipeline + helper with a `"skip"`/`"read"` config
- [x] 3.2 Wire `update_config`: after persisting a patch that carries `code_block_mode`, push the new mode into the shared pipeline; verify with an orchestration test — `update_config({ code_block_mode: "read" })` then `run_normalization` reads a fenced block in full (spec: Configuration Commands scenario)
- [x] 3.3 Capture the applied mode right after `run_normalization` in the synthesis task and thread it into the generation snapshot (design D4); remove the `read_operators` snapshot recording; update orchestration snapshot asserts (mode recorded, `read_operators` gone); verify with `cargo test --manifest-path src-tauri/Cargo.toml commands`

## 4. Frontend

- [x] 4.1 Update `src/lib/tauri.ts`: drop `read_operators` from both interfaces, comment `code_block_mode` as `"brief" | "read"`; verify with `pnpm typecheck`
- [x] 4.2 Add the code block narration field to the Settings form (`SettingsFormValues`, `buildSettingsPatch`, SegmentedControl «Кратко» / «Читать полностью», `settings.code_block.*` i18n keys ru+en); verify with `pnpm test:unit` Settings dialog tests asserting the control renders from config and lands in the patch
- [x] 4.3 Update `GenerationParamsDialog`: `displayCodeBlockMode` maps `'read' | 'brief'`, add `generation.code_block.brief` labels, remove the `read_operators` row and its i18n keys, update dialog tests and the `QueueList.test.tsx` fixture; verify with `pnpm test:unit`
- [x] 4.4 Reword the normalization explainer copy (`preview.explain.details`, ru+en) to describe the Settings modes instead of the directive (spec: preview-dialog delta); verify with `pnpm test:unit` and by reading both catalogs

## 5. Docs and changelog

- [x] 5.1 Rewrite the `README.md` normalization bullet: the Settings control replaces the directive description; regenerate `README.en.md` from it; verify both files mention the setting and no `ruvox-code`
- [x] 5.2 Add the `CHANGELOG.md` `[Unreleased]` entry (user-visible: code block narration setting live, brief default, directive removed); verify the entry matches the conventions format

## 6. Validation

- [x] 6.1 Run `nix develop -c just test` — all Rust (incl. golden fixtures), TS, and Python suites green
- [x] 6.2 Run `nix develop -c just lint` — fmt, clippy, deny, eslint, knip, typecheck, ruff clean
- [x] 6.3 Run `nix develop -c pnpm dlx @fission-ai/openspec validate wire-code-block-mode --strict` — change artifacts valid
- [x] 6.4 Sweep `grep -rn "ruvox-code"` outside `openspec/changes/archive/` and `CHANGELOG.md` — only deliberate references remain (the legacy-directive pinning test + fixture, the unrelated `--ruvox-code-bg` CSS token; main specs sync at archive)
- [ ] 6.5 Manual pass: start the app (`nix develop -c pnpm tauri dev`), open Settings, confirm the «Кратко» / «Читать полностью» control reflects the saved config; switch it, save, add a markdown text with a fenced code block via the preview dialog and confirm the normalized pane follows the new mode without a restart
