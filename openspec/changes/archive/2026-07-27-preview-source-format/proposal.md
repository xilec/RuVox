# Proposal: preview-source-format

## Why

HTML markup that arrives as plain clipboard text (copy from view-source,
DevTools, or any source that does not offer the `text/html` flavor) is
ingested verbatim: the pipeline narrates tags and attributes ("див класс ти
эм…") because flavor-based detection has no way to recognize it. Automatic
content sniffing was considered and rejected for now (heuristic false
positives on legitimate text); the user asked for an explicit manual
choice instead.

## What Changes

- The normalization preview dialog gains a source-format selector
  (plain / markdown / html), defaulting to the configured viewer default
  format.
- Choosing `html` interprets the dialog's text as HTML markup at ingest
  time: it is sanitized and run through the existing frontend extraction
  (`sanitizeHtml` + `extractTextForTts`), and the entry is created with
  `format: "html"`, extracted `original_text`, and `html_source` — the same
  shape as paste-based HTML ingestion. Empty extraction rejects with an
  error notification instead of creating an entry.
- Choosing `plain` or `markdown` keeps the text as-is and persists the
  chosen display format on the entry (previously entries created via the
  dialog always had `format: null`).
- The preview's right pane reflects the choice: for `html` it normalizes
  the extracted text, so the user sees what will actually be narrated.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `preview-dialog`: new requirement for the source-format selector and its
  effect on preview and ingestion.

## Non-goals

- Automatic HTML content sniffing (deferred; may become redundant with the
  manual picker).
- Changing the direct Add flow without the preview dialog (plain flavor
  still ingests as-is there).
- Piper chunking (tracked separately in issue #155).

## Impact

- `src/dialogs/PreviewDialog.tsx` — selector UI, format-aware preview,
  format passed to `onSynthesize`.
- `src/components/AppShell.tsx` — `handlePreviewSynthesize` routes html
  choices through the existing extraction path; passes `format` for
  plain/markdown choices.
- No backend changes: `add_text_entry` already accepts `format` /
  `html_source`; `preview_normalize` is unchanged.
