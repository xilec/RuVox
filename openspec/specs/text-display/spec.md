# Text Display Specification

## Purpose

Covers how the selected entry's original text is displayed in the
`TextViewer`: the three display modes (plain, markdown, HTML), markdown
rendering with word-position spans, HTML sanitization, and mermaid diagram
rendering including theme sync and click-to-zoom.

## Requirements

### Requirement: Display mode switching

The `TextViewer` SHALL provide a `SegmentedControl` with the modes `Plain`,
`Markdown`, and `HTML`, defaulting to `Markdown`. Switching modes SHALL
re-render the same original text instantly. The selected mode is ephemeral
client-side state and is NOT persisted with the entry.

#### Scenario: Switch display mode

- GIVEN an entry is displayed in Markdown mode
- WHEN the user selects `Plain` in the `SegmentedControl`
- THEN the original text is re-rendered as-is without any IPC round-trip

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

In `html` mode the system SHALL render the text as HTML sanitized through
`DOMPurify` before insertion into the DOM. Word highlighting is NOT
available in HTML mode: position events MUST be ignored while the mode is
active.

#### Scenario: Sanitized rendering

- GIVEN an entry containing HTML copied from a browser with a `<script>` tag
- WHEN the viewer is in HTML mode
- THEN the HTML is rendered without the script content, sanitized by
  `DOMPurify`

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
