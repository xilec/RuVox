# Pipeline Golden Fixtures

Golden test fixtures for the Rust TTS pipeline. Each fixture is a triple of files
that captures the expected behavior of the legacy Python pipeline for a specific input.

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

## How to regenerate

Run from the repository root (where `legacy/` and `scripts/` live):

```bash
nix-shell legacy/shell.nix --run "uv run --project legacy python scripts/generate_golden.py"
```

If the nix-shell is unavailable, you can run the script directly with any Python 3.11+
interpreter that has `num2words` installed:

```bash
PYTHONPATH=legacy/src python scripts/generate_golden.py
```

The script imports `ruvox.tts_pipeline` from `legacy/src` and writes all fixture files into
`src-tauri/tests/fixtures/pipeline/`.

## Adding a new test case

1. Open `scripts/generate_golden.py`.
2. Add a new entry to `TEST_CASES` dict: `"my_slug": "Input text here."`.
3. Re-run the generator.
4. Commit the three new files: `my_slug.input.txt`, `my_slug.expected.txt`, `my_slug.char_map.json`.

## Important note on regeneration

If the Rust pipeline output differs from a golden fixture, first determine whether
the discrepancy is intentional before regenerating:

- If the Rust behavior is a deliberate improvement or a known acceptable difference,
  document it in the Rust test (e.g., with a `// known_diff: ...` comment) and
  update the fixture.
- If the Rust behavior is a bug, fix the Rust code — do not update the fixture.
- Only regenerate the golden data when the Python pipeline itself changes or when
  a discrepancy is explicitly approved by the project maintainer.
