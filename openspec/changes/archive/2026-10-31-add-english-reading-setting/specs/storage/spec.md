# Delta: storage

## MODIFIED Requirements

### Requirement: Config File Schema

The `UIConfig` field table SHALL gain:

| Field | Type | Default | Meaning |
|-------|------|---------|---------|
| `language` | string | `"ru"` | UI language: `"ru"` / `"en"` |
| `teratts_voice` | string | `"ru_m5"` | TeraTTS voice id from the installed bundle manifest (`voices[].id`); `ru_m5` is the default |
| `english_reading` | string | `"auto"` | How Latin prose is read: `"auto"` / `"native"` / `"narrated"` |

Every existing defaulting/unknown-key/partial-update rule SHALL apply to the
new fields unchanged.

The `code_block_mode` field SHALL accept the values `"brief"` and `"read"`,
with `"brief"` as the default for fresh and partial configs. The legacy value
`"skip"` SHALL be accepted on load as an alias for `"brief"`; any other value
SHALL fall back to `"brief"`. The `read_operators` field SHALL NOT be part of
`UIConfig`; config files written by earlier builds that contain it MUST still
parse (the unknown key is ignored and dropped on the next save).

The `english_reading` field SHALL accept the values `"auto"`, `"native"` and
`"narrated"`, with `"auto"` as the default for fresh and partial configs; any
other value SHALL fall back to `"auto"` on load and when applied through a
`UIConfigPatch`, so a config written by a newer build cannot put the pipeline
in an undefined state.

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
- **THEN** the default configuration is returned (`speaker` `"aidar"`, `sample_rate` `24000`, `engine` `"silero_native"`, `piper_voice` `"ruslan"`, `code_block_mode` `"brief"`, `english_reading` `"auto"`)

#### Scenario: Older config without engine keys
- **GIVEN** a `config.json` that contains only `speaker`, `sample_rate`, and `speech_rate`
- **WHEN** the configuration is loaded
- **THEN** it parses successfully with `engine` defaulted to `"silero_native"` and `piper_voice` defaulted to `"ruslan"`

#### Scenario: Config round-trips
- **GIVEN** a configuration with `speaker` `"xenia"` and `sample_rate` `48000`
- **WHEN** the configuration is saved and loaded again
- **THEN** the loaded values match the saved ones

#### Scenario: Older config without code_block_mode key
- **GIVEN** a `config.json` written by an earlier build with no `code_block_mode` key
- **WHEN** the configuration is loaded
- **THEN** it parses successfully with `code_block_mode` defaulted to `"brief"`

#### Scenario: Persisted read value stays read
- **GIVEN** a `config.json` with `code_block_mode: "read"`
- **WHEN** the configuration is loaded and saved again
- **THEN** the value stays `"read"` (no migration or coercion)

#### Scenario: Legacy skip value is aliased to brief
- **GIVEN** a `config.json` with `code_block_mode: "skip"`
- **WHEN** the configuration is loaded
- **THEN** `code_block_mode` resolves to `"brief"`

#### Scenario: Unknown code_block_mode value falls back to brief
- **GIVEN** a `config.json` with `code_block_mode: "loud"`
- **WHEN** the configuration is loaded
- **THEN** `code_block_mode` resolves to `"brief"`

#### Scenario: Older config without english_reading key
- **GIVEN** a `config.json` written by a build before this setting existed
- **WHEN** the configuration is loaded
- **THEN** it parses successfully with `english_reading` defaulted to `"auto"`

#### Scenario: Persisted english_reading round-trips
- **GIVEN** a `config.json` with `english_reading: "narrated"`
- **WHEN** the configuration is loaded and saved again
- **THEN** the value stays `"narrated"` (the capability of the active engine is
  not consulted by the storage layer and never rewrites it)

#### Scenario: Unknown english_reading value falls back to auto
- **GIVEN** a `config.json` with `english_reading: "bilingual"`
- **WHEN** the configuration is loaded
- **THEN** `english_reading` resolves to `"auto"`
- **AND** a `UIConfigPatch` carrying `"bilingual"` stores `"auto"` as well

#### Scenario: Config with read_operators parses
- **GIVEN** a `config.json` written by an earlier build containing `read_operators: false`
- **WHEN** the configuration is loaded
- **THEN** it parses successfully, all other fields keep their values, and saving drops the `read_operators` key

#### Scenario: Older config without teratts_voice key
- **GIVEN** a `config.json` written by a build before the TeraTTS engine existed
- **WHEN** the configuration is loaded
- **THEN** it parses successfully with `teratts_voice` defaulted to `"ru_m5"`

#### Scenario: TeraTTS voice round-trips
- **GIVEN** a configuration with `teratts_voice` set to `"eng_f5"`
- **WHEN** the configuration is saved and loaded again
- **THEN** the loaded value is `"eng_f5"` (the field accepts every id the bundle manifest ships, not a fixed set)
