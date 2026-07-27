# Delta: text-display

## MODIFIED Requirements

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
- THEN the HTML is rendered without the script content, sanitized by `DOMPurify`

#### Scenario: HTML mode renders stored source with word spans
- GIVEN an HTML-ingested entry with `html_source`
- WHEN the viewer is in HTML mode
- THEN the rendered words carry `data-orig-*` offsets matching the extracted text, and word highlighting follows playback
