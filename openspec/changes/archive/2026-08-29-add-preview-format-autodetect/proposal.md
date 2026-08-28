## Why

The preview dialog's source-format selector requires the user to know and pick
the format (`plain` / `markdown` / `html`) manually. Misclassifying markup as
plain or markdown reads tags aloud or renders garbage — the exact failure the
format selector exists to prevent — yet the user often cannot tell what they
are looking at (a fetched page, a pasted fragment). A detector that picks the
format automatically removes the decision from the common path while keeping
the manual override for the rare misclassification (issue #241).

## What Changes

- Add a pure `detectFormat` decision step that classifies text as `plain`,
  `markdown`, or `html` from content signals only.
- HTML detection is deliberately conservative: a `<!DOCTYPE html` / `<html`
  prefix or several well-formed tag fragments; technical prose with angle
  brackets (`a < b`, `<T>`) and a single stray tag-looking fragment stay plain.
- Markdown detection uses strong structural signals only: ATX headings, fenced
  code blocks, dense list syntax, or repeated inline links.
- The preview dialog's source-format selector gains an «Авто» value as the
  default; its label shows what was detected for the current text («Авто
  (HTML)»), and detection re-runs as the user edits the text.
- Preview normalization and synthesis ingest follow the effective (detected or
  explicitly chosen) format; an explicit selection keeps today's behavior.
- Imported sources keep their routed format preselected in the dialog
  (extension decides for files), and the URL `text/*` routing switches from
  the interim "always plain" to the shared detector, as the text-import spec
  already mandates ("URL falls back to detection") — implementation catching
  up, no requirement change.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `preview-dialog`: the source-format selection requirement changes — the
  selector defaults to a new auto mode driven by content detection, and a new
  requirement pins the detection rules and their false-positive limits.

## Impact

- `src/lib/detectFormat.ts` (new): pure classifier + unit tests over the
  classification matrix.
- `src/dialogs/PreviewDialog.tsx`: selector data, effective-format resolution,
  i18n strings for the «Авто» labels.
- `src/i18n/ru.ts` / `src/i18n/en.ts`: new strings (localized RU/EN per
  convention).
- No backend, storage, or wire-format changes; `EntryFormat` is unchanged —
  «Авто» is a UI-level selector state that resolves to a concrete format
  before preview and ingest.
