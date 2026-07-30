# Delta spec: viewer-copy-actions

## ADDED Requirements

### Requirement: Viewer context menu

Right-clicking inside the `TextViewer` content area SHALL open a custom
context menu (the native webview menu SHALL be suppressed) in every display
mode (plain, markdown, HTML). The menu SHALL contain only the items
applicable to the click target: a link target adds "Скопировать адрес
ссылки"; a non-empty text selection adds "Копировать"; an image target adds
"Скопировать изображение" and "Скопировать адрес изображения". If no item
applies, the menu SHALL NOT open.

#### Scenario: Menu on a link
- **WHEN** the user right-clicks a link inside the viewer
- **THEN** a context menu opens with "Скопировать адрес ссылки"

#### Scenario: Menu on selected text
- **GIVEN** the user has selected text inside the viewer
- **WHEN** the user right-clicks the selection
- **THEN** a context menu opens with "Копировать"

#### Scenario: Menu on an image
- **WHEN** the user right-clicks an image inside the viewer
- **THEN** a context menu opens with "Скопировать изображение" and
  "Скопировать адрес изображения"

#### Scenario: No applicable target
- **WHEN** the user right-clicks an empty area of the viewer with no
  selection
- **THEN** no context menu opens

### Requirement: Copy link address

Choosing "Скопировать адрес ссылки" SHALL write the link's original `href`
verbatim (as in the source markup) to the clipboard via
`@tauri-apps/plugin-clipboard-manager`. URLs SHALL NOT be resolved against
the webview origin — for relative links that would produce a meaningless
localhost URL.

#### Scenario: Copy relative link
- **GIVEN** a link `<a href="/ru/users/maybe_elf/">` in the viewer
- **WHEN** the user chooses "Скопировать адрес ссылки"
- **THEN** the clipboard contains `/ru/users/maybe_elf/`

### Requirement: Copy selected text

Choosing "Копировать" SHALL write the current text selection inside the
viewer to the clipboard via the clipboard plugin.

#### Scenario: Copy selection
- **GIVEN** the user selected the text `getUserData` in the viewer
- **WHEN** the user chooses "Копировать"
- **THEN** the clipboard contains `getUserData`

### Requirement: Copy image

Choosing "Скопировать изображение" SHALL fetch the image bytes (remote
images included, via the HTTP plugin) and write the bitmap to the clipboard
via the clipboard plugin's image write. If fetching or writing fails, the
system SHALL show an error notification and leave the clipboard unchanged.
"Скопировать адрес изображения" SHALL write the image's original `src`
verbatim to the clipboard. Only the bitmap fetch resolves relative URLs
against the document base.

#### Scenario: Copy image bitmap
- **GIVEN** an image `<img src="https://habrastorage.org/x.png">` in the
  viewer
- **WHEN** the user chooses "Скопировать изображение"
- **THEN** the image bitmap is written to the clipboard

#### Scenario: Image fetch failure
- **GIVEN** an image whose URL cannot be fetched
- **WHEN** the user chooses "Скопировать изображение"
- **THEN** an error notification is shown and the clipboard is unchanged

#### Scenario: Copy image address
- **WHEN** the user chooses "Скопировать адрес изображения" on an image
- **THEN** the clipboard contains the image's original `src` verbatim

### Requirement: Copy link hotkey

When the focus or text selection inside the viewer is on a link, pressing
Ctrl+C / Cmd+C SHALL copy the link's original `href` verbatim instead of the
default copy behavior. Otherwise the default copy behavior SHALL be
preserved.

#### Scenario: Hotkey on a focused link
- **GIVEN** keyboard focus is on a link inside the viewer
- **WHEN** the user presses Ctrl+C
- **THEN** the clipboard contains the link's original `href` verbatim

#### Scenario: Hotkey with a regular selection
- **GIVEN** a text selection that is not on a link
- **WHEN** the user presses Ctrl+C
- **THEN** the default copy of the selected text occurs
