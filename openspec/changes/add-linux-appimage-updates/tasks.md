# Tasks: add-linux-appimage-updates

## 1. Backend capability command

- [ ] 1.1 Add `updater_supported` to `src-tauri/src/commands/mod.rs`: pure predicate
      `fn updater_supported_with(appimage_env: Option<&OsStr>) -> bool` (Windows → true,
      Linux → env present, else false) plus the `#[tauri::command]` wrapper reading
      `std::env::var_os("APPIMAGE")`; unit tests for all three branches; register the
      command in `lib.rs` `invoke_handler`. Verify: `cargo test` green.

## 2. Frontend gating

- [ ] 2.1 `src/lib/tauri.ts`: add `updaterSupported(): Promise<boolean>` invoke wrapper.
      `src/lib/updater.ts`: replace the `UPDATER_ENABLED` userAgent const with an async
      `updaterSupported()` gate used by `checkForUpdatesOnStartup`; update the module
      doc comment (Linux AppImage is now served). Verify: `pnpm typecheck`.
- [ ] 2.2 Rework `src/lib/updater.test.ts`: mock `commands.updaterSupported`; startup
      no-op when unsupported; startup proceeds and prompts when supported; existing
      manual/install-flow tests keep passing through the gate.
- [ ] 2.3 `src/dialogs/Settings.tsx`: replace `UPDATER_ENABLED &&` with state resolved
      from `updaterSupported()` (dialog-open effect; invoke failure → hidden). Verify:
      `pnpm test:unit` green.

## 3. Release workflow

- [ ] 3.1 `release.yml` Linux job: run `pnpm tauri build` without signing env; after
      `fix-appimage-wayland.sh`, remove any stale `.sig` and re-sign the final AppImage
      via `pnpm tauri signer sign` with `TAURI_SIGNING_PRIVATE_KEY[_PASSWORD]`; verify
      the `.sig` exists and fail loudly otherwise.
- [ ] 3.2 `release.yml` Windows job: stop generating/attaching `latest.json`; upload
      the NSIS `.exe.sig` as a workflow artifact. Linux job: upload the final
      `.AppImage.sig` as a workflow artifact.
- [ ] 3.3 New `updater-manifest` job (`needs: [windows-installer, linux-packages]`,
      short `timeout-minutes`): download both sig artifacts, write `latest.json` with
      `windows-x86_64` and `linux-x86_64` entries (version/notes/pub_date as before,
      URLs pointing at the release-download assets), attach it to the draft release;
      update the stale "manifest stays Windows-only" comments (workflow header + Linux
      job).

## 4. Docs & spec sync

- [ ] 4.1 `docs/install.md`: note that the AppImage self-updates in-app (check in
      Settings) while .deb/nix installs are updated by their package manager. Verify:
      rendered section reads correctly.

## 5. Gates & manual pass

- [ ] 5.1 Full gates: `nix develop -c just lint && nix develop -c just test` green.
- [ ] 5.2 Manual pass (checklist for the user): run the app from a locally built
      AppImage — Settings shows «Проверить обновления» and the check runs against the
      endpoint; run via `pnpm tauri dev`/nix — the section is hidden, no errors in the
      log. E2E self-update is verified at the next published release (old AppImage →
      new version round trip).
