# Tasks: preview-source-format

## 1. Preview dialog

- [x] 1.1 Add a Mantine `Select` (plain/markdown/html) to `PreviewDialog`, initialized from a new `defaultFormat` prop passed by `AppShell` from `UIConfig.text_format`
- [x] 1.2 Make the right preview pane format-aware: with `html` it normalizes `extractTextForTts(sanitizeHtml(text))`, otherwise the text as-is; selector changes re-trigger the debounced preview
- [x] 1.3 Pass the selected format through `onSynthesize`

## 2. Ingest routing

- [x] 2.1 `handlePreviewSynthesize`: with `html` route the final text through the existing sanitize+extract path and create the entry with `format: "html"` + `html_source`; empty extraction shows a red notification and creates no entry
- [x] 2.2 With `plain`/`markdown` persist the chosen format on the entry (`doAddEntry` format parameter)

## 3. Tests and gates

- [x] 3.1 TS unit tests for the new pure seams (format-aware preview input selection; html-choice payload shape)
- [x] 3.2 `just test` and `just lint` green
- [ ] 3.3 Manual: paste raw HTML markup (plain flavor) → Add → pick `html` in the dialog → preview shows narratable text → entry renders as HTML and synthesis reads no tags; default flow without touching the selector behaves as before
