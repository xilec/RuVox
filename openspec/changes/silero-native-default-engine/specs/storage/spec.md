# Delta spec: storage

## MODIFIED Requirements

### Requirement: Config File Schema

The system SHALL persist application configuration to `config.json` as a `UIConfig` JSON object:

| Field | Type | Default | Meaning |
|-------|------|---------|---------|
| `speaker` | string | `"aidar"` | Silero speaker name |
| `sample_rate` | number | `24000` | TTS output rate; any of 8000 / 12000 / 16000 / 24000 / 48000 round-trips through the Opus encoder without resampling |
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
