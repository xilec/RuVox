# Delta: html-ingestion

## MODIFIED Requirements

### Requirement: HTML clipboard detection

When the user pastes into the main window (`Ctrl+V`), the system SHALL read
the `text/html` flavor from the paste event's clipboard data. If it is
non-empty, the entry SHALL be created through the HTML ingestion path;
otherwise the system SHALL use the plain-text flavor and the existing plain
ingestion path. The tray "Add" path SHALL keep using the plain-text flavor
only.

The Add-button flow SHALL make a best-effort attempt to read `text/html`
from the system clipboard. What happens next depends on the preview gate
(`preview_dialog_enabled`, see the preview-dialog spec):

- Preview **disabled**: a non-empty HTML flavor SHALL be ingested through
  the HTML path directly (plain-text fallback when extraction yields no
  readable text); otherwise the plugin `readText` result is used as today.
- Preview **enabled**: the HTML flavor SHALL NOT create an entry directly —
  the raw markup is handed to the preview dialog with the source-format
  selector pre-set to `html`, and ingestion happens on dialog confirmation.

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

#### Scenario: Add button with HTML flavor and preview enabled
- GIVEN the clipboard holds `text/html` and `preview_dialog_enabled` is `true`
- WHEN the user clicks the Add button
- THEN no entry is created yet; the preview dialog opens with the raw HTML
  markup and the source-format selector set to `html`
