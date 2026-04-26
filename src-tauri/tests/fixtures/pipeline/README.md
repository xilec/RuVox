# Pipeline Golden Fixtures

Golden test fixtures for the Rust TTS pipeline. Each fixture captures the expected
behavior of the pipeline for a specific input and is treated as the source of truth
in regression tests.

## File layout

For each test case named `<slug>`, there are three files:

- `<slug>.input.txt` — raw input text fed to the pipeline.
- `<slug>.expected.txt` — normalized output text expected from the pipeline.
- `<slug>.char_map.json` — character-level mapping from transformed positions back to original positions.

## char_map.json schema

```json
{
  "original": "...",
  "transformed": "...",
  "char_map": [[orig_start, orig_end], ...]
}
```

Fields:

- `original` — the original input text (same as `<slug>.input.txt`).
- `transformed` — the normalized output (same as `<slug>.expected.txt`).
- `char_map` — array of `[orig_start, orig_end]` pairs, one per character in `transformed`.
  For each character at position `i` in `transformed`, `char_map[i]` is the half-open range
  `[orig_start, orig_end)` in `original` that it corresponds to.
  - If one original character expands to multiple transformed characters, all of them share the same range.
  - If multiple original characters collapse to one transformed character, the range spans all of them.

This mapping is used for word highlighting during playback: given a playback position mapped
to a word in `transformed`, the Rust backend can find the corresponding span in `original`
and highlight it in the UI.

## Covered categories

The fixtures cover all normalizer categories:

- Numbers: plain integers, large numbers, decimals
- Sizes and durations: KB, MB, GB, ms
- Versions: semver, pre-release
- Ranges: year ranges, numeric ranges
- Percentages: integer and decimal
- English words and IT terms: common words, acronyms
- Abbreviations: JSON, XML, HTTP, HTTPS
- Code identifiers: camelCase, PascalCase, snake_case, kebab-case
- URLs, emails, IP addresses, file paths
- Special symbols: Greek letters, math operators, arrow symbols
- Markdown structures: headers, numbered lists, inline code, code blocks, Mermaid diagrams, links
- Mixed content: realistic paragraph combining multiple normalizers

## Adding a new test case

1. Drop three files into this directory: `<slug>.input.txt`, `<slug>.expected.txt`, `<slug>.char_map.json`.
2. The expected output is whatever the current Rust pipeline produces — run the test once,
   inspect the diff, and freeze the actual output as the new golden if the behavior is correct.
3. The simplest way to bootstrap `char_map.json` is to copy the structure from a similar
   existing fixture and edit it for the new case.

## Important note on regeneration

If the Rust pipeline output diverges from a golden fixture, decide whether the change
is intentional before updating the fixture:

- If the new Rust behavior is a deliberate improvement, update the fixture and note the
  reason in the commit message (and a `// known_diff: ...` comment in the test if useful).
- If the new behavior is a bug, fix the Rust code — do not update the fixture.
