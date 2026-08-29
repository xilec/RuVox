# Design: add-linux-appimage-updates

## Context

Windows auto-update works end to end: tauri-plugin-updater is registered,
`tauri.conf.json` embeds the minisign pubkey and the
`releases/latest/download/latest.json` endpoint, and the release workflow
signs the NSIS installer and attaches a `windows-x86_64` manifest entry.
On Linux the frontend force-disables the updater
(`UPDATER_ENABLED = navigator.userAgent.includes('Windows')`), and
`release.yml` builds .deb + .AppImage without any updater artifacts.

Two Linux-specific constraints shape the design:

1. **tauri-plugin-updater can self-update only the AppImage format.** It
   locates the running image via the `APPIMAGE` env var (set by the
   AppImage runtime) and replaces that file. A .deb/nix install has no
   `APPIMAGE` env, so the plugin errors — the app must not surface that
   error as a broken feature.
2. **The shipped AppImage is not the file tauri built.** After
   `pnpm tauri build`, `scripts/fix-appimage-wayland.sh` extracts and
   repackages the image (pure-Wayland fix, load-bearing). Any signature
   computed at build time no longer matches the distributed file.

## Goals / Non-Goals

**Goals:**

- AppImage installs get the same update flow as Windows: startup check,
  Settings check, confirm dialog, download+verify, replace, relaunch.
- Non-AppImage installs see no update feature and no errors.
- The published manifest's signature always matches the distributed file.

**Non-Goals:**

- Self-update for .deb/nix installs (not supported by the plugin; those
  formats are updated by their package manager).
- Update notifications without user consent changes, delta downloads,
  staging/rollout channels.
- macOS (no macOS build exists).

## Decisions

- **Capability check as a Tauri command (`updater_supported`) instead of
  webview sniffing.** The AppImage fact only exists in the backend env —
  the webview has no `APPIMAGE` visibility. The command returns `true` on
  Windows, `env::var_os("APPIMAGE").is_some()` on Linux, `false`
  otherwise. Alternative considered: `@tauri-apps/plugin-os` for the
  platform plus a separate env probe — two IPC calls for one fact; and
  the existing userAgent hack is exactly what misclassified Linux.
  The pure predicate is factored so the env lookup is injectable and
  unit-testable.
- **Sign the AppImage after the wayland repack, via `tauri signer sign`.**
  The build step runs without signing env (the pubkey present with no
  private key only warns), then `fix-appimage-wayland.sh` repacks, then
  `pnpm tauri signer sign` (same `TAURI_SIGNING_PRIVATE_KEY[_PASSWORD]`
  env the Windows job already uses) writes the final `.AppImage.sig`.
  Any stale `.sig` from the build is removed before signing. Alternatives:
  signing at build time and shipping a signature of a file we don't
  distribute (update would abort verification — worse); external minisign
  tooling in CI (extra dependency for a flag the CLI already has).
- **A dedicated `updater-manifest` job owns `latest.json`.** Both build
  jobs upload their `.sig` files as workflow artifacts; the final job
  downloads them, writes `latest.json` with `windows-x86_64` and
  `linux-x86_64`, and attaches it to the draft release. Alternatives:
  generating the file in each job (last writer wins / asset conflict on
  the same filename), or serializing the Windows job behind Linux
  (wastes the Windows cache-warm window for no benefit).
- **Reuse the Windows flow untouched on AppImage.** `shutdown_player_for_update`
  before download (harmless on Linux, required on Windows), progress toast,
  confirm modal, `relaunch()` — no platform branches in `updater.ts`
  beyond the capability gate. The plugin's AppImage install path replaces
  the running image and `relaunch` execs it.

## Risks / Trade-offs

- [Repack step drifts and breaks signatures again] → The sign step runs
  strictly after the repack in the same job and fails loudly if the
  `.sig` is missing; `tauri signer sign` is invoked with the same env
  contract as the Windows job.
- [Manifest job races the draft release into existence] → It `needs`
  both build jobs, and `softprops/action-gh-release` with the same tag
  updates the existing draft; whichever job runs first creates it.
- [First real self-update can only be observed post-release] → The old
  published releases have no `linux-x86_64` entry, so the check honestly
  reports "up to date" until the first release **published** with the new
  manifest; the manual pass covers gate/UI behavior locally, and E2E is
  verified at the next release (task checklist).
- [Deb/nix users lose even the manual check] → Deliberate: a visible
  "check failed: unsupported package format" error is worse than a hidden
  no-op; the Settings section disappears only on formats that could never
  self-update.
