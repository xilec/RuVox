# Delta: storage

## ADDED Requirements

### Requirement: TeraTTS English voice field

`UIConfig` SHALL gain a `teratts_en_voice` field:

| Field | Type | Default | Meaning |
|-------|------|---------|---------|
| `teratts_en_voice` | string | `"eng_f5"` | TeraTTS English voice id (`eng_f5`, `eng_m3`, `eng_f3`, `eng_m4`, `eng_f4_whisper`, `eng_m2_whisper`) |

The existing defaulting, unknown-key and partial-update rules SHALL apply to
the field unchanged: a config without it SHALL parse with the default, an
unknown value SHALL round-trip without coercion (the engine reports an unknown
voice at synthesis time), and `UIConfigPatch` SHALL accept it as an optional
field.

#### Scenario: Older config without teratts_en_voice key

- GIVEN a `config.json` written before the English voice existed
- WHEN the configuration is loaded
- THEN it parses successfully with `teratts_en_voice` defaulted to `"eng_f5"`

#### Scenario: English voice round-trips

- GIVEN a configuration with `teratts_en_voice` set to `"eng_m3"`
- WHEN the configuration is saved and loaded again
- THEN the loaded value is `"eng_m3"`

#### Scenario: Partial update changes only the English voice

- GIVEN a saved configuration with `teratts_voice: "ru_f1"` and `teratts_en_voice: "eng_f5"`
- WHEN a patch with `teratts_en_voice: "eng_f3"` is applied
- THEN only the English voice changes and the Russian voice stays `"ru_f1"`
