# HTML Ingestion Specification

## Purpose

Covers the frontend HTML ingestion path (`src/lib/`): detecting HTML on the
clipboard, sanitizing it with DOMPurify, extracting TTS text with word-level
offset spans via a single DOM walker shared with rendering, and creating the
queue entry with `format: "html"` and a stored `html_source`.

## Requirements

### Requirement: HTML clipboard detection

When the user pastes into the main window (`Ctrl+V`), the system SHALL read
the `text/html` flavor from the paste event's clipboard data. If it is
non-empty, the entry SHALL be created through the HTML ingestion path;
otherwise the system SHALL use the plain-text flavor and the existing plain
ingestion path. The Add-button flow SHALL make a best-effort attempt to read
`text/html` from the system clipboard and SHALL fall back to the existing
plugin `readText` path on any failure. The tray "Add" path SHALL keep using
the plain-text flavor only.

#### Scenario: Paste with HTML flavor
- GIVEN the clipboard holds `text/html` copied from a browser
- WHEN the user presses `Ctrl+V` in the main window
- THEN an entry is created through the HTML ingestion path with `format: "html"`

#### Scenario: Paste without HTML flavor
- GIVEN the clipboard holds only plain text
- WHEN the user presses `Ctrl+V` in the main window
- THEN the entry is created exactly as by the existing plain-text path

#### Scenario: Add button without HTML access
- GIVEN the webview cannot read `text/html` from the system clipboard
- WHEN the user clicks the Add button
- THEN the entry is created from the plugin `readText` result as today

### Requirement: Sanitization before extraction

Ingested HTML SHALL be sanitized with DOMPurify using the existing viewer
configuration before any further processing. The sanitized markup SHALL be
stored as the entry's `html_source`, and extraction SHALL run on the
sanitized document, so the stored HTML, the extraction input, and the
rendered document are the same content.

#### Scenario: Hostile markup is neutralized
- GIVEN clipboard HTML containing a `<script>` tag and an `onclick` attribute
- WHEN the entry is ingested
- THEN `html_source` contains neither the script nor the event handler, and the extracted text contains no script content

### Requirement: Text extraction with word spans

The system SHALL extract TTS text from the sanitized DOM with a single
walker implementation shared by ingestion and rendering. The walker SHALL:

- exclude the subtrees of `nav`, `footer`, `aside`, `script`, `style`,
  `head`, `noscript`, `template`, `svg`, `math`, `button`, `select`,
  `option`, `optgroup`, and `datalist`;
- separate block-level elements from surrounding text with newlines and
  emit a newline for `<br>` / `<hr>`;
- collapse inline whitespace (treating NBSP as a regular space) and trim
  the result;
- assign every word a codepoint offset range in the extracted text,
  renderable as `<span data-orig-start data-orig-end>` elements.

The extracted text SHALL become the entry's `original_text`, so the TTS
pipeline, char-mapping, and word timestamps all refer to it unchanged.

#### Scenario: Paragraphs and code
- GIVEN sanitized HTML `<p>Вызови <code>getUserData()</code> через <b>API</b></p>`
- WHEN extraction runs
- THEN the extracted text is `Вызови getUserData() через API` and the word `API` carries offsets `[27, 30)` in that text

#### Scenario: Chrome elements excluded
- GIVEN sanitized HTML with a `<nav>` menu, a `<button>`, and a `<p>` paragraph
- WHEN extraction runs
- THEN the extracted text contains only the paragraph content

#### Scenario: Block structure becomes newlines
- GIVEN sanitized HTML with two consecutive `<p>` elements
- WHEN extraction runs
- THEN the two paragraphs appear on separate lines in the extracted text

### Requirement: HTML entry creation

The HTML ingestion path SHALL create the entry with `original_text` set to
the extracted text, `format` set to `"html"`, and `html_source` set to the
sanitized HTML, and SHALL start background synthesis exactly like the plain
path. An extraction that yields only whitespace SHALL be rejected like
blank plain text.

#### Scenario: HTML entry is stored and synthesized
- GIVEN HTML ingested from the clipboard
- WHEN the entry is created
- THEN it is persisted with `format: "html"`, non-empty `html_source`, extracted `original_text`, status `pending`, and background synthesis starts
