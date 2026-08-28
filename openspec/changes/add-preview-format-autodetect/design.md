## Context

The preview dialog (`src/dialogs/PreviewDialog.tsx`) keeps its selector state
as an `EntryFormat` (`plain` / `markdown` / `html`) and feeds it directly to
`previewTextFor` (preview) and `onSynthesize` (ingest). The pure decision
layer (`src/lib/addFlow.ts`, `src/lib/ingest.ts`) is already unit-tested
without mounting components; issue #241 asks for a `detectFormat` step in the
same style. The import flow (`src/lib/importFlow.ts`, text-import spec)
deliberately stayed single-source in #224 and routes file formats by
extension and URL formats by Content-Type plus a local markup sniff.

## Goals / Non-Goals

**Goals:**

- One pure, exhaustively unit-tested classifier: `detectFormat(text)`.
- «Авто» as the default selector state, with the detected format visible in
  the label and re-detection while editing.
- Correct-by-default ingest: preview and synthesis always go through the
  effective (detected or explicitly chosen) format.
- HTML detection biased against false positives (tags read aloud are the
  costly mistake), per issue #241's priority note.

**Non-Goals:**

- No changes to `EntryFormat`, the wire protocol, storage, or the backend.
- No change to file-import routing: the extension stays authoritative for
  files (text-import spec, "Import format routing"). URL routing for
  non-markup Content-Types switches from the interim "always plain" to the
  shared detector — that part is *in* scope, because the archived text-import
  spec already mandates detection there ("URL falls back to detection").
- No user setting for enabling/disabling auto-detection: an explicit selector
  click is the escape hatch, persisted per dialog use only.

## Decisions

- **New module `src/lib/detectFormat.ts` rather than extending `ingest.ts`.**
  The classifier is a self-contained content rule; `ingest.ts` stays the
  sanitize/extract decision and `addFlow.ts` the clipboard probe. One home
  per rule (code-quality: DRY); the dialog is the only caller today, which
  keeps the module small and importable later by `importFlow`.
- **HTML heuristic: strong prefix or ≥3 well-formed tag fragments.** A
  fragment matches `/<\/?[a-zA-Z][^<>]*>/` (letter right after `<`/`</`, so
  `a < b` never matches; at least one letter rules out `<>`; attributes
  allowed). `<T>` counts as *one* fragment, and the threshold of three is
  what keeps generics-heavy prose plain. The `<!DOCTYPE html`/`<html` prefix
  short-circuits to `html`. Alternative considered: a fixed allowlist of
  known HTML tag names — rejected as a maintenance-heavy dictionary for no
  observed precision gain.
- **Markdown heuristic: structural signals with density thresholds.** ATX
  heading line, fenced code delimiter on its own line, ≥3 list-item lines,
  or ≥2 inline links. A single decorative dash line or one link stays plain
  ("strong signals only" per the issue); thresholds are constants exported
  from the module so tests pin them by name.
- **«Авто» is a UI-level selector state, not a new `EntryFormat`.** The
  dialog stores `EntryFormat | 'auto'`; the effective format passed to
  preview and `onSynthesize` is always concrete (`sourceFormat === 'auto' ?
  detectFormat(effectiveText) : sourceFormat`). Re-detection while editing
  falls out for free because detection runs on the same `editedText` the
  preview already depends on.
- **The dialog takes an optional `initialFormat` instead of `defaultFormat`.**
  Clipboard openings omit it: the selector starts in auto and detection
  decides (clipboard markup detects as `html`, plain text as `plain`, so the
  effective format converges with the old defaults). Import openings pass the
  routed format so the extension stays authoritative for files (text-import
  spec) — the user still sees it, can override it, or switch to auto.
  `UIConfig.text_format` keeps its viewer role untouched.
- **The URL text/* branch reuses the detector.** The interim routing
  ingested non-markup Content-Types as plain unconditionally; the archived
  text-import spec ("URL falls back to detection") already promises
  content-based classification there, so the branch now routes through
  `detectFormat`. A markdown document served as `text/plain` imports as
  markdown instead of plain.
- **Labels are localized i18n keys** (`preview.source_format.auto_detected`
  with `{0}` interpolation, plus per-format names used by all selector
  options), RU and EN catalogs in the same change (conventions: user-facing
  strings localized).

## Risks / Trade-offs

- **Markdown false positives** (a plain text with 3+ dash lines — e.g. a
  YAML front matter or a menu dump — would ingest as `markdown`). Accepted:
  markdown ingestion is lossless (the text is stored unchanged), only the
  viewer's rendering mode differs, and the user sees the detection in the
  label before pressing the button.
- **HTML false negatives** (a fragment with 1–2 tags narrates raw). Accepted:
  the label shows «Авто (Plain)», so the mistake is visible before synthesis,
  and the explicit `html` override remains one click away.
- **Label churn while editing**: detection re-runs on every state change; the
  label may flip between formats as the user types. Cosmetic and informative
  (it previews the ingest decision) — detection is cheap, no debounce needed.
- **URL text/* reclassification** (a plain-text response that happens to
  carry 3+ tag fragments now ingests through the HTML extraction path
  instead of verbatim). Spec-mandated ("URL falls back to detection") and
  visible in the dialog before confirmation when the preview gate is on.
