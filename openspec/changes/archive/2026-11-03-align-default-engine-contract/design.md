## Context

The default engine is recorded in three places that must agree: the Rust
`UIConfig::default_engine()` (`"teratts"`), the Settings factory defaults
(`SETTINGS_DEFAULTS.engine`, mirrored by a cross-reference comment), and the
specs. #359 synced `storage`, `teratts-engine` and the `ui` first-run prompt,
but left `ipc-commands` "Shared IPC Types" and the `ui` Settings reset scenario
describing the pre-#359 contract (`silero_native`).

## Goals / Non-Goals

**Goals:**

- Make the two stale statements match the shipped behavior.
- Keep the delta limited to the two requirements that drifted.

**Non-Goals:**

- Any code, UI or storage change.
- Re-editing the requirements #359 already synced.

## Decisions

**D1. MODIFIED deltas, not direct spec edits.** `openspec/specs/` is synced
only by archiving a change, and #359 is already archived; a follow-up change
with two MODIFIED requirements is the sanctioned way to correct the text.
Rejected: editing the synced specs by hand — the correction would have no
audit record and the next sync would not carry it.

**D2. The `ui` delta carries the whole "Settings dialog" requirement.** A
MODIFIED delta replaces the entire requirement, so the delta reproduces the
current text verbatim and changes only the reset scenario's THEN clause; the
rest of the requirement must stay byte-identical after the archive.

**D3. "Storage default" is the reference, not a second literal.** The reset
scenario names `"teratts"` as the storage default and requires the same
availability coercion as the initial load, so the canonical source stays
`UIConfig::default_engine` / `SETTINGS_DEFAULTS.engine`: a future default
change updates one constant, and the scenario keeps describing behavior rather
than a hardcoded UI fallback.

## Risks / Trade-offs

- The `ui` delta is large relative to the change; a whitespace difference in
  the reproduced requirement would surface as an unrelated diff after the
  archive. Mitigation: the delta is generated from the synced text, and the
  requirement block is diffed before and after the archive.
- If `ipc-commands` is regenerated from the Rust types later, the
  `(default)` comment must stay; the scenario asserts the observable value
  regardless of the comment.
