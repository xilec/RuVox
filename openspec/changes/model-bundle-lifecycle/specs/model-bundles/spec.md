# Delta: model-bundles

## ADDED Requirements

### Requirement: Model asset inventory

The system SHALL expose one inventory for every downloadable model asset:
Silero Native, TeraTTSv2, and every Piper voice (installed or missing). The
inventory SHALL be a table with one row per model family: the framework/bundle
and its actions in the first column, the voices in the second, and the disk
size in the third. Bundled Silero/TeraTTS voices SHALL be listed as part of the
bundle; Piper voices SHALL live in an expandable list with per-voice download
and delete actions. Each entry SHALL report whether it is installed, its
installed bytes, whether it is active, and whether it can be deleted.
Installed bundles SHALL report their bundle version, model id, and upstream
revision when the bundle metadata provides them. The inventory SHALL report
total disk usage across all assets.

#### Scenario: Installed TeraTTS bundle reports its version

- GIVEN a TeraTTS bundle whose manifest has `bundle_version` 2 and
  `upstream_revision` `abc123`
- WHEN the user opens the model inventory
- THEN the TeraTTS entry is installed, reports version `2` and revision
  `abc123`, and includes the bundle's recursive size

#### Scenario: Inventory opens in its own dialog

- GIVEN the user is in Settings
- WHEN they open the model inventory
- THEN Settings shows only a compact entry point above the engine selector and
  the inventory opens in a separate dialog, keeping the main Settings form short
- AND no divider separates the engine selector from the voice selectors

#### Scenario: Table rows explain per-family behavior

- GIVEN installed bundles and Piper voices
- WHEN the inventory opens
- THEN Silero Native and TeraTTSv2 each occupy one row with version, model,
  update, actions, and an enumeration of their bundled voices
- AND every engine row shows a smooth 20-step approximate quality bar under
  its name, without a numeric value, with a tooltip that the bar combines
  UTMOS/DNSMOS and bandwidth and is only a rough guide
- AND the bundle rows explain channel updates, while the active asset is
  marked with an icon whose tooltip explains the active-update/delete
  restriction
- AND the Piper row explains that its voices have no versions and are
  refreshed by deleting and downloading again

#### Scenario: Piper voices appear as an expandable list

- GIVEN Piper voices `ruslan` and `denis`, with `denis` installed
- WHEN the user expands the Piper row
- THEN both voices appear with their sizes, `denis` offers delete, `ruslan`
  offers download, and the active voice is marked as not deletable
- AND the expanded list does not repeat the "Piper voice" label and has no
  separator from the Piper description row

### Requirement: Channel discovery and offline safety

The system SHALL read a per-family `<asset>.json` channel file containing
`format_version`, `asset`, `bundle_version`, `manifest_url`,
`manifest_sha256`, and `size_bytes`. It SHALL check channels on startup at
most once per 24 hours and on explicit user request. A missing, malformed, or
unreachable channel SHALL leave installed assets usable, SHALL be reported as
an inventory-level channel error, and SHALL NOT produce an error toast.

#### Scenario: Offline startup keeps the installed bundle usable

- GIVEN an installed Silero Native bundle and no network
- WHEN the app starts and checks the channel
- THEN the Silero Native entry stays installed and selectable, and the
  inventory reports a channel check error without a toast

#### Scenario: Manual check bypasses the throttle

- GIVEN the channel was checked less than 24 hours ago
- WHEN the user presses "Check for updates"
- THEN the app fetches the channel again and shows a visible result:
  "No updates", "Updates available: N", or a check-failure message

### Requirement: Atomic update with verification

The system SHALL download a bundle update into a staging directory, resume
partial files, verify the channel's manifest sha256, verify every file against
the manifest, and only then swap it into place. The previous bundle SHALL
remain usable until the swap succeeds. A verification or swap failure SHALL
restore the previous bundle and SHALL leave the installed version unchanged.
The active engine's bundle SHALL NOT be updated or deleted; the command SHALL
return a coded `bundle.in_use` error.

#### Scenario: Corrupt update leaves the old bundle working

- GIVEN an installed bundle and a channel update whose manifest hash or file
  hash does not match
- WHEN the user starts the update
- THEN the download fails with a coded error, the previous bundle remains in
  place, and the engine still loads the old version

#### Scenario: Successful update swaps atomically

- GIVEN an installed bundle and a valid newer channel
- WHEN the update finishes
- THEN the new bundle directory is in place, `install.json` records the new
  channel version, and the previous directory is removed

#### Scenario: Active bundle is protected

- GIVEN Silero Native is the active engine
- WHEN the user tries to delete or update its bundle
- THEN the command fails with `bundle.in_use` and the bundle is untouched

### Requirement: Deletion and skip-version

The system SHALL delete a downloaded asset after confirmation and report the
bytes reclaimed. Deletion SHALL be refused for the active engine's bundle or
the active Piper voice. The user SHALL be able to skip a specific available
version; a skipped version SHALL remain installed, SHALL be marked deferred,
and SHALL NOT be advertised as an available update by automatic checks. The
manual "Update" action SHALL remain available for a deferred version, and an
explicit channel check SHALL NOT clear the deferral.

#### Scenario: Deleting an inactive bundle reclaims space

- GIVEN an installed TeraTTS bundle and Piper is active
- WHEN the user deletes the TeraTTS bundle
- THEN the bundle directory is removed, the inventory no longer lists it as
  installed, and total disk usage drops by its size

#### Scenario: Deferred version remains installable

- GIVEN a channel version newer than the installed TeraTTS bundle
- WHEN the user defers that version
- THEN the inventory marks it deferred and hides the update badge
- AND the manual "Update" action remains available and installs it

### Requirement: Version in generation snapshot

The generation-params snapshot SHALL include the installed bundle version,
model id, and upstream revision when available, in addition to the existing
model name and checksum. Older history entries without these fields SHALL
continue to parse.

#### Scenario: New synthesis records bundle provenance

- GIVEN the TeraTTS bundle version 2 is active
- WHEN a synthesis completes
- THEN its generation snapshot carries version `2`, the model id, and the
  upstream revision
