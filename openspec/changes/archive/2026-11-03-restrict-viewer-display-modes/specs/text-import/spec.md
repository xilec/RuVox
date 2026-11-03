# Delta: text-import

## MODIFIED Requirements

### Requirement: Import format routing

For imported files the extension SHALL decide the source format: `.md` →
markdown, `.html`/`.htm` → the HTML extraction path, `.txt` → plain text.
For URLs — and for file contents that contradict their extension (an
unreadable `.html`) — the system SHALL fall back to format auto-detection
over the raw text. Routed content SHALL reuse the existing ingest decision
layer and its sanitize/extract machinery.

The routed (or detected) format SHALL be persisted as the entry's immutable
source `format` at ingestion. The viewer derives its allowed views from it
and SHALL NOT rewrite it when the user switches between the rendered and
verbatim views (text-display spec, "Display mode switching"); the viewer's
choice is stored separately in `display_mode`.

#### Scenario: Extension decides for files

- WHEN the user imports `notes.md`
- THEN the entry is created with format markdown without consulting content
  sniffing

#### Scenario: URL falls back to detection

- WHEN the user imports a URL whose response is plain text served as
  `text/plain`
- THEN auto-detection classifies it and the entry is created with the
  detected format

#### Scenario: Routed format survives the viewer switch

- GIVEN a file imported as markdown and created with `format: "markdown"`
- WHEN the user switches the viewer to the verbatim view
- THEN `set_entry_display_mode` stores `display_mode: "plain"` and the entry
  still has `format: "markdown"`
