# Tasks: optional-tray-on-close

## 1. Config field

- [x] Add `hide_to_tray_on_close: bool` to `UIConfig` with
      `#[serde(default)]` (off for fresh and pre-existing configs) and `false`
      to `Default`.
- [x] Add `hide_to_tray_on_close: Option<bool>` to `UIConfigPatch` and apply it
      in `apply_config_patch`; extend the exhaustive patch-field table and the
      all-`None` touch test.
- [x] Schema tests: default is off; a config without the key parses to `false`.

## 2. Close behavior

- [x] Add `should_hide_to_tray` in `src-tauri/src/lib.rs`, reading the persisted
      config through `AppState.storage` on every close (read errors → `false`).
- [x] Gate `CloseRequested` (prevent + hide) and `ExitRequested`
      (prevent exit) on the helper, keeping the `user_quit` bypass.
- [x] Unit-test the gate: default off, and on after persisting the enabled flag.
- [x] Hide into the tray only while a tray icon exists, so a failed icon
      creation still lets the window close quit the app; unit-test the
      decision matrix.

## 3. Tray icon visibility

- [x] Add `tray::set_enabled` creating or removing the `"main"` tray icon.
- [x] Gate startup tray creation on the persisted `hide_to_tray_on_close`.
- [x] Sync the icon from `update_config` after saving, so the Settings switch
      adds/removes it without a restart.
- [x] Drop the removed icon on the main thread (the GTK/macOS tray backends
      are main-thread-only).
- [x] Register the tray-menu handlers once for the app lifetime: Tauri never
      prunes menu-event listeners, so per-icon registration would multiply
      the menu actions.
- [x] Roll the setting back when the icon cannot be created, so the stored
      state matches the tray that exists; the frontend re-reads the value.

## 4. Settings UI

- [x] Add `hide_to_tray_on_close` to the `UIConfig` TS type.
- [x] Add the form field, loaded value, submitted patch entry, and a Switch in
      the interface section; factory Reset restores `false`.
- [x] Add RU/EN label and description strings.
- [x] Update the Settings test fixture and add a toggle-submits-patch test.

## 5. Spec + docs

- [x] Apply the `tray`, `ui`, and `ipc-commands` deltas.
- [x] Add a `CHANGELOG.md` `[Unreleased]` note.

## 6. Validation

- [x] `nix develop -c cargo test --manifest-path src-tauri/Cargo.toml`
- [x] `nix develop -c pnpm typecheck`
- [x] `nix develop -c pnpm test:unit`
- [x] `nix develop -c just lint`
- [x] `nix develop -c pnpm dlx @fission-ai/openspec@1.6.0 validate optional-tray-on-close --strict`
- [x] Archive the change (`openspec archive`) before the PR.
