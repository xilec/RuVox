# IPC Commands — Regeneration Play-When-Ready (delta)

## MODIFIED Requirements

### Requirement: Entry Regeneration Command

The system SHALL provide `regenerate_entry(id, play_when_ready)` which drops
the current audio and timestamps, sets `was_regenerated: true` and
`error_message: null`, emits `entry_updated`, and re-runs background
synthesis with the current config (speaker/voice, sample rate) — including
the synthesis-time input length guard (see "Synthesis-time input length
guard"). When `play_when_ready` is `true` and regeneration succeeds, the
fresh audio SHALL start playing (the same autoplay rule as the initial
synthesis); when it is `false`, the regenerated audio is only cached. If the
entry is playing, playback SHALL be stopped first. Regeneration of an entry
with status `processing` SHALL be rejected with `synthesis_error` to avoid
racing the in-flight task.

#### Scenario: regenerate a ready entry

- GIVEN a `ready` entry and a changed `speaker` in the config
- WHEN `regenerate_entry` is invoked with `play_when_ready: false`
- THEN the old audio is deleted, `entry_updated` is emitted with
  `was_regenerated: true`, and a fresh synthesis advances the entry back to
  `ready`

#### Scenario: regenerate with play-when-ready autoplays

- GIVEN a `ready` entry
- WHEN `regenerate_entry` is invoked with `play_when_ready: true`
- THEN after the fresh audio reaches `ready` it starts playing without
  further user action

#### Scenario: regenerate during synthesis is rejected

- GIVEN an entry with status `processing`
- WHEN `regenerate_entry` is invoked
- THEN the command fails with `type: "synthesis_error"` and the in-flight
  synthesis continues

#### Scenario: regenerate an oversized entry under Piper fails with the limit message

- GIVEN an entry with text longer than 100 000 codepoints that was accepted while Silero was active
- WHEN `regenerate_entry` is invoked with Piper as the active engine
- THEN the re-run synthesis fails the entry with the Russian message naming the limit and the Piper engine
