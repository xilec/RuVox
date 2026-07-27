# Tasks: html-view-support

Depends on: change `persist-entry-format` (entry `format` field, `set_entry_format`).

## 1. TS extraction walker

- [x] 1.1 Create `src/lib/htmlText.ts`: walk a sanitized DOM subtree producing (a) extracted TTS text (exclusion/block/whitespace rules per the html-ingestion spec) and (b) word-wrapped rendering via `wrapWordsWithOrigPos` with codepoint offsets into the extracted text
- [x] 1.2 Unit tests `src/lib/htmlText.test.ts`: paragraphs + inline code (offset assertions), excluded chrome tags, block-to-newline structure, lists/tables, NBSP collapsing, whitespace-only extraction

## 2. Rendering with spans

- [x] 2.1 Update `src/lib/html.ts`: `renderHtml` sanitizes, then emits `data-orig-*` word spans via the walker
- [x] 2.2 Update `src/lib/html.test.ts`: span offsets match the extracted text for representative HTML
- [x] 2.3 `TextViewer`: HTML mode renders `entry.html_source ?? entry.original_text`

## 3. Ingestion

- [x] 3.1 `src-tauri/src/storage/schema.rs`: `#[serde(default)] pub html_source: Option<String>` on `TextEntry` + serde tests (round-trip, legacy default)
- [x] 3.2 `add_text_entry`: optional `format` / `html_source` params, wired through `ingest_text` into `storage.add_entry_with_source`; orchestration tests for the HTML parameter path
- [x] 3.3 `src/lib/tauri.ts`: `html_source` on `TextEntry`; extended `addTextEntry` wrapper
- [x] 3.4 Paste handler in `AppShell.tsx`: `Ctrl+V` reads `text/html` from the paste event → DOMPurify → extract → `addTextEntry(extracted, playWhenReady, "html", sanitized)`; empty HTML flavor → existing plain path
- [x] 3.5 Add-button flow: best-effort `navigator.clipboard.read()` for `text/html`, fallback to plugin `readText`

## 4. Highlight in HTML mode

- [x] 4.1 `src/lib/wordHighlight.ts`: remove the HTML gate (`highlightingEnabled`) and TODO(U5); update `wordHighlight.test.ts`
- [x] 4.2 `src/components/TextViewer.tsx`: drop the HTML early-return in the position handler

## 5. Rust extractor removal

- [x] 5.1 Delete `src-tauri/src/pipeline/html_extractor.rs`, `pub mod html_extractor;`, and the `scraper` dependency from `src-tauri/Cargo.toml`

## 6. Gates

- [x] 6.1 `nix develop -c just test` green
- [x] 6.2 `nix develop -c just lint` green
- [ ] 6.3 Manual check on KDE: copy HTML from Chrome → `Ctrl+V` in the app → entry renders as HTML, narration reads clean text, word highlight tracks playback; Add-button fallback still works; tray Add still creates plain entries (pre-PR manual pass)
