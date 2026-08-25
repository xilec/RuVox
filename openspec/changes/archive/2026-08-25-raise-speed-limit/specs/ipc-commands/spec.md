# Delta: ipc-commands

## MODIFIED Requirements

### Requirement: Playback Parameter Commands

The system SHALL provide `set_speed(speed)` and `set_volume(volume)` with
inclusive range validation: `speed` in `[0.5, 3.0]`, `volume` in `[0.0, 1.0]`.
Out-of-range values SHALL be rejected with `config_error` (not clamped).
`set_speed` SHALL persist the value to `UIConfig.speech_rate`; `set_volume`
SHALL NOT persist anything. Pitch-correct speed scaling uses mpv's
`scaletempo2` audio filter.

#### Scenario: valid speed is applied and persisted

- **GIVEN** playback is active
- **WHEN** `set_speed` is invoked with `2.7`
- **THEN** mpv speed is set to 2.7 and `speech_rate: 2.7` is written to the config

#### Scenario: out-of-range values are rejected

- **GIVEN** any playback state
- **WHEN** `set_speed` is invoked with `3.5` or `set_volume` with `1.2`
- **THEN** the command fails with `type: "config_error"` naming the allowed range
