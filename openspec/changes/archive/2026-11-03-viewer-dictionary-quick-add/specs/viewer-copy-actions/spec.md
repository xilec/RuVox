# Delta: viewer-copy-actions

## MODIFIED Requirements

### Requirement: Viewer context menu

Right-clicking inside the `TextViewer` content area SHALL open a custom
context menu (the native webview menu SHALL be suppressed) in every display
mode (plain, markdown, HTML). The menu SHALL contain only the items
applicable to the click target: a link target adds "Скопировать адрес
ссылки"; a non-empty text selection adds "Копировать"; an image target adds
"Скопировать изображение" and "Скопировать адрес изображения"; a selection
that is exactly one valid source token adds «В словарь» before the copy items
(user-dictionary spec, "Viewer quick-add"). If no item applies, the menu
SHALL NOT open.

#### Scenario: Menu on a link

- **WHEN** the user right-clicks a link inside the viewer
- **THEN** a context menu opens with "Скопировать адрес ссылки"

#### Scenario: Menu on selected text

- **GIVEN** the user has selected text inside the viewer
- **WHEN** the user right-clicks the selection
- **THEN** a context menu opens with "Копировать"

#### Scenario: Menu on a single word

- **GIVEN** the user has selected exactly one valid source token inside the
  viewer
- **WHEN** the user right-clicks the selection
- **THEN** a context menu opens with «В словарь» before «Копировать»

#### Scenario: Menu on an image

- **WHEN** the user right-clicks an image inside the viewer
- **THEN** a context menu opens with "Скопировать изображение" and
  "Скопировать адрес изображения"

#### Scenario: No applicable target

- **WHEN** the user right-clicks an empty area of the viewer with no
  selection
- **THEN** no context menu opens
