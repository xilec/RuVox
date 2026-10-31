# Proposal: versioned model bundles

## Why

Model bundles are pinned to a release tag in code and can only be installed
once. TeraTTSv2 weights change upstream, and the app already accumulates
TeraTTSv2, Silero Native, and Piper voice assets with no installed-version
view, update path, or deletion. Users cannot see or reclaim hundreds of
megabytes, and a model update needs a new app release.

## What Changes

- Add a small per-family `channel.json` with the latest bundle version,
  manifest URL, manifest sha256, and download size. The app checks it on
  startup (throttled to once per 24 h) and on demand; a missing channel or
  offline network leaves the installed bundle usable without an error toast.
- Add an atomic install/update path: download into a staging directory with
  resume, verify the channel manifest hash and every file, then swap the
  bundle directory; keep the previous bundle until the new one verifies and
  roll back on failure.
- Add a unified model inventory for Silero Native, TeraTTSv2, and Piper
  voices, opened from a compact Settings row in its own dialog: installed
  version, model id, upstream revision, size, update availability,
  update/delete actions, skip-version, and total disk usage.
- Guard deletion (and updates) of the active engine's bundle; report a
  coded reason and ask the user to switch engines first.
- Record bundle version, model id, and upstream revision in the
  generation-params snapshot alongside the existing model name/checksum.
- Document the maintainer release procedure for publishing a bundle and its
  channel entry.

## Out of Scope

- Hosting changes or a new CDN.
- Automatic background downloads without user consent.
- Model quality changes.
- A scheduled upstream-change detector workflow (documented as a manual
  maintainer step).
