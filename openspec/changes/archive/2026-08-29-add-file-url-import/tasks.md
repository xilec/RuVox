## 1. Backend: file reading

- [x] 1.1 Add `encoding_rs` + `chardetng` deps to `src-tauri/Cargo.toml`; verify `nix develop -c cargo check --manifest-path src-tauri/Cargo.toml`
- [x] 1.2 Implement encoding detection helper over the supported-encoding list using `encoding_rs`+`chardetng` (BOM sniff → UTF-8 validity → chardetng guess → decode to UTF-8) with unit tests incl. CP1251/KOI8-R/CP866 fixtures; verify `cargo test -p ruvox-tauri encoding`
- [x] 1.3 Implement `read_text_file(path, encoding?)` command: extension allowlist, 10 MiB cap, decode, return `{text, encoding}`; coded errors (`import.unsupported_extension`, `import.too_large`, `import.decode_failed`); verify command unit tests + `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings`

## 2. Backend: URL fetching

- [x] 2.1 Implement `fetch_url_text(url)` reusing the shared HTTP client (scheme allowlist http/https, 10 MiB cap, 5 s connect / 30 s total); body decoded by the same import helper; return text + encoding + content type; coded errors (`import.fetch_failed` with `{status}` param, `import.too_large`); verify unit tests with a mocked server
- [x] 2.2 Register both commands in the invoke handler; update `src/lib/tauri.ts` wrappers + typed errors; verify `pnpm typecheck`

## 3. Frontend: import decision layer

- [x] 3.1 Pure `resolveImport(source)` in `src/lib/importFlow.ts`: extension→format mapping for files, URL classification via content type + detection fallback, SPA heuristic (extracted-text threshold + script-dominated markup checks as named constants); full unit-test matrix (`importFlow.test.ts`) covering every spec scenario; verify `pnpm test:unit`

## 4. Frontend: UI entry points

- [x] 4.1 Split-button «Добавить» in AppShell navbar: primary click unchanged, menu «Файл…» / «Файл с кодировкой…» / «По ссылке…»; file picker via Tauri dialog plugin (add capability entry), URL input modal; i18n keys added to `src/i18n/{ru,en}.ts`; verify `pnpm test:unit` + manual click-through
  - Deviation: file picker implemented on plain `rfd` (`pick_import_file` command) per the #223 precedent — no dialog plugin, no capability change; recorded in design.md.
- [x] 4.2 Encoding dialog component: raw decoded preview + encoding Select preselected with detected value; confirm re-decodes and continues to the preview dialog; cancel aborts; verify component test
- [x] 4.3 Drag & drop: subscribe `getCurrentWebview().onDragDropEvent()` in AppShell; full-window overlay during drag-over; single supported file/link starts the import flow; unsupported drops ignored silently; verify manual on Linux (KDE Wayland) per spec scenarios
  - Verified manually by the user on host (niri): D&D works.
- [x] 4.4 Wire all entry points through the preview gate: enabled → PreviewDialog prefilled (file text or fetched markup), disabled → direct ingestion via the existing executor; failures before text exists → error notifications, no dialog; verify unit tests around the gating glue + manual pass

## 5. Gates & docs

- [x] 5.1 Full gates green: `nix develop -c just test && just lint`; knip clean (no unused exports)
  - Note: on the dev machine (NixOS) the `uv run ruff check` step cannot exec
    uv's manylinux ruff binary (stub-ld); ttsd sources verified clean with a
    nixpkgs ruff build instead. CI runs the pinned step normally.
- [x] 5.2 Manual-test task: fresh app — drop `.txt` (UTF-8), `.txt` (CP1251), `.md`, `.html`, an SPA URL, a static-article URL, a 404 URL; check overlay behavior, split-button actions, encoding-dialog correction flow, preview-gate on/off paths, RU/EN notifications
  - Covered by the VM auto-run (article/SPA/404/empty/KOI8, gate on/off, RU/EN)
    + user's manual pass (D&D, file flows, legacy-encoded files KOI8-R/CP866/
    ISO-8859-5 in ~ of the VM — all confirmed working).
