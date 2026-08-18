# Proposal: windows-installer-and-release

## Why

After `fix-windows-portability` makes the backend compile and run on
Windows, users still have no way to get RuVox: there is no Windows build,
no installer, and no release pipeline. Epic #185 targets Windows 10 22H2+ /
11 x86_64 (~97% of the RU desktop segment). This change ships the
packaging, the installer, auto-updates, and the tag-driven release job.

## What Changes

- Bundle Windows runtime resources: `mpv/` (mpv.exe + DLLs + LICENSE,
  from pinned shinchiro builds), `onnxruntime.dll` (pinned
  Microsoft/onnxruntime release, sha256-verified), `espeak-ng-data/`
  (preferred source: the data directory already compiled into the
  espeak-rs-sys build tree; fallback: extract from the official espeak-ng
  release).
- Produce an NSIS installer via `cargo tauri build` on a
  `windows-latest` GitHub Actions runner with
  `webviewInstallMode = embedBootstrapper` (the installer delivers the
  WebView2 runtime when missing — required for Win10 machines without
  it). No MSI (no corporate-deployment need).
- Add `tauri-plugin-updater`: the app checks for updates and installs
  them from signed update artifacts attached to GitHub releases; update
  signing keypair is generated during implementation, the private key
  goes to GitHub Secrets, the public key into `tauri.conf.json`.
- Add a release workflow: pushing a tag `v*` builds on `windows-latest`
  and creates a **draft** GitHub release (softprops/action-gh-release,
  same as the existing silero-bundle workflow) carrying the NSIS
  installer plus updater artifacts. A build-only pull_request trigger
  (no release) guards the Windows build on changes to the workflow or
  Windows-relevant code.

## Capabilities

### New Capabilities

- `windows-installer`: the NSIS installer for Windows 10 22H2+ / 11
  x86_64 — WebView2 bootstrap, per-user install, bundled runtime
  resources (mpv, onnxruntime, espeak-ng-data), no ttsd.
- `auto-update`: in-app update checks and installation via
  tauri-plugin-updater from GitHub release artifacts, with signature
  verification.

### Modified Capabilities

(None — the runtime adaptation requirements land with
`fix-windows-portability`.)

## Impact

- **Code:** `src-tauri/tauri.conf.json` (bundle targets, resources,
  NSIS/WebView2 config, updater pubkey), `src-tauri/Cargo.toml` +
  frontend (tauri-plugin-updater wiring, update notification UI),
  `src-tauri/src/main.rs` / `lib.rs` (updater plugin registration).
- **CI:** new `.github/workflows/release.yml` (tag-triggered) and a
  build-only Windows job; download steps pin URLs + sha256 for mpv and
  onnxruntime.
- **Dependencies:** `tauri-plugin-updater` (Rust + JS). Prebuilt
  third-party binaries downloaded at CI time: mpv (GPL — license file
  bundled), onnxruntime (MIT).
- **GitHub:** new Actions secret for the updater private key; draft
  releases on tags.

## Non-goals

- Code signing of the installer (SmartScreen warning accepted for 0.x —
  tracked in #183).
- Release-notes generation tooling (#184).
- Windows ARM64, macOS, Linux packaging changes.
- Shipping ttsd on Windows.
- Running `cargo test` on Windows CI (build-only job; tests stay on
  Linux).
