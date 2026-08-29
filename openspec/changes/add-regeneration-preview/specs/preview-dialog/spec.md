# Preview Dialog — Regeneration Preview (delta)

## ADDED Requirements

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

## MODIFIED Requirements

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
