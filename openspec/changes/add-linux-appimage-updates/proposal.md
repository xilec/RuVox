# Proposal: add-linux-appimage-updates

## Why

Linux builds ship as AppImage and .deb (#218), but the in-app updater stays
hard-gated off on Linux: the frontend disables it via a userAgent check and
the release workflow writes only a `windows-x86_64` entry into `latest.json`.
Linux AppImage users therefore have no in-app update path — they must manually
download every new release (#226).

## What Changes

- Backend: new `updater_supported` Tauri command — `true` on Windows, on Linux
  only when running from an AppImage (`APPIMAGE` env var set, the format
  tauri-plugin-updater can self-update), `false` elsewhere. This replaces
  platform sniffing in the webview.
- Frontend: `updater.ts` gates the startup check and the Settings
  «Проверить обновления» section on `updater_supported()` instead of
  `navigator.userAgent`, so an AppImage install gets the same
  check → prompt → download → relaunch flow as Windows.
- Release workflow (`release.yml`):
  - the Linux job signs the **final** AppImage via `pnpm tauri signer sign`
    — signing must happen after `fix-appimage-wayland.sh` repacks the image,
    because the repack invalidates any build-time signature;
  - both build jobs upload their updater signatures as workflow artifacts;
  - a new `updater-manifest` job merges the `windows-x86_64` and
    `linux-x86_64` entries into `latest.json` and attaches it to the draft
    release (manifest generation moves out of the Windows job, which can no
    longer own the file alone).
- Docs: `docs/install.md` notes that the AppImage self-updates in-app while
  .deb/nix installs are updated by their package manager.

No new dependencies: tauri-plugin-updater and the signing key/endpoint are
already in place from the Windows auto-update work (#187/#189/#207).

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `auto-update`: the update check and signed installation requirements are
  extended from Windows-only to Windows + Linux AppImage installs; installs
  the updater cannot serve (Linux .deb/nix) get no update UI and no check,
  with no error surfaced.

## Impact

- `src-tauri/src/commands/mod.rs` (+ registration in `src-tauri/src/lib.rs`),
  `src/lib/tauri.ts`, `src/lib/updater.ts` (+ tests), `src/dialogs/Settings.tsx`.
- `.github/workflows/release.yml` — Linux job, Windows job (drops latest.json
  ownership), new `updater-manifest` job.
- `docs/install.md`, `openspec/specs/auto-update/spec.md`.
- Acceptance limit: the full self-update round trip (old AppImage → new
  release) is only provable after the first release published with the new
  manifest; until then the check resolves to "up to date" (published
  releases lack the `linux-x86_64` entry at or above the running version).
