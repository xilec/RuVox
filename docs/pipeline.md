# Normalization pipeline

The pipeline turns technical text into a form suitable for Silero TTS.

**Implementation:** Rust, `src-tauri/src/pipeline/`. Correctness is verified by golden fixtures in `src-tauri/tests/fixtures/pipeline/` (37 cases).

## API

```rust
use crate::pipeline::TTSPipeline;

let mut pipeline = TTSPipeline::new();

// Normalization only (no mapping)
let result: String = pipeline.process("Вызови getUserData() через API");
// → "Вызови гет юзер дата через эй пи ай"

// With char-mapping for highlighting
let (result, mapping) = pipeline.process_with_char_mapping("Test 123");
// result   = "тест сто двадцать три"
// mapping  = CharMapping { original, transformed, char_map }
```

`CharMapping::get_original_range(trans_start, trans_end) -> (orig_start, orig_end)` — converts a position in the normalized text back to the original.

`process_with_char_mapping` is the primary API. `process` is a wrapper that discards the mapping.

The pipeline lives in `AppState` as `Arc<Mutex<TTSPipeline>>` and is called from:

- `add_clipboard_entry` / `add_text_entry` — normalization before synthesis.
- `preview_normalize` — for the preview dialog (FF 1.1).

## Structure

```
src-tauri/src/pipeline/
├── mod.rs              # TTSPipeline::process_with_char_mapping — phase order
├── tracked_text.rs     # TrackedText, CharMapping (position tracking)
├── constants.rs        # GREEK_LETTERS, MATH_SYMBOLS, ARROW_SYMBOLS
├── html_extractor.rs   # Text extraction from HTML (for u8 format)
└── normalizers/
    ├── mod.rs
    ├── numbers.rs       # NumberNormalizer (numbers, sizes, versions, ranges, percentages)
    ├── english.rs       # EnglishNormalizer (IT_TERMS dictionary + transliteration)
    ├── abbreviations.rs # AbbreviationNormalizer (AS_WORD dictionary + LETTER_MAP)
    ├── code.rs          # CodeIdentifierNormalizer (camel/pascal/snake/kebab)
    ├── code_blocks.rs   # CodeBlockHandler (```code``` and ```mermaid```)
    ├── urls.rs          # URLPathNormalizer (URL, email, IP, path)
    └── symbols.rs       # SymbolNormalizer (operators, Greek letters, math)
```

## Processing stages

The order is strictly fixed — breaking it breaks the regression. See `src-tauri/src/pipeline/mod.rs::process_with_char_mapping`.

```
Input
  │
  ▼
┌─ Phase 1.  Code blocks ───────────────────────────┐
│  ```mermaid``` → "Тут мермэйд диаграмма"          │
│  ```python``` → full / brief mode (see config)    │
└────────────────────────────────────────────────────┘
  ▼
┌─ Phase 2-3. Quote / Dash normalization ───────────┐
│  «»""'' → " / '                                    │
│  — – → -                                           │
└────────────────────────────────────────────────────┘
  ▼
┌─ Phase 4. Whitespace ─────────────────────────────┐
│  \n{3,} → \n\n;  [ \t]+ → " "                     │
└────────────────────────────────────────────────────┘
  ▼
┌─ Phase 5-6. Markdown ─────────────────────────────┐
│  Inline code (`code`)                             │
│  Headers (#, ##, ...) → removed                   │
│  Links [text](url) → "text"                       │
│  Numbered lists "1." → "первое:"                  │
└────────────────────────────────────────────────────┘
  ▼
┌─ Phase 7. URL / email / IP / path ────────────────┐
│  https://example.com → "эйч ти ти пи эс ..."      │
│  user@host.com → "user собака host точка ком"     │
│  192.168.1.1 → "сто девяносто два точка ..."      │
│  /home/user → "слэш home слэш user"               │
└────────────────────────────────────────────────────┘
  ▼
┌─ Phase 8-11. Numeric formats ─────────────────────┐
│  Sizes: 100MB / 50ms / 24px                       │
│  Versions: v2.3.1 / 1.2.0-beta                    │
│  Ranges: 10-20 → "от десяти до двадцати"          │
│  Percentages: 99.9% → "девяносто девять и ..."    │
└────────────────────────────────────────────────────┘
  ▼
┌─ Phase 12. Operators (longest first) ─────────────┐
│  ===, !==, ->, =>, >=, <=, !=, ==, &&, ||         │
│  Single = is NOT processed (would break math)     │
└────────────────────────────────────────────────────┘
  ▼
┌─ Phase 13. Special symbols ───────────────────────┐
│  Greek: α/β/γ → альфа/бета/гамма                  │
│  Math: ±/×/÷                                      │
│  Arrows: ←/→/↑/↓                                  │
│  Tilde-as-approx: ~46 → "около 46"                │
└────────────────────────────────────────────────────┘
  ▼
┌─ Phase 14. Code identifiers ──────────────────────┐
│  camelCase → camel + case                         │
│  PascalCase → pascal + case                       │
│  snake_case / SCREAMING_CASE                      │
│  kebab-case                                        │
└────────────────────────────────────────────────────┘
  ▼
┌─ Phase 15. English words ─────────────────────────┐
│  Priority: special (C++/C#/F#) →                  │
│            IT_TERMS → uppercase abbrev →          │
│            AS_WORD → transliteration              │
└────────────────────────────────────────────────────┘
  ▼
┌─ Phase 16. Numbers ───────────────────────────────┐
│  123 → "сто двадцать три"                         │
│  Context: do NOT process numbers next to letters  │
│           (already handled in phases 8-9 / 14)    │
└────────────────────────────────────────────────────┘
  ▼
Postprocess: strip extra whitespace / blank lines
  ▼
Output (TTS-ready) + CharMapping
```

## Why the order matters

- **URLs before numbers** — `192.168.1.1` is treated as an IP; otherwise it would turn into four separate numbers.
- **Versions before numbers** — `v2.3.1` is processed as a whole; otherwise it would split into `2`, `3`, `1`.
- **camelCase before English words** — `getUserData` is split; otherwise it would be transliterated as a single word.
- **Abbreviations before English words** — `API` is read letter by letter; otherwise it would be transliterated.
- **Multi-character operators before single-character ones** — `===` before `==`, `>=` before `>` (in `TRACKED_OPERATOR_KEYS` the order is "longest before shortest").

## TrackedText / CharMapping

`TrackedText` is a wrapper around the text that records every replacement and afterwards builds a `CharMapping`: for each character in the result it stores a range in the original.

```rust
let mut t = TrackedText::new("Test 123 world");
t.sub(&re_number(), |caps| "сто двадцать три".to_string());
t.replace("world", "мир");

let mapping = t.build_mapping();
// mapping.transformed = "Test сто двадцать три мир"
// mapping.get_original_range(5, 8) → (5, 8)  // "сто" → "123"
// mapping.get_original_range(22, 25) → (9, 14)  // "мир" → "world"
```

**Overlap protection:** if a new replacement crosses the boundary of an existing one, `TrackedText` blocks the operation — this guarantees that every character in the result is unambiguously linked to a single original range.

**Underflow guard:** in `current_to_original` the intermediate calculations use `saturating_sub` / `.max(0)` so that long, non-monotonic chains of replacements don't collapse into an illegal `usize`.

## Mermaid marker

Mermaid blocks contain no readable text. `CodeBlockHandler::process` detects a ` ```mermaid ... ``` ` block and replaces it with the string `"Тут мермэйд диаграмма"`.

In the UI mermaid is rendered separately via `mermaid.js` (see [ui.md](ui.md)).

## Code blocks

The behavior for non-mermaid code blocks depends on `UIConfig.code_block_mode`:

- **`full`** (default) — the contents are read character by character with normalization of operators, identifiers, etc.
- **`brief`** — replaced with "далее следует пример кода на <язык>".

## Golden tests

`src-tauri/tests/fixtures/pipeline/` contains 37 pairs of `<case>.input.txt` / `<case>.expected.txt` (+ `.char_map.json` for mapping regression), covering:

- Numbers (`number_plain`, `number_decimal`, `number_large`).
- Sizes (`size_mb`, `size_kb`, `size_gb`, `duration_ms`).
- Versions (`version_patch`, `version_prerelease`, `version_semver`).
- Ranges and percentages (`range_simple`, `range_years`, `percentage_int`, `percentage_decimal`).
- English (`english_word_common`, `english_word_it`).
- Abbreviations (`abbreviation_http`, `abbreviation_upper`).
- Code style (`camelcase`, `pascalcase`, `snake_case`, `kebab_case`).
- URL/email/IP/paths (`url_https`, `email`, `ip_address`, `filepath_absolute`).
- Markdown (`markdown_header`, `markdown_link`, `markdown_list_number`, `markdown_inline_code`, `markdown_code_block`, `markdown_mermaid`).
- Symbols (`greek_letters`, `arrow_symbols`, `math_symbols`, `operators`).
- Mixed (`mixed_paragraph`).

**Run:**

```bash
nix develop -c cargo test --manifest-path src-tauri/Cargo.toml --test golden
nix develop -c cargo test --manifest-path src-tauri/Cargo.toml -- pipeline
```

**Adding a new case:** see [contributing.md](contributing.md).
