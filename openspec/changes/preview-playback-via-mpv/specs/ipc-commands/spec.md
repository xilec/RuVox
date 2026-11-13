## ADDED Requirements

### Requirement: Preview Playback Commands

The system SHALL provide `preview_play(wav_base64)` and `preview_stop()` for the
dictionary editor's pronunciation preview, beside the synthesis contract of
"Pronunciation Preview Command".

`preview_play` SHALL decode the WAV and write it to a unique file under the
application cache preview directory, then start playback in a dedicated
audio-only mpv instance whose plugin window label is `preview`. The command
SHALL return after playback has started and SHALL NOT load anything into the
narration player or emit `playback_*` events. A malformed base64 payload SHALL
be rejected with `preview.invalid_audio`, a decoded payload larger than 8 MiB
with `preview.audio_too_large`, and a player failure with `preview.play_failed`
carrying the underlying message. Starting a new preview while one plays SHALL
first destroy the previous instance and delete its file.

`preview_stop` SHALL stop and destroy the preview instance and delete the
temporary WAV. It SHALL succeed when no preview is playing or the instance has
already exited. A failure to destroy the instance SHALL be reported as
`preview.stop_failed` and SHALL NOT block the file cleanup.

The preview instance SHALL be destroyed on application exit. The startup sweep
of stale preview files SHALL remain the backstop for a crash that leaves a WAV
behind.

#### Scenario: play starts a dedicated player

- GIVEN the frontend invokes `preview_play` with a valid WAV
- WHEN the command returns
- THEN the preview mpv instance is playing the WAV and the narration player has
  received no command

#### Scenario: stop destroys the player and deletes the file

- GIVEN `preview_play` started a preview
- WHEN the frontend invokes `preview_stop`
- THEN the preview instance is destroyed, its temporary WAV is deleted, and no
  playback event is emitted

#### Scenario: stop is idempotent

- GIVEN no preview is playing
- WHEN the frontend invokes `preview_stop`
- THEN the command succeeds

#### Scenario: malformed payload is rejected

- GIVEN `preview_play` receives a string that is not valid base64
- WHEN the command runs
- THEN it returns `preview.invalid_audio` and no player instance is created

#### Scenario: oversized payload is rejected

- GIVEN `preview_play` receives a WAV larger than 8 MiB
- WHEN the command runs
- THEN it returns `preview.audio_too_large` before writing any file

#### Scenario: replacing a preview destroys the previous instance

- GIVEN a preview is playing
- WHEN the frontend invokes `preview_play` again
- THEN the previous instance is destroyed and its file deleted before the new
  playback starts
