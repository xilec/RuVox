# Delta: ui

## ADDED Requirements

### Requirement: TeraTTS English voice selector

When the Settings dialog's engine is «TeraTTSv2», the form SHALL show a second
voice selector for the English voice, below the Russian one. It SHALL list the
six English voices shipped in the bundle (`eng_f5`, `eng_m3`, `eng_f3`,
`eng_m4`, `eng_f4_whisper`, `eng_m2_whisper`), preselect the saved
`teratts_en_voice` (default `eng_f5`), and include the chosen value in the
submitted `UIConfigPatch` as `teratts_en_voice`. The field SHALL survive
engine flips like the Russian voice: switching away from TeraTTS and back
SHALL re-show the saved English choice.

The selector SHALL carry Russian UI labels through the localization layer:
«Голос TeraTTSv2 (английский)» with a description naming `eng_f5` as the
default.

#### Scenario: English voice selector lists the bundle voices

- GIVEN the Settings dialog is open with engine «TeraTTSv2» and saved
  `teratts_en_voice: "eng_m3"`
- WHEN the form renders
- THEN the English selector lists all six English voices with `eng_m3`
  preselected

#### Scenario: Saving persists the English voice

- GIVEN the user picks `eng_f3` in the English voice selector
- WHEN the user clicks "Сохранить"
- THEN the submitted `UIConfigPatch` contains `teratts_en_voice: "eng_f3"`

#### Scenario: English voice survives an engine flip

- GIVEN the user picked `eng_f4_whisper`, then switched to Piper and back to
  «TeraTTSv2»
- WHEN the form re-renders
- THEN the English selector shows `eng_f4_whisper` again
