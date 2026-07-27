# Design: preview-source-format

## Context

The Add flow with `preview_dialog_enabled` opens `PreviewDialog` with the
clipboard text and creates the entry via `handlePreviewSynthesize` →
`doAddEntry(text, playWhenReady)` — always as a plain entry (`format:
null`). When the clipboard's plain flavor carries raw HTML markup (no
`text/html` flavor offered by the source), the markup is synthesized
verbatim. The HTML extraction machinery already exists on the frontend
(`sanitizeHtml`, `extractTextForTts`, `AppShell.addHtmlEntry`) and the
backend already accepts `format`/`html_source` in `add_text_entry`.

## Goals / Non-Goals

**Goals:**

- Manual source-format choice in the preview dialog, honored at ingest.
- Zero backend changes; reuse the existing extraction path verbatim.
- Preview shows what will be narrated for the `html` choice.

**Non-Goals:** automatic sniffing; changing the no-dialog Add flow; Piper
chunking.

## Decisions

### 1. Selector UI: Mantine `Select` in the dialog header/footer

A `Select` with `plain` / `markdown` / `html`, initial value from
`UIConfig.text_format` (the viewer default, passed from `AppShell` as a
prop) so the picker matches what the user already configured. Alternatives
rejected: `SegmentedControl` (duplicates the viewer's mode switch visually,
implies a display toggle rather than an ingest decision); auto-detect
heuristics (rejected by the user for now).

### 2. `html` choice reuses the paste-ingestion path as-is

On "Синтезировать" with `html` selected, `AppShell` routes the final
(original or edited) text through the same sanitize + extract calls used by
`addHtmlEntry`, persisting `format: "html"` + `html_source`. If extraction
yields no readable text, ingestion aborts with a red notification ("не
удалось извлечь текст из HTML") and no entry is created — same contract as
the paste path's `false` return, surfaced explicitly because there is no
plain fallback in the dialog (the user made an explicit choice).

### 3. `plain` / `markdown` choices persist the display format

`doAddEntry` gains the chosen `format` so the entry renders in the picked
mode from the start (previously always `format: null` → viewer default).
Synthesis input is the text unchanged in both cases — the pipeline already
handles markdown constructs; the choice is display + intent only.

### 4. Format-aware preview

The right pane normalizes `extractTextForTts(sanitizeHtml(text))` when
`html` is selected, and the raw text otherwise. Extraction failures in
preview render the same inline error pattern as normalization failures; the
1 s debounce is unchanged.

## Risks / Trade-offs

- [User picks `html` for non-HTML text] → Extraction of plain prose returns
  the prose itself (DOMParser text), so worst case is `format: "html"` on a
  plain entry — harmless and visible in the viewer.
- [Picker adds a click to the common flow] → Default follows the configured
  viewer default; the common flow needs no interaction.
