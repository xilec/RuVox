# Design: versioned model bundles

## Channel format

One JSON file per bundle family, published under `model-channels/` and served
from the repository's raw URL:

```json
{
  "format_version": 1,
  "asset": "silero_native",
  "bundle_version": "v5_ru",
  "manifest_url": "https://github.com/xilec/RuVox/releases/download/.../manifest.json",
  "manifest_sha256": "<hex>",
  "size_bytes": 234989985,
  "published_at": "2026-10-30T00:00:00Z"
}
```

`bundle_version` is a string: TeraTTS uses its numeric `bundle_version`, while
Silero uses its release tag. The manifest hash is verified before the manifest
is parsed, so a truncated or swapped manifest cannot half-install a bundle.

The default base URL is
`https://raw.githubusercontent.com/xilec/RuVox/main/model-channels`; the
`RUVOX_MODEL_CHANNEL_BASE` environment variable overrides it for tests and
local manual runs. A fetch failure is cached as an offline result for the
throttle window and is not surfaced as a toast.

## Installed state and versions

TeraTTS versions come from its manifest (`bundle_version`, `model_version`,
`upstream_revision`). Silero's manifest predates versioning, so a successful
install writes `install.json` next to `manifest.json` with the channel version,
manifest sha256, size, and install time. Existing Silero installs without that
file report the manifest `model_id` as their version; this keeps them usable
and makes a channel version equal to the model id a no-op.

Skip-version choices live in `model_state.json` in the config root, not in the
main config, so a config reset does not silently re-enable a skipped update.

## Atomic install

The existing shared downloader already resumes `.partial` files and verifies
each file. The lifecycle layer adds a deterministic staging directory next to
the bundle (`<asset>.staging`), downloads there, verifies the manifest hash,
and writes `install.json` in staging. The swap is:

1. rename `<asset>` to `<asset>.previous` (if present);
2. rename `<asset>.staging` to `<asset>`;
3. on failure, restore `.previous`;
4. on success, remove `.previous`.

The active engine's bundle is never swapped or deleted: the command returns
`bundle.in_use`, and the UI tells the user to switch engines. This avoids
Windows file locks and keeps the running engine on a valid directory.

Piper voices use the same inventory and delete path, but keep their existing
per-voice downloader (Hugging Face URLs, no channel/manifest version).

## Inventory

`get_model_status` returns one list for all three families. Each entry carries
`id`, `kind`, `installed`, `active`, `installed_version`, `model_id`,
`upstream_revision`, `installed_bytes`, `latest_version`, `latest_bytes`,
`update_available`, `skipped`, `deletable`, and `error`. The response also
carries `total_bytes` and `channel_error`. Sizes are recursive directory sizes
computed on demand; the list is small (two bundles plus four voices).

## Snapshot

`ModelParams` gains optional `bundle_version`, `model_id`, and
`upstream_revision`. They are serde-defaulted so existing history entries keep
parsing. Engines fill them from their manifest/install metadata; Piper leaves
them absent.
