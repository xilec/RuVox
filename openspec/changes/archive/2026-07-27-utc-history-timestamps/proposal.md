# Proposal: Store history timestamps in UTC (#91)

## Summary

`created_at` and `audio_generated_at` are generated from the local clock
(`Local::now().naive_local()`), while the storage schema documents naive
timestamps that readers treat as UTC. The two only agree in a fixed
timezone; after a timezone change entries interleave incorrectly, and any
consumer parsing `history.json` as UTC shows times shifted by the local
offset.

Generate both fields with `Utc::now().naive_utc()` instead.

## Capabilities

- `storage` (modified)

## Non-goals

- No migration of existing `history.json` data: timestamps written so far
  are local-naive. For a single-user desktop app in a fixed timezone the
  stakes are ordering-only; the cutover is documented here instead of
  migrating (a one-time local→UTC rewrite of old entries is possible but
  not worth the risk of misinterpreting mixed data).
- No change to the wire format (still naive, no timezone suffix).

## Approach

Two-line change (`src-tauri/src/storage/service.rs`,
`src-tauri/src/commands/mod.rs`) plus a spec delta; existing entries stay
as they are.
