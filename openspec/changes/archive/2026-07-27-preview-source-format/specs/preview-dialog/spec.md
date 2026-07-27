# Delta: preview-dialog

## ADDED Requirements

### Requirement: Source format selection

The dialog SHALL provide a source-format selector with the values `plain`,
`markdown`, and `html`, initialized from `UIConfig.text_format` (the
configured viewer default). The selector controls how the text is
interpreted when "Синтезировать" is pressed:

- `plain` / `markdown`: the (original or edited) text is ingested unchanged
  and the chosen value SHALL be persisted as the entry's `format`.
- `html`: the text SHALL be treated as HTML markup — sanitized and
  extracted on the frontend (`sanitizeHtml` + `extractTextForTts`) — and
  the entry SHALL be created with `format: "html"`, the extracted text as
  `original_text`, and the sanitized markup as `html_source`. If extraction
  yields no readable text, the system SHALL reject ingestion with a red
  error notification and create no entry.

The right preview pane SHALL reflect the selection: with `html` it shows
the normalization of the extracted text (what will actually be narrated);
with `plain` / `markdown` it shows the normalization of the text as-is.
Changing the selector SHALL re-trigger the debounced preview.

#### Scenario: HTML markup picked explicitly

- GIVEN the dialog is open with raw HTML markup as text and `html` selected
- WHEN the user presses "Синтезировать"
- THEN the entry is created with `format: "html"`, extracted plain text as
  `original_text`, and the sanitized markup as `html_source`, and synthesis
  narrates the extracted text (no tags or attributes)

#### Scenario: HTML choice with no extractable text

- GIVEN the dialog text is markup that yields no readable text (e.g. only
  excluded elements) and `html` is selected
- WHEN the user presses "Синтезировать"
- THEN a red error notification is shown and no entry is created

#### Scenario: Markdown choice persists display format

- GIVEN the dialog is open and `markdown` is selected
- WHEN the user presses "Синтезировать"
- THEN the entry is created with the text unchanged and `format:
  "markdown"` persisted, so the viewer renders it in markdown mode

#### Scenario: Preview follows the selector

- GIVEN the dialog is open with raw HTML markup as text
- WHEN the user switches the selector from `markdown` to `html`
- THEN the right pane updates (after the debounce) to the normalization of
  the text extracted from the markup instead of the normalization of the
  raw markup
