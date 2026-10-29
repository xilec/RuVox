# Delta: preview-dialog

## MODIFIED Requirements

### Requirement: Quick-add to dictionary from preview

The preview dialog SHALL offer a "В словарь" action, enabled only while the
current text selection in either pane is a single valid source token: one word
of Latin or Russian Cyrillic letters and digits with at least one letter, all
letters from one script. Activating it SHALL open the dictionary editor with
the add form prefilled: `from` set to the selected token, `to` empty.
Selections that are not a single valid token (multi-word, containing
punctuation, or mixing scripts) SHALL leave the action disabled with a hint
explaining what a valid token is.

#### Scenario: Latin word selection enables the action

- GIVEN the preview shows "Ivanov" in the original pane
- WHEN the user selects exactly "Ivanov"
- THEN the "В словарь" action becomes enabled and opens the editor with
  from "Ivanov" and an empty spoken form

#### Scenario: Russian word selection enables the action

- GIVEN the preview shows "замок" in the normalized pane
- WHEN the user selects exactly "замок"
- THEN the "В словарь" action becomes enabled and opens the editor with
  from "замок" and an empty spoken form

#### Scenario: Multi-word selection keeps the action disabled

- GIVEN the preview shows "docker compose"
- WHEN the user selects the whole phrase
- THEN the "В словарь" action stays disabled with the hint explaining that a
  token is one word
