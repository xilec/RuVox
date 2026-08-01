# Proposal: html-view-support

## Why

When HTML is copied from a browser, only the plain-text flavor reaches the
pipeline: formatting is lost and, worse, there is no way to view the copied
content as rendered HTML with working word highlighting. The existing Rust
extractor (`src-tauri/src/pipeline/html_extractor.rs`) is an unfinished
prototype — it is dead code, its spans carry a `(0, 0)` sentinel (html5ever
does not expose source offsets), its offsets are in bytes while the rest of
the stack uses codepoints, and its `normalise_extracted` pass invalidates
the spans. Fixing highlight on top of it would require a second, drifting
re-implementation of the extraction rules on the frontend.

Instead, extraction moves to the frontend (GitHub issue #6, agreed
"variant B"): the webview has the reference HTML parser, the same DOM that
is rendered, and one coordinate system (codepoints in the extracted text)
for TTS mapping and highlighting alike.

## What Changes

- **HTML ingestion:** pasting into the app (`Ctrl+V`) detects the
  `text/html` clipboard flavor, sanitizes it with DOMPurify, extracts plain
  text for TTS via a new TS extractor, and creates the entry with
  `format: "html"`. The Add-button flow tries the same HTML detection and
  falls back to plain text.
- **New TS extractor** (`src/lib/htmlText.ts`): walks the sanitized DOM,
  applies block/exclusion rules, produces the TTS text plus word spans with
  codepoint offsets (`data-orig-*`), reusing `wrapWordsWithOrigPos`.
- **Schema:** `TextEntry.html_source: string | null` — sanitized HTML kept
  for rendering; `original_text` stores the extracted text (what the TTS
  pipeline consumed), so char-mapping, timestamps and regeneration work
  unchanged.
- **HTML mode highlighting:** `renderHtml` emits `data-orig-*` word spans;
  the `highlightingEnabled` HTML gate and TODO(U5) are removed.
- **Deletion:** `src-tauri/src/pipeline/html_extractor.rs` and the
  `scraper` dependency are removed (dead code).
- Tray "Add" keeps reading the plain-text flavor (graceful degradation).

## Capabilities

### New Capabilities

- `html-ingestion`: detecting HTML clipboard content, sanitizing it,
  extracting TTS text with word spans, and storing the entry with
  `html_source`.

### Modified Capabilities

- `storage`: History File Schema gains `html_source` (optional, defaults
  to null).
- `ipc-commands`: `TextEntry` IPC type gains `html_source`;
  `add_text_entry` accepts optional format/html-source parameters.
- `text-display`: HTML mode renders `html_source` with word spans;
  highlight is available in HTML mode.
- `word-highlight`: HTML mode no longer ignores position events.
- `position-mapping`: the "HTML text extraction with spans" requirement
  (Rust extractor, byte offsets, sentinel) is removed — extraction is a
  frontend concern with codepoint offsets.

## Impact

- New: `src/lib/htmlText.ts` + tests; paste handler in `AppShell.tsx`.
- Changed: `src/lib/html.ts` (span emission), `src/lib/wordHighlight.ts`
  (gate removal), `src/components/TextViewer.tsx` (highlight in HTML mode),
  `src/lib/tauri.ts` (`html_source`, extended `addTextEntry`),
  `src-tauri/src/storage/schema.rs` (`html_source` field),
  `src-tauri/src/commands/mod.rs` (`add_text_entry` params).
- Removed: `src-tauri/src/pipeline/html_extractor.rs`, `scraper` from
  `src-tauri/Cargo.toml`.
- Depends on change `persist-entry-format` (entry `format` field).
- GitHub issue #6 closes.
