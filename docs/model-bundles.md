# Model bundles and channels

Silero Native and TeraTTSv2 ship their weights as GitHub Release assets. The
app installs them on demand, checks for updates through a small channel file,
and lets the user delete an inactive bundle or a Piper voice from Settings.

## Channel file

Each family has one JSON document under [`model-channels/`](../model-channels/),
named after the asset id: `silero_native.json` and `teratts.json`. The app
requests `<channel-base>/<asset>.json`, so a hyphenated `silero-native.json`
is never found:

```json
{
  "format_version": 1,
  "asset": "silero_native",
  "bundle_version": "v5_ru",
  "manifest_url": "https://github.com/xilec/RuVox/releases/download/silero-models-v5_ru/manifest.json",
  "manifest_sha256": "<sha256 of manifest.json>",
  "size_bytes": 234989985,
  "published_at": "2026-10-30T00:00:00Z"
}
```

`bundle_version` is a string: TeraTTS uses its numeric bundle version, Silero
uses its release tag. The manifest hash is checked before the manifest is
parsed, so a truncated or swapped manifest cannot half-install a bundle.

The app fetches channels from
`https://raw.githubusercontent.com/xilec/RuVox/main/model-channels` and caches
the result for 24 hours. `RUVOX_MODEL_CHANNEL_BASE` overrides the base URL for
local testing. A missing or unreachable channel is offline-safe: installed
bundles keep working and Settings shows a non-blocking note.

## Release procedure

1. Build and verify the bundle locally (see `silero-native/export/README.md`
   or `teratts/export/README.md`).
2. Publish the release assets under a new tag (`silero-models-vN`,
   `teratts-models-vN`). Upload `manifest.json` and every file listed in it.
3. Compute `sha256sum manifest.json` and the total size of the listed files.
4. Update the family's channel file:
   - `bundle_version` — the new version string;
   - `manifest_url` — the new release's manifest URL;
   - `manifest_sha256` — the hash from step 3;
   - `size_bytes` — the total size from step 3;
   - `published_at` — the publication time in UTC.
5. Open a PR with the channel change. The app picks it up after the next
   startup check or when the user presses "Проверить обновления".

The active engine's bundle is never updated or deleted in place; the user
switches to another engine first. This keeps Windows file locks and a running
engine's directory stable.

## Local update test

Point the app at a local HTTP server that serves the two channel files and the
release manifest/files:

```bash
RUVOX_MODEL_CHANNEL_BASE=http://127.0.0.1:8000/channels \
  nix develop -c pnpm tauri dev
```

Serve the channel files from `channels/` and the release assets from the URLs
named in `manifest_url`; the manifest hash and every file hash still have to
match. The install is staged under `<bundle>.staging` next to the bundle and
swapped only after verification.
