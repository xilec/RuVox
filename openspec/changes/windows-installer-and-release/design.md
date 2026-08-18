# Design: windows-installer-and-release

## Context

See proposal.md — Why. Depends on `fix-windows-portability` (the code
already resolves the bundled `mpv/` and `espeak-ng-data/` paths and runs
without ttsd). Existing precedent: `export-silero-bundle.yml` publishes a
release via softprops/action-gh-release. Local verification of the
installer uses quickemu VMs (Win10 22H2 + Win11 Enterprise Evaluation) —
the maintainer's machine is NixOS-only.

## Goals / Non-Goals

**Goals:**

- Tag `v*` → draft GitHub release with the NSIS installer and updater
  artifacts.
- Reproducible CI downloads of third-party binaries (pinned URL +
  sha256).
- PR-triggered build-only Windows job guarding the Windows build.

**Non-Goals:**

- Code signing (#183), release-notes tooling (#184), MSI, ARM64.
- Matrix builds for other OSes.

## Decisions

### D1: Release flow — manual workflow over tauri-action

A hand-written `release.yml` (`on.push.tags: v*` + `workflow_dispatch`)
running `pnpm tauri build`, then softprops/action-gh-release with
`draft: true`. Chosen over the official tauri-action: consistent with the
existing silero-bundle workflow, full control over the asset list, and
draft releases fit the repo's approve-then-publish rule.

### D2: Third-party binaries at CI time, pinned

- **mpv**: shinchiro `mpv-x86_64-*` 7z release — pinned tag, sha256 of
  the archive recorded in the workflow (or a `scripts/` manifest),
  extracted into `src-tauri/resources/mpv/` before the build; mpv LICENSE
  included (GPL, compatible with our GPL-3.0).
- **onnxruntime.dll**: official `microsoft/onnxruntime` release
  (`onnxruntime-win-x64-<ver>.zip`), pinned + sha256, DLL placed next to
  the exe via bundle resources.
- **espeak-ng-data**: extracted from the espeak-rs-sys build tree
  (`target/.../espeak-ng-data/`) after `cargo build` — same version as
  the linked library, no extra download. Fallback if the data turns out
  incomplete: extract from the official espeak-ng Windows release.

*Alternative considered:* committing binaries to the repo. Rejected —
tens of MB in git, undiffable; pinned downloads are reproducible enough.

### D3: tauri.conf.json bundle wiring

`bundle.targets: ["nsis"]` (from `"all"` — MSI explicitly out),
`bundle.windows.webviewInstallMode: embedBootstrapper`,
`bundle.resources` entries for `mpv/`, `espeak-ng-data/`, and
`onnxruntime.dll`, `bundle.createUpdaterArtifacts: true` and the updater
pubkey under `plugins.updater`. `bundle.windows.wix` left untouched.

### D4: Updater wiring

`tauri-plugin-updater` (Rust + `@tauri-apps/plugin-updater`), keypair
generated once via `pnpm tauri signer generate` during implementation;
private key + password in GitHub Secrets (`TAURI_SIGNING_PRIVATE_KEY`,
`TAURI_SIGNING_PRIVATE_KEY_PASSWORD`), pubkey in `tauri.conf.json`.
Frontend: check on app start + a manual check in Settings; Russian
notification with a restart-to-update action. Endpoints point at the
GitHub releases `latest` JSON.

### D5: Watch items from the xwin investigation, applied to CI

- If the runner's cmake is 4.x, set `CMAKE_POLICY_VERSION_MINIMUM=3.5`
  for the vendored opus build.
- Verify bindgen finds the runner's preinstalled LLVM (`LIBCLANG_PATH`
  if needed).
- espeak-ng may git-clone `sonic` at configure time — if flaky, pre-seed
  or disable per its CMake options.

## Risks / Trade-offs

- [SmartScreen blocks unsigned installer for early users] → accepted for
  0.x; tracked in #183; updater signatures are independent of code
  signing.
- [espeak-ng-data extraction from OUT_DIR is fragile] → spike it first
  (task 2.1); fallback to the official espeak-ng release is decided in
  D2.
- [Runner image drift (cmake/LLVM versions) breaks the build
  non-deterministically] → watch items in D5; pin tool versions in the
  workflow where possible.
- [Installer size ~100-150 MB] → accepted; models/voices download at
  runtime instead of inflating the installer.

## Migration Plan

First tag `v0.3.0` (or next per CHANGELOG) produces the first draft
release; published manually after the VM checklist passes. Rollback:
delete the draft release/tag; no user-facing state to migrate.

## Open Questions

None blocking. Exact mpv/onnxruntime versions are pinned at
implementation time and recorded in the workflow.
