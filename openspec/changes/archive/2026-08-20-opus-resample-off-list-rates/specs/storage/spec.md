# Delta: storage

## MODIFIED Requirements

### Requirement: Audio File Storage

The system SHALL store synthesized audio per entry as `audio/{uuid}.opus`, where `{uuid}` is the entry's `EntryId`. The file SHALL be an Ogg-Opus stream:

| Property | Value |
|----------|-------|
| Container | Ogg |
| Codec | Opus (RFC 6716, RFC 7845) |
| Channels | 1 (mono) |
| Sample rate | One of 8 / 12 / 16 / 24 / 48 kHz — the rates libopus accepts natively (RFC 6716 §2). The TTS subprocess SHOULD write one of these; if it writes any other rate (e.g. a Piper voice at 22050 Hz, or 44100 Hz), the Rust side SHALL resample it to the nearest native rate before encoding. `OpusHead` SHALL record the native (resampled) rate the encoder actually used, not the original off-list rate |
| Bitrate | 32 000 bps (VOIP application) |
| Frame size | 20 ms |
| Pre-skip | Queried from `libopus`'s lookahead, scaled to 48 kHz output ticks |

The encoding pipeline is: the TTS subprocess writes a mono 32-bit-float WAV; the Rust side transcodes it to Opus and removes the source WAV. If the WAV's sample rate is not one of the Opus-native rates, the Rust side SHALL resample it to the nearest native rate first. On encode failure the source `.wav` SHALL be left in place as a playback fallback. `save_audio` SHALL return the relative filename for `TextEntry.audio_path`.

#### Scenario: Saving audio returns the relative filename
- GIVEN an entry with id `550e8400-e29b-41d4-a716-446655440000`
- WHEN audio bytes are saved for the entry
- THEN the file `audio/550e8400-e29b-41d4-a716-446655440000.opus` exists and the returned filename is `550e8400-e29b-41d4-a716-446655440000.opus`

#### Scenario: Transcode failure keeps the WAV fallback
- GIVEN a synthesized `.wav` that fails Opus encoding
- WHEN the transcode step runs
- THEN the source `.wav` remains on disk so playback can still use it

#### Scenario: Off-list sample rate is resampled to the nearest native rate
- GIVEN a synthesized mono 32-bit-float WAV at an off-list rate (e.g. 22050 Hz from a Piper voice)
- WHEN the Rust side transcodes it to Opus
- THEN the entry is stored as `.opus` (the source `.wav` is removed), `OpusHead` records 24000 Hz, and no "unsupported wav format" error is logged

#### Scenario: Native sample rate passes through without resampling
- GIVEN a synthesized mono 32-bit-float WAV at a native rate (e.g. 48000 Hz)
- WHEN the Rust side transcodes it to Opus
- THEN the entry is stored as `.opus` and `OpusHead` records 48000 Hz, with no resampling step

### Requirement: Legacy WAV to Opus Migration

On every app launch the system SHALL run a one-shot migration sweep over the loaded entries: any entry whose `audio_path` ends in `.wav` SHALL be transcoded to `.opus`, the entry's `audio_path` updated to the new filename, and the source `.wav` removed. The sweep SHALL be idempotent (already-`.opus` entries are not considered) and SHALL NOT abort on per-entry failures — encode errors and missing source files are logged and counted while the app keeps starting normally. Legacy `.wav` references in `history.json` SHALL continue to parse indefinitely. Off-list WAV rates (e.g. 22050 Hz Piper clips) SHALL be resampled to the nearest native rate during the transcode, not rejected.

#### Scenario: Legacy entry is migrated
- GIVEN an entry whose `audio_path` points at an existing `{uuid}.wav`
- WHEN the migration sweep runs
- THEN the entry's `audio_path` ends in `.opus`, the `.opus` file exists, and the source `.wav` is removed

#### Scenario: Legacy off-list-rate WAV is migrated
- GIVEN an entry whose `audio_path` points at an existing `{uuid}.wav` at an off-list rate (e.g. 22050 Hz)
- WHEN the migration sweep runs
- THEN the entry is migrated to `.opus` (resampled to the nearest native rate) and the source `.wav` is removed

#### Scenario: Migration is idempotent
- GIVEN all entries already reference `.opus` files
- WHEN the migration sweep runs
- THEN no entries are considered and no files are touched

#### Scenario: Missing source file does not abort the sweep
- GIVEN one entry referencing a missing `.wav` and another referencing an existing `.wav`
- WHEN the migration sweep runs
- THEN the missing one is skipped with a warning and the existing one is migrated

### Requirement: Config File Schema

The system SHALL persist application configuration to `config.json` as a `UIConfig` JSON object:

| Field | Type | Default | Meaning |
|-------|------|---------|---------|
| `speaker` | string | `"aidar"` | Silero speaker name |
| `sample_rate` | number | `24000` | TTS output rate for the native engines; the native Opus rates 8000 / 12000 / 16000 / 24000 / 48000 round-trip without resampling, and any other rate is resampled to the nearest native one before encoding |
| `speech_rate` | number | `1.0` | Playback speed multiplier (0.5–2.0) |
| `notify_on_ready` | boolean | `true` | Show notification when synthesis completes |
| `notify_on_error` | boolean | `true` | Show notification on synthesis error |
| `text_format` | string | `"plain"` | Default viewer format: `"plain"` / `"markdown"` / `"html"` |
| `max_cache_size_mb` | number | `500` | Soft limit on audio cache size in MB; drives startup eviction (0 = disabled) |
| `code_block_mode` | string | `"read"` | How to handle Markdown code blocks: `"skip"` / `"read"` |
| `read_operators` | boolean | `true` | Whether to speak mathematical/code operators |
| `theme` | string | `"auto"` | Color scheme: `"light"` / `"dark"` / `"auto"` |
| `player_hotkeys` | object | 10-key map (`play_pause` → `"Space"`, `forward_5` → `"Right"`, `backward_5` → `"Left"`, `forward_30` → `"Shift+Right"`, `backward_30` → `"Shift+Left"`, `speed_up` → `"]"`, `speed_down` → `"["`, `next_entry` → `"n"`, `prev_entry` → `"p"`, `repeat_sentence` → `"r"`) | Local player hotkeys |
| `window_geometry` | `[x, y, width, height]` or null | `null` | Saved window geometry |
| `preview_dialog_enabled` | boolean | `true` | Show normalization preview dialog before synthesis |
| `engine` | string | `"silero_native"` | Active TTS engine: `"piper"` / `"silero"` / `"silero_native"` |
| `piper_voice` | string | `"ruslan"` | Active Piper voice id |

Every field SHALL default when absent from the JSON, so configs written by older builds parse cleanly and silently adopt current defaults (e.g. pre-engine configs switch to `"silero_native"`). Unknown JSON keys SHALL be ignored on read. When `config.json` does not exist, the service SHALL return the default configuration. Partial updates SHALL be expressed as a patch object in which omitted fields keep their current value.

#### Scenario: Missing config returns defaults
- GIVEN no `config.json` in the cache directory
- WHEN the configuration is loaded
- THEN the default configuration is returned (`speaker` `"aidar"`, `sample_rate` `24000`, `engine` `"silero_native"`, `piper_voice` `"ruslan"`)

#### Scenario: Older config without engine keys
- GIVEN a `config.json` that contains only `speaker`, `sample_rate`, and `speech_rate`
- WHEN the configuration is loaded
- THEN it parses successfully with `engine` defaulted to `"silero_native"` and `piper_voice` defaulted to `"ruslan"`

#### Scenario: Config round-trips
- GIVEN a configuration with `speaker` `"xenia"` and `sample_rate` `48000`
- WHEN the configuration is saved and loaded again
- THEN the loaded values match the saved ones
