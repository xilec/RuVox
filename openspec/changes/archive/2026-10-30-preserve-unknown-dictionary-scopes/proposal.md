## Why

Version 2 collections are keyed by an engine id. A newer build may add an
engine this build does not know; silently dropping that collection on the next
save would lose the user's data.

## What Changes

- Preserve a version 2 collection whose `scope` is unknown as a raw TOML table.
- Write preserved collections back unchanged on the next save; they do not
  participate in lookup in the older build.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `user-dictionary`: unknown-scope collections survive a load/save round-trip.

## Impact

- `src-tauri/src/dictionary/{mod,store}.rs`; no UI change.
