# Playback Specification

## Purpose

Covers audio playback of synthesized entries: the mpv-backed player on the
Rust side, the playback Tauri commands and events, the player UI controls in
the header (play/pause, position slider, speed, volume), and the
window-scoped keyboard shortcuts.

## Requirements

### Requirement: mpv-backed playback engine

The system SHALL play entry audio through an mpv subprocess controlled via
`tauri-plugin-mpv`, started with `--no-video` and the `scaletempo2` audio
filter (pitch-correct speed scaling). Loading an entry SHALL NOT start
playback implicitly: `load` pauses immediately and an explicit `play` is
required. After the mpv subprocess has been destroyed (e.g. by a
hide-to-tray cycle), subsequent player commands MUST fail gracefully with a
typed error instead of panicking, and `ensure_mpv_alive` SHALL re-initialize
mpv on demand.

#### Scenario: Load then play

- GIVEN an entry with status `ready` and an existing audio file
- WHEN `play_entry` is invoked
- THEN the audio is loaded paused and playback starts only after `play`

#### Scenario: Command after mpv destruction

- GIVEN the mpv instance was destroyed while the window was hidden
- WHEN any playback command is invoked before re-initialization
- THEN the command returns an error and the application does not panic

### Requirement: Playback commands and events

The system SHALL expose the commands `play_entry`, `pause_playback`,
`resume_playback`, `stop_playback`, `seek_to`, `set_speed`, and `set_volume`.
`play_entry` SHALL refuse entries whose status is not `ready`. The backend
SHALL emit `playback_started` (also on resume), `playback_paused` (with
`position_sec`), `playback_stopped`, and `playback_position` events. A
background task SHALL emit `playback_position` every 100 ms while playing,
carrying `position_sec`, `entry_id`, and `duration_sec` when known.

#### Scenario: Play a ready entry

- GIVEN an entry with status `ready`
- WHEN `play_entry(id)` is invoked
- THEN playback starts and a `playback_started` event with the `entry_id`
  and `duration_sec` is emitted

#### Scenario: Play a non-ready entry

- GIVEN an entry with status `processing`
- WHEN `play_entry(id)` is invoked
- THEN the command fails with a "not ready" error and nothing plays

#### Scenario: Position ticks

- GIVEN an entry is playing
- WHEN 100 ms elapse
- THEN a `playback_position` event with the current `position_sec` is emitted

### Requirement: End-of-file detection

The system SHALL treat playback as finished when the polled position reaches
`duration - 0.2s` or when mpv reports no position while a duration is known.
On end-of-file the system SHALL emit `playback_finished` followed by
`playback_stopped` and mark the player as not playing. An unknown duration
MUST NOT trigger end-of-file.

#### Scenario: Track plays to the end

- GIVEN an entry playing with a known duration
- WHEN the position reaches the end of the file
- THEN `playback_finished` and `playback_stopped` are emitted and the UI
  returns to the stopped state

### Requirement: Seek behavior

`seek_to` SHALL seek to an absolute position in seconds. The backend SHALL
immediately emit a `playback_position` event with the seek target and
suppress periodic position ticks for a 300 ms window so a stale pre-seek
position cannot snap the UI back; end-of-file detection MUST still run
during the suppression window.

#### Scenario: Seek forward

- GIVEN an entry playing at position 10s
- WHEN `seek_to(30)` is invoked
- THEN a `playback_position` event with `position_sec = 30` is emitted
  immediately and the next ticks report positions from 30s onward

### Requirement: Speed and volume control

The system SHALL support playback speeds from 0.5x to 2.0x with pitch
correction (`scaletempo2`), and volume from 0 to 100% (mapped internally from
the 0.0-1.0 range to mpv's 0-100). The UI SHALL offer a `NumberInput` for
speed with step 0.1; rotating the mouse wheel over the speed field SHALL
change the speed by 0.1 per step without scrolling the page. Values outside
the allowed range SHALL be clamped.

#### Scenario: Wheel over speed field

- GIVEN the speed is 1.0x
- WHEN the user scrolls up over the speed field
- THEN the speed becomes 1.1x, is applied via `set_speed`, and the page does
  not scroll

#### Scenario: Speed clamping

- GIVEN the speed field
- WHEN the user enters 3.0
- THEN the applied speed is clamped to 2.0x

### Requirement: Player UI controls

The player SHALL sit in the application header and provide: a play/pause
toggle button (disabled when no entry is loaded), a position slider that
shows an optimistic position while dragging and seeks on release, a time
display `MM:SS / MM:SS` (or `H:MM:SS` for hour-long audio), the speed field,
and a volume slider labeled in percent. While the user drags the position
slider, incoming `playback_position` events MUST NOT overwrite the dragged
value.

#### Scenario: Drag the position slider

- GIVEN an entry is playing
- WHEN the user drags the position slider and releases it at 45s
- THEN the slider shows 45s during the drag and `seek_to(45)` is invoked on
  release

### Requirement: Window-scoped hotkeys

The system SHALL support the following hotkeys while the RuVox window is
focused: `Space` toggles play/pause, `ArrowLeft` seeks back 5 seconds
(clamped at 0), `ArrowRight` seeks forward 5 seconds (clamped at the
duration). Global hotkeys are intentionally NOT implemented; the hotkeys
MUST NOT fire while the window is unfocused.

#### Scenario: Space toggles playback

- GIVEN an entry is playing and the window is focused
- WHEN the user presses `Space`
- THEN playback pauses; pressing `Space` again resumes from the same
  position

#### Scenario: Arrow seek clamping

- GIVEN playback is at position 3s
- WHEN the user presses `ArrowLeft`
- THEN the position seeks to 0s, not a negative value
