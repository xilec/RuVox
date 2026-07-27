# Design: html-view-support

## Context

Agreed direction ("variant B"): HTML extraction lives on the frontend, the
backend stays format-agnostic. The existing Rust extractor
(`src-tauri/src/pipeline/html_extractor.rs`) is deleted: dead code, `(0,0)`
sentinel spans (html5ever exposes no source offsets), byte offsets vs the
codepoint stack, and a `normalise_extracted` pass that invalidates spans.

Clipboard constraints discovered during exploration:

- `tauri-plugin-clipboard-manager` (v2) has **no `read_html`** — the
  permission table offers `allow-write-html` only.
- `arboard` reads are known-flaky on KDE Plasma 6 / Wayland (issue #93) —
  that is why the Add-button flow reads via the plugin from the webview.
- The webview **paste event** (`Ctrl+V`) carries `text/html` in
  `event.clipboardData` natively — no permissions, no arboard.

Depends on change `persist-entry-format` (entry `format` field).

## Goals / Non-Goals

**Goals:**

- Copying HTML from a browser and pasting into RuVox creates an entry that
  renders as sanitized HTML and is narrated as clean extracted text.
- Word highlighting works in HTML mode.
- One coordinate system end-to-end: codepoints of the extracted text
  (`original_text`), used by char-mapping, timestamps, and `data-orig-*`
  spans alike.
- One implementation of the extraction rules (the TS walker), shared by
  ingestion and rendering.

**Non-Goals:**

- HTML support in the tray "Add" path (no webview; plain flavor only).
- Markdown auto-detection; editing `html_source`; re-synthesis on format
  toggle (see persist-entry-format, D2).
- Perfect fidelity to browser `innerText` (CSS visibility, `:before` content
  etc. are out of scope for a DOM-only walk).

## Decisions

### D1: Extraction on the frontend, backend format-agnostic

The backend receives ready-to-normalize text and never sees HTML. All
pipeline/char-map/timestamps/regenerate code paths stay untouched.

Alternative considered: rewrite the Rust extractor and compose
HTML→plain→normalized mappings — rejected: html5ever gives no source
offsets, forcing a second extraction-rule implementation on the frontend
for highlighting; two implementations of the same rules will drift.

### D2: `original_text` = extracted text; `html_source` for rendering

`TextEntry.html_source: Option<String>` (serde default `None`) stores the
**sanitized** HTML for display. `original_text` keeps its existing meaning:
"the text the TTS pipeline consumed" — which is exactly what char-mapping
and `original_pos` refer to. Display modes: `plain`/`markdown` render
`original_text`; `html` renders `html_source` (falling back to
`original_text` when null, e.g. a plain entry toggled to HTML mode).

Alternative considered: `original_text` = raw HTML plus a separate
`tts_text` field — rejected: it branches every backend consumer of
`original_text` (pipeline input, preview, search) on the format.

### D3: Paste event is the primary HTML-detection channel

- `Ctrl+V` anywhere in the main window: `event.clipboardData.getData("text/html")`
  — non-empty → HTML ingestion path; empty → existing plain path
  (`getData("text/plain")`). Reliable in WebKit2GTK, no permission prompts.
- Add button: best-effort `navigator.clipboard.read()` for a `text/html`
  item; on any failure fall back to the current plugin `readText` path.
- Tray: unchanged (`arboard` plain flavor).

Alternatives considered: a `read_html` plugin call (does not exist upstream);
an arboard-based Rust command (same KDE/Wayland reliability problem as #93).

### D4: Sanitize first, extract from the sanitized DOM, store the sanitized HTML

Ingestion order: clipboard HTML → DOMPurify (existing `renderHtml` config)
→ parse once via `DOMParser` → walk. `html_source` stores the DOMPurify
output. Render time sanitizes the stored source again (idempotent) — what
we extracted from, what we store, and what we render are the same document.

### D5: One walker produces both the TTS text and the render spans

`src/lib/htmlText.ts` exports a walk over a DOM subtree that (a) builds the
extracted text with block/exclusion rules (block-level elements separated
by newlines, chrome tags excluded, inline whitespace collapsed, NBSP →
space — mirroring the deleted Rust rules), and (b) wraps every word of
every included text node in `<span data-orig-start data-orig-end>` with
codepoint offsets into the extracted text, reusing `wrapWordsWithOrigPos`.
Ingestion uses (a); `renderHtml` uses (b). Because both come from one walk
over the same document, offsets cannot drift.

Offsets are codepoints, consistent with `wordSpans.ts` and the
position-mapping spec; no byte offsets anywhere.

### D6: Delete the Rust extractor and its spec requirement

Remove `src-tauri/src/pipeline/html_extractor.rs`, the
`pub mod html_extractor;` declaration, and the `scraper` dependency. The
`position-mapping` requirement "HTML text extraction with spans" is REMOVED
(extraction semantics move to the new `html-ingestion` capability spec).

## Risks / Trade-offs

- [`navigator.clipboard.read()` may be blocked in WebKit2GTK] → paste event
  is the primary path; the Add button degrades to plain text exactly as
  today. Manual test on KDE covers both.
- [Extraction rules subtly differ from browser `innerText`] → acceptable
  for TTS; rules are pinned by unit tests with representative HTML
  (paragraphs, lists, tables, code blocks, nested inline markup, excluded
  chrome).
- [DOMPurify config changes alter `html_source` retroactively] → stored
  source is re-sanitized at render, so a stricter future config takes
  effect on old entries too; extraction offsets are recomputed at render
  from the same stored source.
- [Tray users lose formatting] → documented degradation; the plain flavor
  carries the same text content.

## Migration Plan

None — `html_source` is `Option` with a serde default; legacy entries parse
unchanged. No behavior change for plain/markdown entries.

## Open Questions

(none)
