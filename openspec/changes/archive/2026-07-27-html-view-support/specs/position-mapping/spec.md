# Delta: position-mapping

## REMOVED Requirements

### Requirement: HTML text extraction with spans

**Reason**: The Rust extractor (`src-tauri/src/pipeline/html_extractor.rs`)
is deleted. Its html-side mapping was never implemented (`html_start` /
`html_end` carried the sentinel `0`), its offsets were in bytes while the
rest of the stack uses codepoints, and the module was dead code. HTML
extraction moves to the frontend, where a single walker produces both the
TTS text and the render spans in one codepoint coordinate system.

**Migration**: Extraction behavior (excluded tags, block-to-newline
structure, whitespace collapsing) is re-specified in the `html-ingestion`
capability. The TTS pipeline and char-mapping are unaffected: they always
operated on plain text and still do — the extracted text simply arrives as
`original_text` from the frontend.
