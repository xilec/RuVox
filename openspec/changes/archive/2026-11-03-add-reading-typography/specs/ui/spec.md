## ADDED Requirements

### Requirement: Text viewer reading controls

The text viewer SHALL show reading-typography controls in a toolbar above the
content, to the right of the display-mode switch, in every display mode —
including a plain source, which has no display-mode switch:

- a `Узко | Широко` `SegmentedControl` bound to `UIConfig.reading_width`
  (`"narrow"` | `"wide"`);
- a `[ − ] N% [ + ]` stepper bound to `UIConfig.reading_scale` with the ladder
  80/100/125/150/175/200 percent: `−` moves to the nearest lower step, `+` to
  the nearest higher step, clicking the percentage resets it to 100, and the
  bound buttons are disabled at 80 % and 200 %.

Each control SHALL persist its change immediately through
`commands.updateConfig` (`{ reading_width }` or `{ reading_scale }`) with
optimistic UI: the control shows the new value before the request resolves, and
on failure it reverts to the previous value and a localized error toast is
shown. Both controls SHALL be keyboard reachable with localized accessible
names from the RU/EN catalogs.

The `reading_scale` SHALL be applied as CSS `zoom` on the viewer content
container in all display modes, so text, headings, code, tables, borders and
images scale by the same factor while the pane keeps its width and the content
re-wraps inside it. `transform: scale()` MUST NOT be used. `pre` SHALL keep
`overflow-x: auto`, and every rendered table SHALL sit in a horizontal scroll
wrapper (`data-scroll-x`), so wide code and tables scroll inside the reading
column instead of being clipped.

`reading_width: "narrow"` SHALL constrain the content to
`max-width: min(var(--ruvox-reading-measure), 100%)` with `margin-inline: auto`;
`"wide"` SHALL keep the current full-pane width. Width and scale SHALL be
independent: the narrow column keeps its character measure as the scale
changes, and the wide layout still fills the pane.

#### Scenario: Controls are present in every display mode

- **GIVEN** an entry is selected, with any source format and display mode
- **WHEN** the viewer renders
- **THEN** the reading-width segments and the reading-scale stepper are in the
  toolbar, and either control changes the content immediately

#### Scenario: Scale ladder and bounds

- **GIVEN** the saved `reading_scale` is 100
- **WHEN** the user presses `+` twice
- **THEN** the displayed scale is 150 %, `+` stays enabled, and at 200 % `+` is
  disabled while at 80 % `−` is disabled

#### Scenario: Clicking the percentage resets the scale

- **GIVEN** the saved `reading_scale` is 175
- **WHEN** the user clicks the percentage value
- **THEN** the scale becomes 100 % and `update_config` receives
  `reading_scale: 100`

#### Scenario: Failed persistence reverts the control

- **GIVEN** the saved `reading_scale` is 100
- **WHEN** the user presses `+` and `update_config` rejects
- **THEN** the control returns to 100 % and a localized error toast is shown

#### Scenario: Zoom scales the whole document

- **GIVEN** an HTML entry with headings, a table, code and an image
- **WHEN** the scale is 150 %
- **THEN** the headings, table, code and image all render 1.5× larger, the pane
  width is unchanged, and tables and code scroll inside the column instead of
  being clipped

#### Scenario: Narrow column measures about 70 characters

- **GIVEN** `reading_width: "narrow"` in a 1440 px window
- **WHEN** the viewer renders
- **THEN** the content is centered with a maximum measure near 70 characters,
  and choosing `"wide"` restores the full-pane layout

## MODIFIED Requirements

### Requirement: Design tokens

The system SHALL define design tokens in exactly two places: `src/theme.ts` (`createTheme` with `primaryColor: 'blue'`, otherwise Mantine defaults) for Mantine-scale values, and `:root` custom properties in `src/globals.css` prefixed `--ruvox-*` for what Mantine has no scale for.

The following `--ruvox-*` tokens SHALL exist:

| Token | Meaning |
|-------|---------|
| `--ruvox-brand` | Brand mark violet; must match `src-tauri/icons/source.svg` |
| `--ruvox-highlight-bg` | Word-highlight background during playback |
| `--ruvox-pre-bg` / `--ruvox-code-bg` | Code surfaces in rendered markdown/HTML |
| `--ruvox-table-border` / `--ruvox-table-header-bg` | Table chrome in rendered content |
| `--ruvox-reading-font-size` / `--ruvox-reading-line-height` | Reading text in TextViewer (17 px / 1.6) |
| `--ruvox-reading-measure` | Maximum line measure of the narrow reading column (70ch) |
| `--ruvox-reading-scale` | Reading zoom multiplier applied to the viewer content (1 = 100 %) |
| `--ruvox-preview-z` / `--ruvox-preview-shadow` | Floating preview window chrome |

CSS Modules MUST NOT contain literal hex/px values where a `--mantine-*` or `--ruvox-*` token exists; one-off measurements tied to a layout quirk (e.g. the 74 px header, the 9 px play-button lift) are allowed inline only with a "why" comment. `html { font-size: 17px }` SHALL scale all rem-based Mantine tokens (HiDPI workaround on Wayland/KDE with WebKitGTK). Light/dark variants SHALL use the native CSS `light-dark()` function inside token definitions.

#### Scenario: Token-driven colors

- GIVEN a component needs the word-highlight background
- WHEN its CSS Module is written
- THEN it references `var(--ruvox-highlight-bg)` instead of a literal color, and the value flips with the color scheme via `light-dark()`

#### Scenario: Reading width and scale come from tokens

- GIVEN the viewer renders the narrow reading column at 150 %
- WHEN its CSS Module is written
- THEN it uses `var(--ruvox-reading-measure)` and `var(--ruvox-reading-scale)` instead of a literal `70ch` or `1.5`, and the scale override is set as an inline custom property on the content container
