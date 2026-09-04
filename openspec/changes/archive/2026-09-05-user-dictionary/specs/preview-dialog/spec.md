## ADDED Requirements

### Requirement: Quick-add to dictionary from preview

The preview dialog SHALL offer a "В словарь" action, enabled only while the
current text selection in either pane is a single valid source token (Latin
letters and digits, at least one letter). Activating it SHALL open the
dictionary editor with the add form prefilled: `from` set to the selected
token, `to` empty. Selections that are not a single valid token (Cyrillic,
multi-word, containing punctuation) SHALL leave the action disabled with a
hint explaining what a valid token is.

#### Scenario: Latin word selection enables the action

- GIVEN the preview shows "Ivanov" in the original pane
- WHEN the user selects exactly "Ivanov"
- THEN the "В словарь" action becomes enabled and opens the editor with
  from "Ivanov" and an empty spoken form

#### Scenario: Cyrillic selection keeps the action disabled

- GIVEN the preview shows normalized Cyrillic text
- WHEN the user selects a Cyrillic word
- THEN the "В словарь" action stays disabled
