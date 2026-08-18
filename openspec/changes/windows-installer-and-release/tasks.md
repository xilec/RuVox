# Tasks: windows-installer-and-release

Depends on: `fix-windows-portability` (archived or at least implemented).

## 1. Third-party resource acquisition spike

- [ ] 1.1 Spike: build `src-tauri` once and locate the compiled `espeak-ng-data/` directory in the espeak-rs-sys build tree; verify it contains `ru_dict`, `phondata`, `intonations`; document the extraction path (or record the fallback to the official espeak-ng release)
- [ ] 1.2 Pick and pin versions + sha256 for the mpv (shinchiro) and onnxruntime (Microsoft) downloads; record them in the workflow or a `scripts/` manifest

## 2. Bundle configuration

- [ ] 2.1 `tauri.conf.json`: `bundle.targets: ["nsis"]`, `webviewInstallMode: embedBootstrapper`, `bundle.resources` for `mpv/`, `espeak-ng-data/`, `onnxruntime.dll`
- [ ] 2.2 CI download/extract steps place resources under `src-tauri/resources/` before `pnpm tauri build`; resources are gitignored

## 3. Auto-update

- [ ] 3.1 Add `tauri-plugin-updater` (Rust) + `@tauri-apps/plugin-updater` (JS); register the plugin; `createUpdaterArtifacts: true`
- [ ] 3.2 Generate the updater keypair (`pnpm tauri signer generate`); pubkey into `tauri.conf.json`, private key + password into GitHub Secrets
- [ ] 3.3 Frontend: check-for-updates on app start + manual check in Settings; Russian notification with update-and-restart action; silent failure when offline

## 4. Release workflow

- [ ] 4.1 `.github/workflows/release.yml`: `on.push.tags: v*` + `workflow_dispatch`, `windows-latest`, pnpm install, resource downloads (D2), `pnpm tauri build`
- [ ] 4.2 Apply the D5 watch items as needed (cmake policy var, LIBCLANG_PATH, sonic)
- [ ] 4.3 Publish a **draft** release via softprops/action-gh-release with the NSIS installer + updater artifacts (`*.nsis.zip`, `*.sig`)
- [ ] 4.4 PR-triggered build-only Windows job (no release) guarding Windows-relevant changes

## 5. Verification

- [ ] 5.1 Local quickemu VMs: Windows 10 22H2 and Windows 11 Enterprise Evaluation
- [ ] 5.2 Manual checklist in the VM: install (incl. WebView2 bootstrap on a machine without it) → launch → Piper voice download → synthesize → bundled mpv playback → tray → update check → uninstall
- [ ] 5.3 Tag a release, confirm the draft release appears with all artifacts, publish after approval

## 6. Gates

- [ ] 6.1 `nix develop -c just lint` and `nix develop -c just test` green on Linux
- [ ] 6.2 `pnpm dlx @fission-ai/openspec validate --strict` passes for this change
