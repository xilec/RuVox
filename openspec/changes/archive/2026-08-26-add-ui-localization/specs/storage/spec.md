# Delta: storage

## MODIFIED Requirements

### Requirement: Config File Schema

The `UIConfig` field table gains:

| Field | Type | Default | Meaning |
|-------|------|---------|---------|
| `language` | string | `"ru"` | UI language: `"ru"` / `"en"` |

Every existing defaulting/unknown-key/partial-update rule applies to the new
field unchanged.

#### Scenario: Older config without language key
- **GIVEN** a `config.json` written by a pre-localization build (no `language` key)
- **WHEN** the configuration is loaded
- **THEN** it parses successfully with `language` defaulted to `"ru"`

#### Scenario: Language round-trips
- **GIVEN** a configuration with `language` set to `"en"`
- **WHEN** the configuration is saved and loaded again
- **THEN** the loaded value is `"en"`

#### Scenario: Missing config returns defaults
- **GIVEN** no `config.json` in the cache directory
- **WHEN** the configuration is loaded
- **THEN** the default configuration is returned (`speaker` `"aidar"`, `sample_rate` `24000`, `engine` `"silero_native"`, `piper_voice` `"ruslan"`)

#### Scenario: Older config without engine keys
- **GIVEN** a `config.json` that contains only `speaker`, `sample_rate`, and `speech_rate`
- **WHEN** the configuration is loaded
- **THEN** it parses successfully with `engine` defaulted to `"silero_native"` and `piper_voice` defaulted to `"ruslan"`

#### Scenario: Config round-trips
- **GIVEN** a configuration with `speaker` `"xenia"` and `sample_rate` `48000`
- **WHEN** the configuration is saved and loaded again
- **THEN** the loaded values match the saved ones
