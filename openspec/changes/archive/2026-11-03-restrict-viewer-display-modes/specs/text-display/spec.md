# Delta: text-display

## MODIFIED Requirements

### Requirement: Display mode switching

The `TextViewer` SHALL derive the offered views from the entry's source
format and SHALL NOT offer a view that reinterprets the text as another
format. The source format is resolved as follows: a stored `html_source`
means `html`; an explicit `format` of `markdown` or `plain` is used as
recorded; a `null` format (older builds, the tray ingestion path) and a
stored `html` without markup fall back to content detection
(`detectFormat(original_text)`). A stored `html` without `html_source` is
not trusted on its own: HTML ingestion always stores the markup, so such a
value came from a path that recorded the detection result without keeping
the markup (the dialog-less Add path). Detection remains authoritative
there and may yield `html` again; the viewer then renders `original_text`
through the sanitized HTML path instead of treating the state as
impossible. A stale `format` that contradicts the derived source (the old
viewer toggle wrote its display choice into `format`) SHALL be clamped at
render time; no data migration is performed.

The allowed views are:

- `markdown` → the rendered markdown view (`Markdown`, the default) and the
  verbatim view («Исходный»);
- `html` → the rendered HTML view (`HTML`, the default) and the verbatim
  view («Исходный»);
- `plain` → only the verbatim view, and the `SegmentedControl` SHALL NOT be
  rendered for such an entry.

Switching views SHALL re-render the same original text instantly and SHALL
persist the choice in the entry's `display_mode` via
`set_entry_display_mode`; it SHALL NOT change the source `format`. When an
entry is selected, the viewer SHALL restore its persisted `display_mode`;
an entry without one SHALL show the default view of its source format. The
persisted choice is display-only: it does not change the stored text, audio,
or timestamps.

The rendered HTML view SHALL use the stored `html_source`. An entry whose
HTML source was inferred from content (a legacy entry or a stored `format:
"html"` without markup) has no stored markup and SHALL render its
`original_text` as the fallback, sanitized through the same HTML path. A
plain source SHALL render `original_text` verbatim even when a stale
`display_mode: "rendered"` is stored.

The labels SHALL come from the translation catalog: the rendered option
names its format (`Markdown` / `HTML`), the verbatim option reads
«Исходный» (EN: "Raw").

#### Scenario: Markdown entry offers only the markdown views

- GIVEN an entry whose source format is `markdown`
- WHEN the entry is selected in the viewer
- THEN the `SegmentedControl` offers `Markdown` and «Исходный», no `HTML`
  option is present, and the text renders in markdown mode by default

#### Scenario: Switch display mode

- GIVEN a markdown entry displayed in the rendered view
- WHEN the user selects «Исходный»
- THEN the markup characters are shown literally and
  `set_entry_display_mode` persists `"plain"` on the entry, while `format`
  stays `"markdown"`

#### Scenario: HTML entry offers only the HTML views

- GIVEN an entry whose source format is `html` with a stored `html_source`
- WHEN the entry is selected
- THEN the `SegmentedControl` offers `HTML` and «Исходный», and the rendered
  view uses the stored `html_source`

#### Scenario: Plain entry has no switch

- GIVEN an entry whose source format is `plain`
- WHEN the entry is selected
- THEN no `SegmentedControl` is rendered and the text is displayed verbatim
  (markup visible)

#### Scenario: Selection restores the saved mode

- GIVEN an entry persisted with `display_mode: "plain"`
- WHEN the user selects that entry in the queue
- THEN the `SegmentedControl` shows «Исходный» and the text renders verbatim

#### Scenario: Entry without a saved mode uses the default

- GIVEN a markdown entry with `display_mode: null`
- WHEN the user selects that entry
- THEN the rendered markdown view is shown

#### Scenario: Legacy entry without a source format

- GIVEN an entry with `format: null` and no `html_source`
- WHEN the viewer derives its source
- THEN the source is `detectFormat(original_text)`: a markdown-signalled
  text gets `Markdown` / «Исходный», a plain text gets no switch, and an
  HTML-signalled text gets `HTML` / «Исходный»

#### Scenario: Stale format is clamped

- GIVEN an entry with `format: "html"`, no `html_source`, and markdown text
- WHEN the entry is displayed
- THEN the viewer offers `Markdown` / «Исходный» and does not render HTML

#### Scenario: HTML source without stored markup renders its text

- GIVEN an entry with `format: "html"`, no `html_source`, and its text
  detected as `html`
- WHEN the entry is displayed
- THEN the viewer offers `HTML` / «Исходный» and the rendered view sanitizes
  and shows `original_text` instead of reporting an impossible state

#### Scenario: Switching the view never rewrites the source format

- GIVEN any entry whose source format is `markdown` or `html`
- WHEN the user switches between the rendered and verbatim views
- THEN `format` keeps its ingestion value and only `display_mode` changes,
  and the audio, timestamps and normalized text stay untouched
