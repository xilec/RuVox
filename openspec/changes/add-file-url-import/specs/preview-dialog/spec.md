## MODIFIED Requirements

### Requirement: Add flow gating

The Add-button flow SHALL probe both clipboard flavors before deciding:
a best-effort `navigator.clipboard.read()` for the `text/html` flavor, and
the plain text via
`tauri-plugin-clipboard-manager::readText()` (the only clipboard path that
works reliably on Wayland/KDE Plasma 6; WebKit's `navigator.clipboard` is
permission-gated, while on WebView2/Chromium it succeeds after a one-time
permission grant). Both reads are best-effort; the plain result is *used*
only when no HTML flavor exists or — on the direct path — when HTML
extraction yields no readable text.

When `config.preview_dialog_enabled` is `true` (the default in
`storage::schema::UIConfig::default`), the system SHALL open `PreviewDialog`
for **either** flavor — HTML content SHALL NOT bypass the dialog:

- HTML flavor present → the dialog opens pre-filled with the raw HTML markup
  and the source-format selector initialized to `html`.
- Only plain text present → the dialog opens pre-filled with the plain text
  and the selector initialized from `UIConfig.text_format`.
- Neither → no dialog; a neutral blue «Буфер обмена пуст» hint is shown.

When `preview_dialog_enabled` is `false`, no dialog opens and the flow is
the direct ingestion path: HTML flavor → HTML ingestion (plain fallback when
extraction yields no readable text), otherwise plain-text `addTextEntry`.

An empty clipboard or a clipboard read failure SHALL surface the neutral
«Буфер обмена пуст» hint, not an error notification (on Windows an empty
clipboard surfaces as a read error from the plugin).

`AppShell` SHALL load `UIConfig` once per mount for this decision and treat
a config load failure as "dialog disabled".

The same gating decision SHALL apply to every import entry point (drag &
drop, «Файл…», «Файл с кодировкой…», «По ссылке…»): with the gate enabled,
the imported source opens the `PreviewDialog` pre-filled with its decoded
text or fetched markup — it SHALL NOT create an entry directly. Import
failures that happen before any text exists (undecodable file, fetch error,
SPA shell) SHALL surface their own error notifications instead of opening
the dialog.

#### Scenario: Dialog opens for HTML clipboard content

- GIVEN `preview_dialog_enabled` is `true` and the clipboard holds
  `text/html` copied from a browser
- WHEN the user clicks Add
- THEN the preview dialog opens pre-filled with the raw HTML markup, the
  source-format selector is set to `html`, and no queue entry is created yet

#### Scenario: Dialog opens when enabled

- GIVEN `preview_dialog_enabled` is `true` and the clipboard contains only
  plain text
- WHEN the user clicks Add
- THEN the preview dialog opens pre-filled with the clipboard text and no
  queue entry is created yet

#### Scenario: Direct add when disabled

- GIVEN `preview_dialog_enabled` is `false`
- WHEN the user clicks Add
- THEN no dialog opens: HTML content is ingested through the HTML path,
  plain text goes to `commands.addTextEntry(text, true)` directly

#### Scenario: Empty clipboard is a hint, not an error

- GIVEN the clipboard is empty (or the read fails)
- WHEN the user clicks Add
- THEN a neutral blue «Буфер обмена пуст» notification is shown and nothing
  else happens

#### Scenario: Unreadable HTML with no plain text is the same hint

- GIVEN `preview_dialog_enabled` is `false`, the clipboard holds HTML markup
  that yields no readable text, and no plain-text flavor
- WHEN the user clicks Add
- THEN a neutral blue «Буфер обмена пуст» notification is shown and no
  entry is created

#### Scenario: Dropped file respects the gate

- GIVEN `preview_dialog_enabled` is `true`
- WHEN the user drops a `.txt` file onto the window
- THEN the preview dialog opens pre-filled with the decoded text and no
  entry is created until confirmation

#### Scenario: Dropped file ingests directly when disabled

- GIVEN `preview_dialog_enabled` is `false`
- WHEN the user drops a `.txt` file onto the window
- THEN no dialog opens and an entry is created from the decoded text at once

#### Scenario: Failed import never opens the dialog

- GIVEN `preview_dialog_enabled` is `true`
- WHEN the user imports a URL that responds with HTTP 403
- THEN the localized error notification is shown, no dialog opens, and no
  entry is created
