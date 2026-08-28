## ADDED Requirements

### Requirement: Source format auto-detection

The system SHALL classify text into a source format (`plain`, `markdown`, or
`html`) from content signals only, without any configuration or user input:

- `html` — when the trimmed text starts with a `<!DOCTYPE html` or `<html`
  prefix (case-insensitive), or when it contains **three or more** well-formed
  tag fragments (an angle-bracket construct that opens with `<` or `</`
  followed by a letter, may carry attributes, and closes with `>`).
- `markdown` — when the text carries at least one **strong structural signal**:
  an ATX heading line (`#`–`######` followed by a space), a fenced code block
  delimiter (``` ``` ```` or `~~~`) on its own line, three or more list-item
  lines (starting with `-`, `*`, `+`, or a numbered `1.`-style marker), or two
  or more inline links (`[text](target)`).
- `plain` — otherwise, and always for empty or whitespace-only text.

The classification SHALL be conservative on the `html` side, because reading
markup aloud is the costlier mistake than under-detecting it: technical prose
with angle brackets (`a < b`, `x -> y`, C++ includes), single generic
parameters (`<T>`), or one stray tag-looking fragment in an otherwise plain
text SHALL NOT classify as `html`.

#### Scenario: Full HTML document is detected

- WHEN the text starts with `<!DOCTYPE html>` (or `<html`) and contains markup
- THEN the detected format is `html`

#### Scenario: Markup fragment with several tags is detected

- GIVEN the text is not a full document but carries three or more well-formed
  tags (e.g. `<p>Первый</p><p>Второй</p><b>третий</b>`)
- WHEN the format is detected
- THEN the detected format is `html`

#### Scenario: Angle-bracket prose stays plain

- GIVEN the text is technical prose such as `if a < b && c > d` or
  `Vec<T> get_user_data()`
- WHEN the format is detected
- THEN the detected format is `plain` — the fragments are not tags and fewer
  than three tag-like constructs exist

#### Scenario: Single stray tag-looking fragment stays plain

- GIVEN a plain paragraph that contains exactly one tag-looking fragment
  (e.g. `<cmath>` in `подключите <cmath> для std::sqrt`)
- WHEN the format is detected
- THEN the detected format is `plain`

#### Scenario: Markdown structural signals are detected

- GIVEN a text with an ATX heading, or a fenced code block, or three or more
  list-item lines, or two or more inline links
- WHEN the format is detected
- THEN the detected format is `markdown`

#### Scenario: Sparse markdown-looking decoration stays plain

- GIVEN a plain paragraph where a single line happens to start with `-` and no
  other structural signal exists
- WHEN the format is detected
- THEN the detected format is `plain`

#### Scenario: Empty text classifies as plain

- GIVEN the text is empty or whitespace-only
- WHEN the format is detected
- THEN the detected format is `plain`

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
  and the source-format selector initialized to the auto mode; the effective
  source format is the detected one (`html` for clipboard markup).
- Only plain text present → the dialog opens pre-filled with the plain text
  and the selector initialized to the auto mode; the effective source format
  is the detected one. `UIConfig.text_format` no longer drives the dialog's
  initial selector state (it remains the viewer's display default).
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
  source-format selector is set to the auto mode showing the detected `html`,
  and no queue entry is created yet

#### Scenario: Dialog opens when enabled

- GIVEN `preview_dialog_enabled` is `true` and the clipboard contains only
  plain text
- WHEN the user clicks Add
- THEN the preview dialog opens pre-filled with the clipboard text, the
  source-format selector is set to the auto mode, and no queue entry is
  created yet

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

### Requirement: Source format selection

The dialog SHALL provide a source-format selector with the values `auto`
(the default), `plain`, `markdown`, and `html`. The `auto` value SHALL be
selected when the dialog opens from a clipboard flow; an opening from an
import SHALL preselect the routed format instead (text-import spec,
"Import format routing"), and every opening allows switching to any value
including the auto mode. The `auto` label SHALL show the format currently
detected for the text under consideration (e.g. «Авто (HTML)»), and the
detection SHALL re-run whenever the text changes — including while the user
edits it.

The effective source format is the explicitly chosen value when the selector
is not in the auto mode, and the detected format when it is. The effective
format controls both the right preview pane and what happens when
"Синтезировать" is pressed:

- `plain` / `markdown`: the (original or edited) text is ingested unchanged
  and that value SHALL be persisted as the entry's `format`.
- `html`: the text SHALL be treated as HTML markup — sanitized and
  extracted on the frontend (`sanitizeHtml` + `extractTextForTts`) — and
  the entry SHALL be created with `format: "html"`, the extracted text as
  `original_text`, and the sanitized markup as `html_source`. If extraction
  yields no readable text, the outcome depends on how the `html` effective
  format came about:
  - the dialog was opened from an **auto-detected** HTML clipboard flavor
    and a plain flavor was carried along → the system SHALL fall back to
    ingesting the plain text (the same rule as the ungated direct path);
  - otherwise (an explicit selector choice, or no plain flavor on the
    clipboard) → the system SHALL reject ingestion with a red error
    notification and create no entry.

The right preview pane SHALL reflect the effective format: with `html` it
shows the normalization of the extracted text (what will actually be
narrated); with `plain` / `markdown` it shows the normalization of the text
as-is. Changing the selector or — in the auto mode — the text SHALL
re-trigger the debounced preview.

#### Scenario: Auto is the default and shows the detection

- GIVEN the dialog opens with plain technical prose
- WHEN the user looks at the source-format selector
- THEN it shows the auto mode with the detected format in its label
  (e.g. «Авто (Plain)»), and no explicit format is selected

#### Scenario: Auto re-detects after an edit

- GIVEN the dialog is open in edit mode with the auto mode selected and
  plain text
- WHEN the user replaces the text with an HTML fragment carrying three or
  more tags
- THEN the selector's label switches to the detected `html` and the right
  pane (after the debounce) shows the normalization of the extracted text

#### Scenario: Explicit choice overrides detection

- GIVEN the dialog is open with text detected as `markdown`
- WHEN the user explicitly selects `plain` and presses "Синтезировать"
- THEN the text is ingested unchanged with `format: "plain"` — detection
  plays no further role

#### Scenario: HTML markup picked explicitly

- GIVEN the dialog is open with raw HTML markup as text and `html` selected
- WHEN the user presses "Синтезировать"
- THEN the entry is created with `format: "html"`, extracted plain text as
  `original_text`, and the sanitized markup as `html_source`, and synthesis
  narrates the extracted text (no tags or attributes)

#### Scenario: HTML choice with no extractable text

- GIVEN the dialog was opened with plain text, the user explicitly selected
  `html`, and the text is markup that yields no readable text (e.g. only
  excluded elements)
- WHEN the user presses "Синтезировать"
- THEN a red error notification is shown and no entry is created

#### Scenario: Auto-detected HTML with no extractable text falls back to plain

- GIVEN the dialog was opened from an auto-detected HTML clipboard flavor
  (effective format `html`, plain flavor carried), and the markup is chrome
  that yields no readable text while the plain flavor has content
- WHEN the user presses "Синтезировать" without changing the selector
- THEN the plain clipboard text is ingested as a normal entry and no error
  is shown

#### Scenario: Markdown choice persists display format

- GIVEN the dialog is open and `markdown` is the effective format
- WHEN the user presses "Синтезировать"
- THEN the entry is created with the text unchanged and `format:
  "markdown"` persisted, so the viewer renders it in markdown mode

#### Scenario: Preview follows the selector

- GIVEN the dialog is open with raw HTML markup as text
- WHEN the user switches the selector from `markdown` to `html`
- THEN the right pane updates (after the debounce) to the normalization of
  the text extracted from the markup instead of the normalization of
  the raw markup
