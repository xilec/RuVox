# Delta: user-dictionary

## ADDED Requirements

### Requirement: Viewer quick-add

The viewer context menu SHALL offer «В словарь» when the selection inside the
viewer is exactly one valid source token as defined by "Dictionary entry
semantics" — one word of Latin or Russian Cyrillic letters and digits with at
least one letter and optional internal hyphens — in every display mode
(plain, Markdown, HTML). The item SHALL NOT appear for a multi-word
selection, an invalid token, or no selection; those selections keep the
menu's copy-only contract and the menu SHALL still open when a copy action
applies. The item SHALL be rendered before the copy actions.

Choosing the item SHALL open the existing dictionary editor prefilled with the
selected word as the add form's source word, with the scope selector and the
immediate-save path unchanged; saving SHALL persist the entry to the selected
collection exactly like the preview quick-add. Opening and closing the editor
SHALL NOT pause, stop or otherwise interrupt playback.

When the editor was opened from the viewer quick-add and at least one
dictionary write succeeded before it closed, closing the editor SHALL offer to
re-synthesize the entry that was selected when the editor opened, asking
«Переозвучить сейчас?». Confirming SHALL run the existing regeneration command
for that entry; declining SHALL change nothing and leave the stored audio, its
status and its pronunciation untouched. The offer SHALL NOT appear when the
editor was opened from another entry point (Settings or the preview quick-add)
or when nothing was written. If the regeneration fails, the previous audio
SHALL remain playable, following the existing regeneration semantics.

Only the confirmed regeneration MAY stop playback; neither opening the editor,
nor closing it, nor the offer itself SHALL issue a player command.

#### Scenario: Single word in plain mode

- GIVEN the viewer shows a plain-text entry and the user selected the single
  word "getUserData"
- WHEN the user right-clicks the selection
- THEN the menu shows «В словарь» before «Копировать»

#### Scenario: Single word in Markdown and HTML modes

- GIVEN the viewer renders a Markdown or an HTML entry
- WHEN the user right-clicks a selected single word
- THEN the menu shows «В словарь» before the copy items

#### Scenario: Multi-word selection

- GIVEN the user selected several words inside the viewer
- WHEN the user right-clicks the selection
- THEN the menu offers only «Копировать» and no «В словарь» item

#### Scenario: Invalid token selection

- GIVEN the user selected a fragment that is not one valid source token
  (punctuation inside it, a multi-word run, or a digit-only token)
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

#### Scenario: Offer after a save

- GIVEN the editor was opened from the viewer quick-add and the user saved at
  least one entry
- WHEN the user closes the editor
- THEN a confirmation titled «Переозвучить сейчас?» appears

#### Scenario: No offer without a write

- GIVEN the editor was opened from the viewer quick-add
- WHEN the user closes it without saving anything
- THEN no regeneration offer appears and no regeneration command is issued

#### Scenario: No offer for other entry points

- GIVEN the editor was opened from Settings or from the preview quick-add
- WHEN the user saves a word and closes the editor
- THEN no regeneration offer appears

#### Scenario: Confirming the offer regenerates

- GIVEN the editor was opened from the viewer quick-add, the user saved a
  word, closed the editor and confirmed the offer
- WHEN the confirmation is accepted
- THEN the existing regeneration command runs for the entry that was selected
  when the editor opened

#### Scenario: Declining the offer changes nothing

- GIVEN the regeneration offer is shown
- WHEN the user declines it
- THEN no regeneration command runs and the stored audio, its status and its
  pronunciation stay as they were

#### Scenario: Failed regeneration keeps the old audio

- GIVEN the user confirmed the regeneration offer
- WHEN the synthesis fails
- THEN the previous recording remains playable, following the existing
  regeneration semantics

#### Scenario: Playback survives the editor

- GIVEN an entry is playing
- WHEN the user opens the dictionary editor from the viewer and closes it
  without confirming a regeneration
- THEN playback continues uninterrupted
