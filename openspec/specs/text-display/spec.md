# Text Display Specification

## Purpose

Covers how the selected entry's original text is displayed in the
`TextViewer`: the rendered view of the entry's source format (markdown or
HTML) and the verbatim view («Исходный»), markdown rendering with
word-position spans, HTML sanitization, and mermaid diagram rendering
including theme sync and click-to-zoom. The source format is resolved when
the entry is ingested and clamped at render time.
## Requirements
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

### Requirement: Plain text mode

In `plain` mode the system SHALL show the original text unmodified (markdown
markup such as `**bold**` or `# Header` stays visible), preserving line
breaks. Every word SHALL be wrapped in a `<span data-orig-start
data-orig-end>` element carrying its character offsets in the original text,
so word highlighting works in plain mode.

#### Scenario: Markdown source shown verbatim

- GIVEN an entry whose text contains `## Установка` and `**bold**`
- WHEN the viewer is in plain mode
- THEN the markup characters are shown literally and each word span carries
  its original offsets

### Requirement: Markdown mode

In `markdown` mode the system SHALL render the text with `markdown-it`,
including headings, bold/italic, inline code, code blocks with syntax
highlighting, links, lists, and tables. Inline text tokens SHALL be wrapped
in `<span data-orig-start data-orig-end>` elements so playback word
highlighting can map timestamps onto the rendered HTML.

#### Scenario: Render markdown

- GIVEN an entry with markdown headings, a list, and a code block
- WHEN the viewer is in markdown mode
- THEN the text is rendered as formatted HTML and inline words carry
  `data-orig-*` offsets

### Requirement: HTML mode

In `html` mode the system SHALL render the entry's `html_source` (falling
back to `original_text` when `html_source` is null) as HTML sanitized
through `DOMPurify` before insertion into the DOM. Every word SHALL be
wrapped in a `<span data-orig-start data-orig-end>` element carrying its
codepoint offsets in the extracted text (`original_text`), produced by the
same extraction walker that generated the TTS text — so playback word
highlighting works in HTML mode.

#### Scenario: Sanitized rendering

- GIVEN an entry containing HTML copied from a browser with a `<script>` tag
- WHEN the viewer is in HTML mode
- THEN the HTML is rendered without the script content, sanitized by
  `DOMPurify`

#### Scenario: HTML mode renders stored source with word spans
- GIVEN an HTML-ingested entry with `html_source`
- WHEN the viewer is in HTML mode
- THEN the rendered words carry `data-orig-*` offsets matching the extracted text, and word highlighting follows playback

### Requirement: Mermaid diagram rendering

In `markdown` mode, fenced ` ```mermaid ` code blocks SHALL be rendered as
inline SVG diagrams via `mermaid.js` instead of syntax-highlighted code. The
mermaid theme SHALL follow the current color scheme (`dark` theme in dark
mode, `default` otherwise) and re-render when the scheme changes. If the
diagram source is invalid, the system SHALL keep the raw block visible and
log the error instead of breaking the view. In `plain` mode mermaid blocks
SHALL be shown as source code.

#### Scenario: Render a mermaid block

- GIVEN an entry containing a fenced mermaid block
- WHEN the viewer is in markdown mode
- THEN the block is rendered as an SVG diagram matching the current
  light/dark color scheme

#### Scenario: Invalid mermaid source

- GIVEN an entry with a mermaid block that has a syntax error
- WHEN the viewer renders it
- THEN the raw diagram source stays visible and the rest of the document
  renders normally

### Requirement: Mermaid click-to-zoom

Clicking a rendered mermaid SVG SHALL open a modal dialog titled "Mermaid
diagram" showing the diagram enlarged with horizontal scrolling when needed.

#### Scenario: Zoom a diagram

- GIVEN a rendered mermaid diagram in the viewer
- WHEN the user clicks it
- THEN a modal opens displaying the diagram at a larger size

### Requirement: Select-all inside the viewer

While focus or the selection is inside the viewer, `Ctrl+A` / `Cmd+A` SHALL
select only the rendered text content, not the whole window. When the user
is typing in an input, textarea, or contentEditable element, the default
browser behavior SHALL apply.

#### Scenario: Select rendered text only

- GIVEN the text cursor or selection is inside the viewer content
- WHEN the user presses `Ctrl+A`
- THEN only the viewer's rendered text becomes selected

### Requirement: Read-only rendered content

In every display mode (plain, markdown, HTML) the rendered content SHALL be
inert: clicking a link (`<a>`) inside the viewer SHALL NOT navigate the
webview or trigger any external handler — the click SHALL be intercepted and
its default action prevented. The link's original `href` (verbatim, as in the
source markup) SHALL be visible on hover (tooltip); resolving it against the
webview origin would yield a meaningless localhost URL for relative links.
Interactive elements in rendered content SHALL be inoperable: buttons,
selects, textareas and inputs SHALL be disabled; `<video>`/`<audio>` SHALL
render without media controls; `<details>` SHALL not toggle. Neutralization
SHALL NOT remove the elements from the rendered output and SHALL NOT rely on
sanitization changes that alter the stored `html_source`.

#### Scenario: Click on a link does not navigate

- WHEN the user clicks a link inside the viewer in any display mode
- THEN the webview stays on the application and no navigation or external
  open occurs

#### Scenario: Link URL visible on hover

- GIVEN rendered content containing `<a href="/ru/users/maybe_elf/">`
- WHEN the user hovers the link
- THEN a tooltip shows the original href `/ru/users/maybe_elf/` verbatim

#### Scenario: Form controls are disabled

- GIVEN rendered content containing a `<button>` and a `<select>`
- WHEN the content is displayed
- THEN both controls are disabled and cannot be operated

#### Scenario: Media renders without controls

- GIVEN rendered content containing `<video controls src="...">`
- WHEN the content is displayed
- THEN the video element renders without playback controls

#### Scenario: Mermaid click-to-zoom still works

- WHEN the user clicks a rendered mermaid diagram
- THEN the zoom modal opens (link interception does not affect non-link
  clicks)

### Requirement: Viewer entry header

The viewer SHALL render an entry header above the scrollable reading area, so
the entry's identity and metadata stay visible while the text scrolls. The
header SHALL contain, from top to bottom:

- the title line;
- the note line;
- a metadata row with the date added, the status badge, the engine and the
  voice of the current recording, and — at its right end — the format display
  switch when the entry's source format offers rendered views.

The title shown SHALL be the stored `title`, or — when it is `null` — the title
derived from `original_text` by the same algorithm the backend uses for
seeding, so legacy entries look identical to freshly added ones. The title line
SHALL be click-to-edit: clicking it or its pencil affordance (visible on hover
or keyboard focus) opens a single-line input seeded with the stored title and
the derived title as placeholder. `Enter` and blur SHALL save, `Esc` SHALL
cancel without persisting, and an empty or whitespace-only value SHALL store
`null` and render the derived fallback again instead of an empty line. Saving
SHALL be optimistic: the new value renders immediately and a rejected IPC call
reverts it and shows the localized red error toast.

The note SHALL render dimmed under the title, clamped to two lines with an
expand/collapse toggle when the text overflows the clamp. An entry without a
note SHALL show the «Добавить заметку» affordance (EN: "Add a note"). Editing
SHALL use an autosize textarea with `Ctrl+Enter`/Save committing, `Esc`/Cancel
discarding, and the same optimistic-write-with-rollback rule as the title. The
note is plain text with newlines; it SHALL NOT be rendered as Markdown/HTML and
SHALL NOT be sent to TTS.

An incoming `entry_updated` for the displayed entry (for example a status
change or a synthesis completion) SHALL NOT discard an open title or note draft.
The draft is applied or cancelled only by the user's own commit/cancel action.
A successful persisted update SHALL be reflected in the header; when no draft
is open, an externally changed note or title SHALL render the new value.

The metadata row SHALL show the date added as the local-time rendering of the
naive UTC `created_at` in the app language, a localized status badge, and the
engine and voice from the `generation` snapshot. The engine and voice labels
SHALL be engine-specific: Piper voices by their localized names, Silero
speakers by their speaker name, TeraTTS voices by their stored id, and an
unknown id by the stored id. An entry without a `generation` snapshot SHALL
show the explicit placeholders «не озвучено» for the engine and «—» for the
voice rather than an empty or guessed value.

The format `SegmentedControl` SHALL move from the old toolbar into the right end
of the metadata row; it SHALL NOT be rendered for a plain source that offers no
rendered view. The reading-typography controls (#354/#357) SHALL remain next to
it in the viewer header and SHALL stay available in every display mode,
including a plain source.

#### Scenario: Legacy entry renders the derived title

- **GIVEN** an entry with `title: null` and `original_text` starting with `## Установка`
- **WHEN** the viewer renders the header
- **THEN** the title line shows `Установка` and no explicit title is stored

#### Scenario: Title editing is keyboard-complete

- **GIVEN** an entry displayed in the viewer
- **WHEN** the user opens the title editor, types a value and presses `Enter`
- **THEN** `set_entry_title` persists the trimmed value, the header shows it
  immediately, and pressing `Esc` instead leaves the stored title untouched

#### Scenario: Clearing the title returns to the derived fallback

- **GIVEN** an entry with a stored title
- **WHEN** the user opens the editor, clears the input and saves
- **THEN** `null` is persisted and the header shows the derived title

#### Scenario: Note editing and clearing

- **GIVEN** an entry without a note
- **WHEN** the user clicks «Добавить заметку», types text and presses `Ctrl+Enter`
- **THEN** `set_entry_note` persists the trimmed note and the header shows the
  clamped note; clearing the textarea and saving removes it and restores the
  «Добавить заметку» state

#### Scenario: Draft survives an entry update

- **GIVEN** the note editor is open with an unsaved draft
- **WHEN** an `entry_updated` event changes the entry's status
- **THEN** the draft text and the open editor are preserved

#### Scenario: Failed save rolls back with a toast

- **GIVEN** an entry whose title is `"Старое"`
- **WHEN** the user saves a new title and the IPC call rejects
- **THEN** the header shows `"Старое"` again and a red localized error toast is
  shown

#### Scenario: Metadata row for an entry without audio

- **GIVEN** an entry whose `generation` is `null`
- **WHEN** the header renders
- **THEN** the metadata row shows the local date added, the status badge,
  «не озвучено» and «—»

#### Scenario: Metadata row for a recorded entry

- **GIVEN** a ready entry whose `generation` records engine `silero_native` and
  voice `xenia`
- **WHEN** the header renders
- **THEN** the metadata row shows `Готово`, `Silero (нативный)` and `Xenia`

#### Scenario: Reading controls stay available after the switch moves

- **GIVEN** a plain-text entry with no display switch
- **WHEN** the viewer renders
- **THEN** the reading width and scale controls are present in the header and
  the format switch is absent

