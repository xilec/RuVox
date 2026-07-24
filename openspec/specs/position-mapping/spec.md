# Position Mapping Specification

## Purpose

Covers how the backend tracks, for every character of normalized (or
HTML-extracted) text, the corresponding range in the original source. This
mapping powers word-level highlighting during playback: TTS word timestamps
refer to the normalized text, and the UI converts them back to original-text
positions via `CharMapping`. Implemented by `TrackedText` / `CharMapping`
(`src-tauri/src/pipeline/tracked_text.rs`) and by the HTML extractor
(`src-tauri/src/pipeline/html_extractor.rs`).

## Requirements

### Requirement: CharMapping data model

The system SHALL represent the mapping as `CharMapping { original,
transformed, char_map }`, where `char_map` holds, for every **Unicode
codepoint** position in `transformed`, the range `(orig_start, orig_end)`
(exclusive end) in `original`, also in codepoint units. Byte offsets MUST NOT
be used. The length of `char_map` SHALL always equal the codepoint count of
`transformed`, including after expanding replacements (e.g. "42" → "сорок
два") and contracting ones (e.g. "getUserData" → "гет"). Codepoints that were
never replaced SHALL map to the identity range `(i, i + 1)`; every codepoint
produced by a replacement SHALL map to the full original range of that
replacement.

#### Scenario: Unchanged text maps to identity

- GIVEN `TrackedText` over "Hello world" with no replacements
- WHEN the mapping is built
- THEN `char_map` has 11 entries and every entry is `(i, i + 1)`

#### Scenario: Replaced word maps all its output codepoints to the source range

- GIVEN `TrackedText` over "Hello world" where "world" was replaced with
  "мир"
- WHEN the mapping is built
- THEN each of the 3 codepoints of "мир" maps to `(6, 11)` — the original
  range of "world"

#### Scenario: Length invariant after expansion

- GIVEN any input processed by `TTSPipeline::process_with_char_mapping`
- WHEN the mapping is returned
- THEN `char_map.len()` equals the codepoint count of `transformed`

### Requirement: Original range lookup

The system SHALL provide
`CharMapping::get_original_range(trans_start, trans_end) -> (usize, usize)`
returning the minimal bounding range in `original` that covers all codepoints
in `transformed[trans_start..trans_end]`. An out-of-range `trans_start` SHALL
be clamped to the last valid position, so a query past the end of the text
returns the last character's entry instead of failing. The system SHALL also
provide `get_original_word_range(trans_pos)` that resolves the position and
expands the result outward to whitespace boundaries, yielding the whole word
in the original text.

#### Scenario: Range spanning replaced and unchanged text

- GIVEN "Hello world" where "world" was replaced with "мир"
- WHEN `get_original_range(4, 7)` is queried (spanning "o " and the first
  codepoint of "мир")
- THEN the result is `(4, 11)` — the bounding range from the unchanged prefix
  through the end of "world"

#### Scenario: Position past end is clamped

- GIVEN a mapping built from "Hello"
- WHEN `get_original_range(10, 15)` is queried
- THEN the result is `(4, 5)` — the entry of the last character

#### Scenario: Word range expands to whitespace

- GIVEN "Hello world" where "world" was replaced with "мир"
- WHEN `get_original_word_range(7)` is queried (inside "мир")
- THEN the result is `(6, 11)` — the full original word "world"

### Requirement: TrackedText replacement guarantees

`TrackedText` SHALL record every `replace` / `sub` / `replace_byte_range`
operation and guarantee that each codepoint of the final text is linked to
exactly one original range. No-op replacements (new text equal to old text)
SHALL be skipped. A new replacement whose match touches a codepoint inside an
already-replaced region, or whose computed original range overlaps an
existing replacement, SHALL be skipped — later pipeline phases MUST NOT
re-process text produced by earlier phases. Position arithmetic SHALL use
saturating operations so that long non-monotonic replacement chains can never
underflow into an illegal index.

#### Scenario: Nested replacement is blocked

- GIVEN "Hello world" where "world" was replaced with "foo bar"
- WHEN a second replacement targets "foo" (inside the replaced region)
- THEN the second replacement is skipped and the text stays "Hello foo bar"

#### Scenario: Already-consumed region is not re-normalized

- GIVEN the input "192.168.1.1" processed by the pipeline
- WHEN the IP phase has consumed the address
- THEN the later number phase does not re-expand the digits inside the
  replacement, because its matches overlap the recorded region

### Requirement: Pipeline mapping output

`TTSPipeline::process_with_char_mapping` SHALL return the normalized text
together with its `CharMapping`, where `original` is the exact input string.
For empty input the system SHALL return an empty text and an empty mapping.
After the final trim of the normalized output the system SHALL slice
`char_map` by the number of trimmed leading/trailing codepoints (not bytes),
so the invariant "`char_map.len()` equals the codepoint count of the result"
holds for multi-byte (Cyrillic) content as well.

#### Scenario: Mapping for a normalized identifier

- GIVEN the input "getUserData"
- WHEN `process_with_char_mapping` is called
- THEN `mapping.original` equals "getUserData", the transformed text is
  "гет юзер дата", and `char_map` is non-empty with every entry pointing
  into the original range

#### Scenario: Trim fixup with multi-byte input

- GIVEN the input "  привет мир  " (leading/trailing spaces around Cyrillic)
- WHEN `process_with_char_mapping` is called
- THEN the result is trimmed, no panic occurs, and `char_map.len()` equals
  the codepoint count of the trimmed result

### Requirement: HTML text extraction with spans

The system SHALL provide `extract_text_for_tts(html) -> TrackedHtml` that
extracts readable plain text from an HTML document using the `scraper`
(html5ever) parser, tolerating malformed markup. Subtrees of the tags `nav`,
`footer`, `aside`, `script`, `style`, `head`, `noscript`, `template`, `svg`,
`math`, `button`, `select`, `option`, `optgroup`, and `datalist` SHALL be
excluded entirely. Block-level elements SHALL be separated from surrounding
text by newlines, and `<br>` / `<hr>` SHALL emit a newline. Extracted text
SHALL have whitespace collapsed (including non-breaking spaces treated as
regular spaces) and SHALL be trimmed, with span offsets adjusted accordingly.
Each emitted text run SHALL be recorded as an `HtmlCharSpan { text_start,
text_end, html_start, html_end }` using **byte** offsets; because per-node
source offsets are not available from the parser, `html_start`/`html_end`
SHALL carry the sentinel value `0` meaning "source position unknown".
`TrackedHtml::html_range_for(text_start, text_end)` SHALL return the bounding
HTML range over all spans overlapping the queried text range, or `None` when
no span overlaps. The helper `normalise_extracted` SHALL collapse consecutive
blank lines in extracted text to single newline separators.

#### Scenario: Chrome elements excluded

- GIVEN an HTML document with a `<nav>` menu, a `<script>` block, and a
  `<p>` paragraph
- WHEN `extract_text_for_tts` runs
- THEN the extracted text contains only the paragraph content

#### Scenario: Block structure becomes newlines

- GIVEN HTML with two consecutive `<p>` elements
- WHEN `extract_text_for_tts` runs
- THEN the two paragraphs appear on separate lines in the extracted text

#### Scenario: Span lookup with no overlap

- GIVEN a `TrackedHtml` result
- WHEN `html_range_for` is queried with a text range that overlaps no
  recorded span
- THEN it returns `None`
