# Tasks: fix-export-format-combobox

## 1. Backend

- [x] 1.1 `src-tauri/Cargo.toml`: linux-gated `ashpd` dependency
      (`default-features = false` — the runtime feature must come from
      rfd's declaration; both at once do not compile).
- [x] 1.2 `src-tauri/src/commands/export.rs`: `run_save_dialog` (Linux) —
      direct portal `SaveFileRequest` with `ruvox-<id>.wav` pre-filled and
      the «Формат» choice combo (WAV default, Ogg Opus alternative);
      cancellation maps to `None`, portal failures to the new
      `export.dialog_failed` code; the response's choice value travels back
      with the path. Non-Linux keeps rfd (both filters, extension decides).
      Normalization follows the reported choice, falling back to the stored
      format's extension.

## 2. Tests

- [x] 2.1 Rust: normalization with a reported format (matching kept,
      mismatched/foreign replaced), without one (recognized kept, foreign
      replaced, missing appended), `stored_ext_of` defaults.

## 3. Specs & gates

- [x] 3.1 Delta spec: ipc-commands — portal dialog with the format combo,
      requested-format normalization, `export.dialog_failed`.
- [x] 3.2 Gates green (`cargo test` export cluster, clippy, fmt,
      typecheck, `pnpm test:unit`); `openspec validate --specs --strict`.
