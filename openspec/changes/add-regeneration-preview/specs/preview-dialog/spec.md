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
