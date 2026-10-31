## ADDED Requirements

### Requirement: TeraTTS Voice Catalogue Command

The system SHALL provide `get_teratts_voices()` returning the voice ids from the installed TeraTTS bundle manifest (`voices[].id`), preserving manifest order. The command SHALL read only `manifest.json` through the bundle parser: it MUST NOT hash bundle files, walk the bundle directory, or consult the update channel. When the manifest is missing, unreadable, or declares an unsupported `format_version`, the command SHALL succeed with an empty list instead of failing, because `get_available_engines()` already reports the coded reason the engine is unusable.

#### Scenario: Installed bundle reports every voice
- **GIVEN** a TeraTTS bundle whose manifest lists `ru_m5`, `ru_f1`, `eng_f3` and `eng_f5`, in that order
- **WHEN** `get_teratts_voices` is invoked
- **THEN** the result is `["ru_m5", "ru_f1", "eng_f3", "eng_f5"]`

#### Scenario: Missing bundle reports an empty catalogue
- **GIVEN** no TeraTTS bundle in the app data dir
- **WHEN** `get_teratts_voices` is invoked
- **THEN** the command succeeds with an empty list

#### Scenario: Incompatible manifest reports an empty catalogue
- **GIVEN** a TeraTTS bundle whose manifest declares an unsupported `format_version`
- **WHEN** `get_teratts_voices` is invoked
- **THEN** the command succeeds with an empty list
