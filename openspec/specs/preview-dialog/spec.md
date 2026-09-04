# Preview Dialog Specification

## Purpose

Covers the normalization preview dialog (FF 1.1): a floating, non-modal window (`src/dialogs/PreviewDialog.tsx`) that shows the clipboard text and its normalized form side by side before synthesis, lets the user edit the source text with live re-normalization, and only then creates a queue entry. Also covers the `preview_normalize` backend command and the `preview_dialog_enabled` configuration gate that controls whether the dialog appears in the Add flow.

## Requirements

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
  is the detected one. `UIConfig.text_format` is no longer consumed by the
  dialog — and, after this change, by any frontend code at all; the field
  stays in the schema untouched.
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

### Requirement: Floating non-modal window

The system SHALL implement the dialog as a floating, non-modal window via `react-rnd` inside a Mantine `Portal`, so it sits above the app (z-index `--ruvox-preview-z`, below Mantine notifications) without blocking the UI underneath.

The `Rnd` element SHALL be anchored inside a viewport-sized fixed container (`.viewportContainer`, `position: fixed; inset: 0; pointer-events: none`) so that its `(x, y)` coordinates are viewport coordinates regardless of document scroll. The window SHALL be draggable by its header ("Предпросмотр нормализации") and resizable from any edge or corner, with a minimum size of 560×380, an initial size of 900×620, centered on every open.

Pressing ESC SHALL close the dialog, equivalent to Cancel. The header close button SHALL behave the same way.

#### Scenario: Geometry resets on each open

- GIVEN the user previously moved and resized the dialog, then closed it
- WHEN the dialog is opened again
- THEN it reappears centered at 900×620, with edit mode off, "Read Now" on, and both checkboxes unchecked

#### Scenario: ESC cancels

- GIVEN the dialog is open
- WHEN the user presses ESC
- THEN the dialog closes and no entry is added to the queue

#### Scenario: Underlying UI stays interactive

- GIVEN the dialog is open over the main window
- WHEN the user clicks outside the dialog panel
- THEN the click reaches the underlying application (the container is click-through; only the panel itself captures pointer events)

### Requirement: Side-by-side panes with live normalization

The dialog body SHALL show two panes: "Оригинал" (left) and "После нормализации" (right).

The left pane SHALL be a read-only scrollable `<pre>` by default; clicking "Редактировать" switches it to a `<Textarea>` with the current text. Every change to the text under consideration SHALL trigger re-normalization after a 1000 ms debounce.

The right pane SHALL show the normalized result from `commands.previewNormalize(text)`; while a normalization request is in flight it SHALL show a `<Loader>`, and on failure it SHALL show `"(ошибка нормализации: ...)"` inline. When the text is empty or whitespace-only, the right pane SHALL be empty and no request SHALL be issued.

#### Scenario: Edit re-normalizes with debounce

- GIVEN the dialog is open in edit mode
- WHEN the user types into the textarea and pauses for 1 second
- THEN `commands.previewNormalize` is called once with the edited text and the right pane updates with the result

#### Scenario: Normalization error is shown inline

- GIVEN the dialog is open
- WHEN `commands.previewNormalize` rejects
- THEN the right pane shows `"(ошибка нормализации: <reason>)"` instead of a result

### Requirement: Footer controls

In the Add/import mode the dialog footer SHALL contain:

| Control | Behavior |
|---------|----------|
| "Больше не показывать этот диалог" (Checkbox) | On synthesis, persists `preview_dialog_enabled: false` via `commands.updateConfig` (no threshold; disables the dialog globally) |
| "Синхронный скроллинг" (Checkbox) | Mirrors scrolling between the two panes by relative position, with ping-pong protection via `syncingRef` |
| "Read Now" (Switch, default ON) | Passed as `playWhenReady` to `addTextEntry`; ON plays after `ready`, OFF only enqueues |
| "Отмена" (Button) | Closes the dialog without adding anything |
| "Редактировать" (Button) | Switches the left pane to edit mode; hidden while editing |
| "Синтезировать" (Button) | Confirms and synthesizes; disabled while normalization is loading |

In the regeneration mode (see "Regeneration preview") the footer SHALL omit
the checkbox and the «Редактировать» button — they cannot apply to an
immutable entry — and the confirm button SHALL be labeled
«Перегенерировать»; the remaining controls stay.

#### Scenario: Synchronized scrolling mirrors position

- GIVEN "Синхронный скроллинг" is checked and both panes overflow
- WHEN the user scrolls the left pane to 50% of its range
- THEN the right pane scrolls to 50% of its own range, without echoing a scroll event back

### Requirement: Synthesis confirmation

When "Синтезировать" is pressed in the Add/import mode, the system SHALL:

1. Use `editedText.trim()` when in edit mode, otherwise the original clipboard text; an empty edited result MUST fall back to the original text.
2. If "Больше не показывать этот диалог" is checked, call `commands.updateConfig({ preview_dialog_enabled: false })` and update the cached config in `AppShell`.
3. Close the dialog and call `commands.addTextEntry(finalText, playWhenReady)`, selecting the new entry and showing a confirmation notification.

The preview dialog SHALL be the only place in the UI where the user can edit source text before synthesis; after confirmation the text is stored as the entry's immutable `original_text`. The regeneration confirm is governed by "Regeneration preview" — it re-runs synthesis for the stored `original_text` instead of creating an entry.

#### Scenario: Edited text is synthesized

- GIVEN the user edited the text and left "Read Now" ON
- WHEN the user clicks "Синтезировать"
- THEN the dialog closes, `addTextEntry` receives the trimmed edited text with `playWhenReady = true`, and the new entry becomes selected

#### Scenario: Opt-out persists

- GIVEN "Больше не показывать этот диалог" is checked
- WHEN the user clicks "Синтезировать"
- THEN `commands.updateConfig({ preview_dialog_enabled: false })` is issued, and the next Add click skips the dialog

### Requirement: preview_normalize backend command

The system SHALL expose a Tauri command `preview_normalize` (`src-tauri/src/commands/mod.rs`) that runs the normalization pipeline on the given text inside `tokio::task::spawn_blocking` (the pipeline is CPU-bound and must not block the tokio reactor) and returns `PreviewNormalizeResult { normalized }`.

```rust
#[derive(Debug, Serialize)]
pub struct PreviewNormalizeResult {
    pub normalized: String,
}
```

The command MUST NOT touch storage: previewing creates no `TextEntry`, so preview drafts never appear in the queue. The character mapping returned by `process_with_char_mapping` SHALL be discarded — the dialog needs only the result string.

#### Scenario: Preview without side effects

- GIVEN the dialog requested a preview for some text
- WHEN `preview_normalize` completes
- THEN the response contains only the normalized string, and the history file and audio cache are unchanged

#### Scenario: Frontend contract

- GIVEN the frontend wrapper `commands.previewNormalize(text)` in `src/lib/tauri.ts`
- WHEN it invokes the backend
- THEN it calls `tauriInvoke('preview_normalize', { text })` and resolves to `PreviewNormalizeResult`

### Requirement: preview_dialog_enabled configuration

The system SHALL store the dialog gate as `preview_dialog_enabled: boolean` in `UIConfig`, defaulting to `true`. It SHALL be changeable in two ways: the Settings dialog toggle "Показывать диалог предпросмотра перед синтезом", and the in-dialog "Больше не показывать этот диалог" checkbox (which only ever sets it to `false`; re-enabling requires the Settings toggle).

#### Scenario: Restore via Settings

- GIVEN the dialog was disabled via its own checkbox
- WHEN the user enables "Показывать диалог предпросмотра перед синтезом" in Settings and saves
- THEN the next Add click opens the preview dialog again

### Requirement: Source format selection

The Add/import dialog SHALL provide a source-format selector with the
values `auto` (the default), `plain`, `markdown`, and `html`. The `auto`
value SHALL be selected when the dialog opens from a clipboard flow; an
opening from an import SHALL preselect the routed format instead
(text-import spec, "Import format routing"), and every opening allows
switching to any value including the auto mode. The `auto` label SHALL
show the format currently detected for the text under consideration
(e.g. «Авто (HTML)»), and the detection SHALL re-run whenever the text
changes — including while the user edits it. The regeneration dialog
SHALL NOT show the selector: its preview is the plain pipeline over the
stored `original_text` (see "Regeneration preview").

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

### Requirement: Source format auto-detection

The system SHALL classify text into a source format (`plain`, `markdown`, or
`html`) from content signals only, without any configuration or user input:

- `html` — when the trimmed text starts with a `<!DOCTYPE html` or `<html`
  prefix (case-insensitive), or when, after trimming whitespace and
  zero-width characters (`U+200B`–`U+200D`, `U+FEFF`) at both ends, it
  **starts with a well-formed tag AND ends with a well-formed tag** (an
  angle-bracket construct that opens with `<` or `</` followed by a letter,
  may carry attributes, and closes with `>`). Markup is delimited by tags:
  placeholder-like fragments buried in prose never satisfy both boundaries.
- `markdown` — when the text carries at least one **strong structural signal**:
  an ATX heading line (`#`–`######` followed by a space), a fenced code block
  delimiter (``` ``` ```` or `~~~`) on its own line, three or more list-item
  lines (starting with `-`, `*`, `+`, or a numbered `1.`-style marker), or two
  or more inline links (`[text](target)`).
- `plain` — otherwise, and always for empty or whitespace-only text.

The classification SHALL be conservative on the `html` side, because reading
markup aloud is the costlier mistake than under-detecting it: technical prose
with angle brackets (`a < b`, `x -> y`, C++ includes), single generic
parameters (`<T>`), or stray tag-looking fragments in an otherwise plain
text SHALL NOT classify as `html` — such texts do not both start and end
with a tag.

#### Scenario: Full HTML document is detected

- WHEN the text starts with `<!DOCTYPE html>` (or `<html`) and contains markup
- THEN the detected format is `html`

#### Scenario: Full HTML document stays html despite heading-like lines

- GIVEN a text starting with `<!DOCTYPE html>` whose body contains a line
  like `# notes` inside markup
- WHEN the format is detected
- THEN the detected format is `html` — the document prefix outranks the
  markdown signals

#### Scenario: Markup fragment with several tags is detected

- GIVEN the text is not a full document but both starts and ends with
  well-formed tags (e.g. `<p>Первый</p><p>Второй</p><b>третий</b>`)
- WHEN the format is detected
- THEN the detected format is `html`

#### Scenario: Bare tag-pair snippet is detected

- GIVEN a text consisting of a single tag pair with content (e.g.
  `<b>жирным</b>`)
- WHEN the format is detected
- THEN the detected format is `html` — both boundaries are tags

#### Scenario: Changelog-style prose with placeholder fragments stays markdown

- GIVEN a changelog-style document: an ATX heading (`# Changelog`), several
  list-item lines, and fragments such as `` `<type>(<module>): <desc>` ``
  and `` `<UnlistenFn>` ``
- WHEN the format is detected
- THEN the detected format is `markdown` — the text neither starts nor ends
  with a tag, so the markdown structure decides

#### Scenario: Text starting with a tag but not ending with one stays non-html

- GIVEN a text that starts with a tag-like construct but ends with ordinary
  text (e.g. `<T> get_user_data()` or an unclosed fragment
  `<p>раз\n<p>два\n<p>три`)
- WHEN the format is detected
- THEN the detected format is NOT `html` — the end boundary is not a tag

#### Scenario: Angle-bracket prose stays plain

- GIVEN the text is technical prose such as `if a < b && c > d` or
  `Vec<T> get_user_data()`
- WHEN the format is detected
- THEN the detected format is `plain`

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

### Requirement: Normalization explainer

The dialog SHALL show, on every open, a short explainer line (one–two
sentences, localized) between the header and the panes stating in user terms
what normalization is and what the two panes are: RuVox rewrites technical
text (English identifiers, abbreviations, numbers, URLs, operators, code) so
the speech engine can narrate it in Russian; the left pane is the source and
the right pane is what will actually be spoken. The line SHALL NOT displace
or obscure any existing control; the dialog's minimum size and layout stay
as specified elsewhere.

The header SHALL also carry a small help affordance (an icon button) with a
click-toggled popover containing the fuller explanation: what categories get
rewritten, what the source-format selector (Авто / Обычный текст / Markdown / HTML) controls, and that fenced code block narration follows the code
block narration setting from Settings — «Кратко» replaces each block with a
brief marker sentence («далее следует пример кода на <язык>»), «Читать
полностью» reads identifiers and operators out loud; Mermaid blocks always
become the «Тут мермэйд диаграмма» marker. The popover copy SHALL NOT
mention any in-text directives. The popover SHALL include a link that opens
the README's normalization section in the system browser. The affordance
MUST NOT intercept the header's drag behavior: dragging by the icon area is
not required, but clicking it SHALL toggle the popover, not move or resize
the window, and the icon SHALL expose an accessible name (aria-label).

Both the explainer line and the tooltip copy SHALL come from the i18n
dictionaries (`preview.explain.*` keys), Russian and English.

#### Scenario: Explainer is visible on open

- GIVEN `preview_dialog_enabled` is `true` and the user opens the Add flow
- WHEN the preview dialog appears
- THEN a short explainer line about what normalization does is visible
  between the header and the panes, and the existing controls are all
  reachable

#### Scenario: Help affordance shows details and opens the README

- GIVEN the preview dialog is open
- WHEN the user activates the header help icon and then activates the link in
  its popover
- THEN a fuller explanation (rewritten categories, source-format selector,
  code-block narration) is shown, and the system browser opens the README's
  normalization section; the dialog state is unchanged

#### Scenario: Copy is localized

- GIVEN the UI language is `ru` or `en`
- WHEN the dialog is open
- THEN both the explainer line and the popover text are rendered in the
  active language from the `preview.explain.*` i18n keys

### Requirement: Regeneration preview
The «Перегенерировать аудио» queue context-menu action SHALL open the preview
dialog pre-filled with the entry's `original_text` before any audio is
touched. The right pane SHALL show the normalization the regeneration will
narrate: the plain pipeline over `original_text` as-is — no source-format
detection and no HTML extraction, because a stored entry's `original_text` is
already synthesis-ready (HTML-ingested entries store the extracted text).

In regeneration mode the dialog SHALL hide the controls that do not apply:

- «Редактировать» — the entry's `original_text` is immutable after creation;
- the source-format selector — regeneration does not consult it;
- «Больше не показывать этот диалог» — that gate belongs to the Add flow and
  MUST NOT disable the regeneration preview.

The «Read Now» switch, the synchronized-scrolling checkbox, «Отмена», and the
confirm button remain. The confirm button SHALL be labeled
«Перегенерировать».

Confirming SHALL close the dialog and invoke `regenerate_entry` with the
entry id and the switch state; the command owns the delete-then-synthesize
sequence, so the old audio is deleted only after confirmation. Cancelling —
via the «Отмена» button, ESC, or the header close icon — SHALL leave the
entry, its audio, and its status completely untouched. The blue
«перегенерация» notification SHALL appear only after confirmation.

The context-menu item stays disabled for `processing` entries (the backend
rejects their regeneration).

Only one preview dialog may be open at a time: opening the regeneration
preview closes the Add-flow preview if it was open, and an Add/import
opening closes the regeneration preview — the non-modal floating windows
must never stack with their window-level ESC handlers doubled up.

#### Scenario: Regenerate opens the preview instead of synthesizing

- GIVEN a `ready` entry whose normalization the user wants to inspect
- WHEN the user picks «Перегенерировать аудио» in the queue context menu
- THEN the preview dialog opens pre-filled with the entry's `original_text`,
  the right pane shows its normalization, and the old audio still exists

#### Scenario: Cancel keeps the existing audio

- GIVEN the regeneration preview is open for a `ready` entry
- WHEN the user presses ESC (or «Отмена», or the close icon)
- THEN the dialog closes, the entry keeps its audio, status, and timestamps,
  and no notification is shown

#### Scenario: Confirm regenerates

- GIVEN the regeneration preview is open with «Read Now» off
- WHEN the user presses «Перегенерировать»
- THEN the dialog closes, `regenerate_entry` is invoked with
  `play_when_ready: false`, and the blue «перегенерация» notification appears
  while the entry re-runs synthesis

#### Scenario: Regeneration mode hides inapplicable controls

- GIVEN the regeneration preview is open
- WHEN the user looks at the dialog
- THEN «Редактировать», the source-format selector, and «Больше не
  показывать этот диалог» are absent, while «Read Now», «Синхронный
  скроллинг», «Отмена», and «Перегенерировать» are present

#### Scenario: Preview shows the stored text without re-extraction

- GIVEN an HTML-ingested entry (its `original_text` is the extracted TTS
  text, `html_source` the sanitized markup)
- WHEN the regeneration preview opens
- THEN the left pane shows `original_text` and the right pane shows the
  normalization of that text — the markup is not extracted again

#### Scenario: Error entry can be regenerated through the preview

- GIVEN an `error` entry
- WHEN the user picks «Перегенерировать аудио»
- THEN the preview dialog opens, and confirming re-runs synthesis for the
  entry

#### Scenario: Previews are mutually exclusive

- GIVEN the Add-flow preview dialog is open (the dialog is non-modal, the
  queue underneath stays interactive)
- WHEN the user picks «Перегенерировать аудио»
- THEN the Add-flow preview closes and only the regeneration preview is
  shown — the two floating windows never stack, and ESC closes exactly one
  dialog

### Requirement: Quick-add to dictionary from preview

The preview dialog SHALL offer a "В словарь" action, enabled only while the
current text selection in either pane is a single valid source token (Latin
letters and digits, at least one letter). Activating it SHALL open the
dictionary editor with the add form prefilled: `from` set to the selected
token, `to` empty. Selections that are not a single valid token (Cyrillic,
multi-word, containing punctuation) SHALL leave the action disabled with a
hint explaining what a valid token is.

#### Scenario: Latin word selection enables the action

- GIVEN the preview shows "Ivanov" in the original pane
- WHEN the user selects exactly "Ivanov"
- THEN the "В словарь" action becomes enabled and opens the editor with
  from "Ivanov" and an empty spoken form

#### Scenario: Cyrillic selection keeps the action disabled

- GIVEN the preview shows normalized Cyrillic text
- WHEN the user selects a Cyrillic word
- THEN the "В словарь" action stays disabled
