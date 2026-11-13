## ADDED Requirements

### Requirement: Viewer quick-add

The viewer context menu SHALL offer «В словарь» when the selection inside the
viewer is exactly one valid source token as defined by "Dictionary entry
semantics", in every display mode (plain, Markdown, HTML). The item SHALL NOT
appear for a multi-word selection, an invalid token, or no selection; those
selections keep the menu's copy-only contract and the menu SHALL still open
when a copy action applies. The item's placement relative to the copy items
SHALL follow "Viewer context menu" in the viewer-copy-actions spec.

Choosing the item SHALL open the existing dictionary editor prefilled with the
selected word as the add form's source word and the global scope selected,
with the scope selector and the immediate-save path unchanged; saving SHALL
persist the entry to the selected collection exactly like the preview
quick-add. Opening and closing the editor SHALL NOT pause, stop or otherwise
interrupt playback.

Closing the editor SHALL NOT offer to re-synthesize the entry and SHALL NOT
issue a regeneration command, whether or not the session wrote to the
dictionary. Re-synthesizing a recording after a dictionary change stays
available only through the entry's context menu («Перегенерировать аудио»),
which keeps its own preview and confirmation semantics; the automatic offer
after the editor closes SHALL NOT exist.

#### Scenario: Single word in plain mode

- GIVEN the viewer shows a plain-text entry and the user selected the single
  word "getUserData"
- WHEN the user right-clicks the selection
- THEN the menu shows «В словарь»

#### Scenario: Single word in Markdown and HTML modes

- GIVEN the viewer renders a Markdown or an HTML entry
- WHEN the user right-clicks a selected single word
- THEN the menu shows «В словарь»

#### Scenario: Hyphenated token

- GIVEN the viewer shows a plain-text entry and the user selected "UTF-8"
- WHEN the user right-clicks the selection
- THEN the menu shows «В словарь»

#### Scenario: Multi-word selection

- GIVEN the user selected several words inside the viewer
- WHEN the user right-clicks the selection
- THEN the menu offers only «Копировать» and no «В словарь» item

#### Scenario: Mixed-script token

- GIVEN the viewer shows a plain-text entry and the user selected "IPзамок"
- WHEN the user right-clicks the selection
- THEN the menu offers only «Копировать» and no «В словарь» item

#### Scenario: Invalid token selection

- GIVEN the user selected a fragment that is not one valid source token
  (punctuation other than an internal hyphen, a multi-word run, or a
  digit-only token)
- WHEN the user right-clicks the selection
- THEN the menu offers only «Копировать»

#### Scenario: Editor is prefilled

- GIVEN the user chose «В словарь» for the selected word "getUserData"
- WHEN the editor opens
- THEN the add form's source word is "getUserData", the spoken form is empty,
  and the scope selector shows the global scope

#### Scenario: Saving persists the entry

- GIVEN the editor opened from the viewer is prefilled with "getUserData"
- WHEN the user enters the spoken form and saves
- THEN the entry is persisted to the selected collection exactly like a
  preview quick-add entry

#### Scenario: Closing after a save offers nothing

- GIVEN the editor was opened from the viewer quick-add and the user saved at
  least one entry
- WHEN the user closes the editor
- THEN no regeneration offer appears and no regeneration command is issued

#### Scenario: Playback survives the editor

- GIVEN an entry is playing
- WHEN the user opens the dictionary editor from the viewer and closes it
- THEN playback continues uninterrupted
