# Delta: ipc-commands

## ADDED Requirements

### Requirement: Synthesis voice follows the active engine

The voice passed to the TTS engine during synthesis SHALL be selected by the
engine **active at synthesis time** (`EngineSwitcher.kind()`), not by the
persisted `UIConfig.engine`: Piper → `UIConfig.piper_voice`, Silero and
Silero Native → `UIConfig.speaker`. This matters whenever the startup
fallback serves a different engine than the config names (e.g.
`engine = "silero_native"` with no model bundle on disk runs Piper for that
session): the fallback engine SHALL receive its own voice id.

#### Scenario: fallback engine receives its own voice

- GIVEN the persisted config has `engine = "silero_native"` and
  `piper_voice = "ruslan"`, and the Silero Native bundle is not downloaded,
  so the active engine is Piper
- WHEN a synthesis runs
- THEN the Piper engine is invoked with voice `ruslan`, not the Silero
  speaker id

#### Scenario: no reverse coercion

- GIVEN the persisted config has `engine = "piper"` and
  `speaker = "aidar"`, and the active engine is Silero Native
- WHEN a synthesis runs
- THEN the engine is invoked with voice `aidar`, not the Piper voice id

### Requirement: Piper voice auto-download on synthesis

When a synthesis on the **active** Piper engine fails with
`voice_not_installed`, the system SHALL download the voice via the Piper
voice catalog and retry the synthesis once. The auto-download SHALL emit the
`voice_download_*` events so the user sees a progress notification instead
of a silent stall; only a failed download (or a failed retry) surfaces an
error to the entry. The gate SHALL key on the active engine kind, so a Piper
fallback session (persisted config naming a Silero engine) is covered too.
Auto-download does not apply to the Silero engines — their voices ship with
the engine.

#### Scenario: missing Piper voice is fetched transparently

- GIVEN the active engine is Piper and the configured Piper voice is not on
  disk
- WHEN a synthesis runs
- THEN a `voice_download_started` event fires, the voice files are
  downloaded, and the synthesis is retried once with the same parameters

#### Scenario: failed download surfaces the error

- GIVEN the active engine is Piper and the configured voice is not in the
  catalog (or the download fails)
- WHEN a synthesis runs
- THEN `voice_download_finished` carries `ok: false` with the message, and
  the entry transitions to `error`

#### Scenario: fallback session covered

- GIVEN the persisted config has `engine = "silero_native"`, the bundle is
  missing, and the active engine is Piper
- WHEN a synthesis hits `voice_not_installed`
- THEN the auto-download and retry run exactly as if Piper were the
  persisted engine
