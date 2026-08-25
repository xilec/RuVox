# Proposal: raise-speed-limit

## Why

Users narrating dense technical texts want faster-than-2.0x playback for skim
listening, but the player caps speed at 2.0x (UI input, frontend clamp, and
backend validation all enforce `[0.5, 2.0]`). The cap is arbitrary from the
user's perspective; mpv and the pitch-preserving `scaletempo2` filter handle
higher multipliers fine.

## What Changes

- Raise the playback speed upper limit from **2.0** to **3.0** (lower bound
  stays 0.5):
  - backend `set_speed` validation range `[0.5, 3.0]`;
  - frontend clamp in Player and the NumberInput max/tooltip;
  - config-restore clamp applied at startup.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `ipc-commands`: the `set_speed` requirement changes its validated range
  from `[0.5, 2.0]` to `[0.5, 3.0]` (out-of-range rejection scenario updated
  accordingly).

## Impact

- `src-tauri/src/commands/mod.rs` (`set_speed` range check + doc comment).
- `src/components/Player.tsx` (clamps in `handleSpeedChange` and the
  startup restore effect, NumberInput `max`, tooltip label).
- No storage/schema change: `speech_rate` is an unconstrained number.
- Existing configs with values in `[2.0, 3.0]` (impossible before) are
  unaffected; nothing migrates.

## Non-goals

- No change to the lower bound (0.5) or step size (0.1).
- No per-entry or per-engine speed overrides.
- No UI redesign of the speed control.
