# Tasks — reading width and reading scale

## 1. OpenSpec

- [x] 1.1 Proposal, design, and deltas for `ui`, `storage` and `ipc-commands`.

## 2. Backend

- [x] 2.1 `UIConfig.reading_width: String` (`"narrow"` default, canonicalized
  on load and patch like `code_block_mode`) and `reading_scale: u32` (percent,
  default 100, clamped to 80–200 on load and patch); both in `UIConfigPatch`.
- [x] 2.2 Rust tests: defaults, unknown width → narrow, scale clamp on load and
  through a patch, round-trip.

## 3. Frontend

- [x] 3.1 `ReadingWidth` / `reading_scale` in the IPC types and pure reading
  helpers (ladder, clamp, nearest step, zoom factor) in `src/lib/reading.ts`.
- [x] 3.2 Shared `ReadingScaleControl` and reading-width segments used by the
  viewer toolbar and Settings.
- [x] 3.3 Viewer toolbar right of the display switch, present in every display
  mode; optimistic `update_config` with rollback + toast for both controls.
- [x] 3.4 CSS: narrow column `min(var(--ruvox-reading-measure), 100%)` +
  `margin-inline: auto`, wide unchanged; `zoom` on `.content` through
  `--ruvox-reading-scale`; `pre`/`table` keep `overflow-x: auto`.
- [x] 3.5 Settings «Чтение» block (width + scale) wired into the form and the
  submitted `UIConfigPatch`; factory defaults and reset include both fields.
- [x] 3.6 `TextViewer` receives the persisted values from `AppShell`.
- [x] 3.7 RU/EN labels and accessible names.

## 4. Tests

- [x] 4.1 Rust: storage load/patch canonicalization and round-trip.
- [x] 4.2 TS: helper ladder/clamp/step unit tests.
- [x] 4.3 TextViewer: controls in every mode, ladder+bounds, reset-to-100,
  applied `--ruvox-reading-scale`, width attribute, failed-persist rollback +
  toast, re-sync from props.
- [x] 4.4 Settings: saved values rendered, changed values submitted, untouched
  values preserved.

## 5. Validation

- [x] 5.1 `cargo test --manifest-path src-tauri/Cargo.toml`
- [x] 5.2 `pnpm typecheck`
- [x] 5.3 `pnpm test:unit`
- [x] 5.4 `just lint`
- [x] 5.5 `openspec validate add-reading-typography --strict`, then archive the
  change and sync the specs; `openspec validate --specs --strict`.
- [x] 5.6 CHANGELOG `[Unreleased]` note.
