# Text Display Specification

## Purpose

Covers how the selected entry's original text is displayed in the
`TextViewer`: the three display modes (plain, markdown, HTML), markdown
rendering with word-position spans, HTML sanitization, and mermaid diagram
rendering including theme sync and click-to-zoom.
## Requirements
### Requirement: Display mode switching

The `TextViewer` SHALL derive the offered views from the entry's source
format and SHALL NOT offer a view that reinterprets the text as another
format. The source format is resolved as follows: a stored `html_source`
means `html`; an explicit `format` of `markdown` or `plain` is used as
recorded; a `null` format (older builds, the tray ingestion path) and a
stale `format: "html"` without stored markup fall back to content
detection (`detectFormat(original_text)`). A stale `format` that contradicts
the derived source (the old viewer toggle wrote its display choice into
`format`) SHALL be clamped at render time; no data migration is performed.

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

The rendered HTML view SHALL use the stored `html_source`. A legacy entry
whose HTML source was inferred from content has no stored markup and SHALL
render its `original_text` as the fallback. A plain source SHALL render
`original_text` verbatim even when a stale `display_mode: "rendered"` is
stored.

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

