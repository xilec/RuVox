## MODIFIED Requirements

### Requirement: UI localization layer

The frontend SHALL localize user-visible strings through a hand-rolled
localization layer (`src/i18n/`): string catalogs for `ru` (default) and
`en`, keyed by stable message ids; a translation helper `t(key, params?)`;
and a locale store (`src/stores/locale.ts`) seeded from
`commands.getConfig()` at startup and updated by the Settings selector.
Interpolation SHALL use positional `{0}`-style params. A missing catalog key
SHALL fall back to the RU entry, then to the key itself. Non-React modules
(notification bridge, updater, copy helpers) SHALL read the active locale at
call time.

All user-visible strings of the shell, queue, player, viewer, dialogs, and
notification helpers SHALL come from the catalogs; no hardcoded UI prose in
components. The viewer header (#353) SHALL localize its edit affordances,
keyboard hints, the empty-note state and the «не озвучено» placeholder, and the
queue card's title tooltip and search over title/note SHALL work in both
languages. Backend error codes (`CommandError.code`) SHALL have catalog
entries under an `errors.` prefix; unknown codes fall back to `message`, then
to a generic per-`type` string (see the ipc-commands spec).

#### Scenario: RU is the default on first run
- **GIVEN** a fresh config without a saved language
- **WHEN** the app starts
- **THEN** every visible string comes from the RU catalog

#### Scenario: Switching to EN relabels everything
- **GIVEN** the app running in Russian
- **WHEN** the user switches the language to English in Settings
- **THEN** component labels, dialog texts, status badges, toasts triggered
  afterwards, and error messages rendered from backend codes are all in
  English


### Requirement: Recording parameters dialog

The queue context menu item "Параметры записи…" — and a double-click on the
queue item, gated the same way — SHALL open a read-only dialog showing the
entry's ingestion source ("Источник": буфер обмена / файл / ссылка) as the
first row, followed by the generation snapshot: engine, voice, sample rate,
model identity (name and checksum when available), app version, the code-block
narration mode used, normalized-text checksum, audio codec and size, duration,
generation timestamp, and the generation number (`generation_count`). The item
SHALL be disabled for entries that have neither a snapshot nor a generation
timestamp (never synthesized).

Values absent from the snapshot SHALL render as a placeholder dash rather than guessed values. For entries with audio but no snapshot (synthesized by older builds), the dialog SHALL show an explanatory line that parameters were not recorded. Engine and voice names SHALL be shown with their localized or engine-specific display names: Piper voices by their localized names, Silero speakers by their speaker name, TeraTTS voices by their stored id, and an id the catalogue does not know by the stored id. The viewer header SHALL use the same shared display helpers, so the two surfaces never disagree. The dialog and menu item SHALL be localized in Russian and English.

#### Scenario: Dialog shows the snapshot

- GIVEN a ready entry whose snapshot records engine `silero_native`, speaker `xenia`, and an Ogg Opus file
- WHEN the user opens "Параметры записи…" for the entry
- THEN the dialog shows the ingestion source first, then the localized engine name, the Silero speaker name (`Xenia`), the sample rate, the audio codec and size, and the generation number
#### Scenario: Absent values render as a dash

- GIVEN a ready entry whose snapshot has no model identity
- WHEN the parameters dialog is open
- THEN the model row renders a placeholder dash, not an invented value

#### Scenario: Legacy entry without a snapshot

- GIVEN a ready entry synthesized by an older build (`generation` is null, `audio_generated_at` is set)
- WHEN the user opens "Параметры записи…" for the entry
- THEN the dialog opens, shows the generation timestamp and duration where known, an explanatory line that parameters were not recorded, and dashes for unknown values

#### Scenario: Item disabled for never-synthesized entries

- GIVEN a pending entry with no audio
- WHEN the user opens the context menu for the entry
- THEN "Параметры записи…" is disabled


### Requirement: Text viewer reading controls

The text viewer SHALL show reading-typography controls in the viewer header's
metadata row (the entry header added by #353), placed after the display-mode
switch when one is rendered, in every display mode — including a plain source,
which has no display-mode switch:

- a `Узко | Широко` `SegmentedControl` bound to `UIConfig.reading_width`
  (`"narrow"` | `"wide"`);
- a `[ − ] N% [ + ]` stepper bound to `UIConfig.reading_scale` with the ladder
  80/100/125/150/175/200 percent: `−` moves to the nearest lower step, `+` to
  the nearest higher step, and a separate `↺` reset button returns the scale to
  100 %; the bound buttons are disabled at 80 % and 200 %, and the reset button
  is disabled while the scale is 100 %.

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
  header metadata row, and either control changes the content immediately

#### Scenario: Scale ladder and bounds

- **GIVEN** the saved `reading_scale` is 100
- **WHEN** the user presses `+` twice
- **THEN** the displayed scale is 150 %, `+` stays enabled, and at 200 % `+` is
  disabled while at 80 % `−` is disabled

#### Scenario: Off-ladder persisted scale

- **GIVEN** the persisted `reading_scale: 93`
- **WHEN** the viewer renders
- **THEN** the stepper shows `93 %`, `−` moves it to 80 %, and `+` moves it to 100 %

#### Scenario: The reset button returns the scale to 100 %

- **GIVEN** the saved `reading_scale` is 175
- **WHEN** the user clicks `↺`
- **THEN** the scale becomes 100 % and `update_config` receives
  `reading_scale: 100`
- **AND** the `↺` button is disabled while the scale is 100 %

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

