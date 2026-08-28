## MODIFIED Requirements

### Requirement: Application shell layout

The system SHALL render the root layout via `MantineAppShell` in `src/components/AppShell.tsx` with a fixed 74 px header containing only the `Player` component, a navbar containing the queue, and a main area rendering `TextViewer` for the selected entry.

The header height of 74 px with asymmetric padding (top 18 / bottom 8 in `Player.module.css`) MUST be preserved: the floating slider labels (position and volume) must not be clipped by the native window title bar, and the 48 px brand logo must fit next to the Play button.

The navbar SHALL be drag-resizable via an invisible 6 px strip on its right border with `cursor: col-resize`; the width MUST be clamped to a minimum of 180 px and a maximum of 70% of the window width.

The navbar SHALL contain a "Очередь" title with an Add split-button (size `xs`) to its right — the primary part keeps the clipboard Add behavior and the dropdown menu offers the import actions «Файл…», «Файл с кодировкой…», and «По ссылке…» (see the text-import capability) — plus a search input filtering the queue, and the `QueueList` component. The `PreviewDialog` and `SettingsModal` SHALL be mounted from `AppShell`.

While content is dragged over the window, AppShell SHALL render a full-window drop overlay («Отпустите, чтобы добавить») above the layout; the overlay SHALL not intercept clicks when no drag is active.

#### Scenario: Window renders the three regions

- GIVEN the application has started
- WHEN the main window is displayed
- THEN the header shows only the player controls, the navbar shows "Очередь" with an Add split-button, a search field and the entry list, and the main area shows the selected entry's text (or a "Нет выбранной записи" placeholder)

#### Scenario: Navbar resize respects bounds

- GIVEN the application window is 1000 px wide
- WHEN the user drags the navbar's right border far to the left
- THEN the navbar width stops at 180 px
- AND when the user drags it far to the right the width stops at 700 px (70% of the window width)

#### Scenario: Search hotkey focuses the queue filter

- GIVEN the application window is focused
- WHEN the user presses Ctrl+F (or Cmd+F)
- THEN focus moves to the "Поиск по записям" input and the webview's built-in find-in-page is suppressed
