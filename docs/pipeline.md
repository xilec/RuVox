# Pipeline нормализации

Pipeline превращает технический текст в форму, пригодную для Silero TTS.

**Реализация:** Rust, `src-tauri/src/pipeline/`. Корректность проверяется golden-фикстурами в `src-tauri/tests/fixtures/pipeline/` (37 кейсов).

## API

```rust
use crate::pipeline::TTSPipeline;

let mut pipeline = TTSPipeline::new();

// Только нормализация (без mapping)
let result: String = pipeline.process("Вызови getUserData() через API");
// → "Вызови гет юзер дата через эй пи ай"

// С char-mapping для подсветки
let (result, mapping) = pipeline.process_with_char_mapping("Test 123");
// result   = "тест сто двадцать три"
// mapping  = CharMapping { original, transformed, char_map }
```

`CharMapping::get_original_range(trans_start, trans_end) -> (orig_start, orig_end)` — преобразование позиции в нормализованном тексте обратно в оригинал.

`process_with_char_mapping` — основной API. `process` — обёртка, отбрасывающая mapping.

Pipeline хранится в `AppState` как `Arc<Mutex<TTSPipeline>>` и вызывается из:

- `add_clipboard_entry` / `add_text_entry` — нормализация перед синтезом.
- `preview_normalize` — для preview-диалога (FF 1.1).

## Структура

```
src-tauri/src/pipeline/
├── mod.rs              # TTSPipeline::process_with_char_mapping — порядок фаз
├── tracked_text.rs     # TrackedText, CharMapping (отслеживание позиций)
├── constants.rs        # GREEK_LETTERS, MATH_SYMBOLS, ARROW_SYMBOLS
├── html_extractor.rs   # Извлечение текста из HTML (для u8 формата)
└── normalizers/
    ├── mod.rs
    ├── numbers.rs       # NumberNormalizer (числа, размеры, версии, диапазоны, проценты)
    ├── english.rs       # EnglishNormalizer (IT_TERMS словарь + транслитерация)
    ├── abbreviations.rs # AbbreviationNormalizer (AS_WORD словарь + LETTER_MAP)
    ├── code.rs          # CodeIdentifierNormalizer (camel/pascal/snake/kebab)
    ├── code_blocks.rs   # CodeBlockHandler (```code``` и ```mermaid```)
    ├── urls.rs          # URLPathNormalizer (URL, email, IP, path)
    └── symbols.rs       # SymbolNormalizer (операторы, греческие, математика)
```

## Этапы обработки

Порядок строго фиксирован — нарушение ломает регрессию. См. `src-tauri/src/pipeline/mod.rs::process_with_char_mapping`.

```
Input
  │
  ▼
┌─ Phase 1.  Code blocks ───────────────────────────┐
│  ```mermaid``` → "Тут мермэйд диаграмма"          │
│  ```python``` → full / brief mode (см. config)    │
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
│  Headers (#, ##, ...) → удаляется                 │
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
│  Single = НЕ обрабатывается (ломает math)         │
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
│            AS_WORD → транслитерация               │
└────────────────────────────────────────────────────┘
  ▼
┌─ Phase 16. Numbers ───────────────────────────────┐
│  123 → "сто двадцать три"                         │
│  Контекст: НЕ обрабатывать число рядом с буквами  │
│            (уже обработано в фазах 8-9 / 14)      │
└────────────────────────────────────────────────────┘
  ▼
Postprocess: удаление лишних пробелов / пустых строк
  ▼
Output (TTS-ready) + CharMapping
```

## Почему порядок важен

- **URL раньше чисел** — `192.168.1.1` обрабатывается как IP, иначе превратится в четыре отдельных числа.
- **Версии раньше чисел** — `v2.3.1` обрабатывается целиком, иначе разбьётся на `2`, `3`, `1`.
- **camelCase раньше английских слов** — `getUserData` разбивается, иначе транслитерируется как одно слово.
- **Аббревиатуры раньше английских слов** — `API` читается побуквенно, иначе попытка транслитерации.
- **Многосимвольные операторы перед однобуквенными** — `===` перед `==`, `>=` перед `>` (в `TRACKED_OPERATOR_KEYS` порядок «длинный раньше короткого»).

## TrackedText / CharMapping

`TrackedText` — обёртка над текстом, которая запоминает все замены и потом строит `CharMapping`: для каждого символа результата хранится диапазон в оригинале.

```rust
let mut t = TrackedText::new("Test 123 world");
t.sub(&re_number(), |caps| "сто двадцать три".to_string());
t.replace("world", "мир");

let mapping = t.build_mapping();
// mapping.transformed = "Test сто двадцать три мир"
// mapping.get_original_range(5, 8) → (5, 8)  // "сто" → "123"
// mapping.get_original_range(22, 25) → (9, 14)  // "мир" → "world"
```

**Защита от пересечений:** если новая замена пересекает границу существующей, `TrackedText` блокирует операцию — это гарантирует, что каждый символ результата однозначно связан с одним диапазоном оригинала.

**Underflow guard:** в `current_to_original` промежуточные вычисления используют `saturating_sub` / `.max(0)`, чтобы не схлопывать длинные не-монотонные цепочки замен в нелегальный `usize`.

## Mermaid-маркер

Mermaid-блоки не содержат читаемого текста. `CodeBlockHandler::process` детектит блок ` ```mermaid ... ``` ` и заменяет его строкой `"Тут мермэйд диаграмма"`.

В UI mermaid рендерится отдельно через `mermaid.js` (см. [ui.md](ui.md)).

## Code-блоки

Поведение для не-mermaid блоков кода зависит от `UIConfig.code_block_mode`:

- **`full`** (по умолчанию) — содержимое читается посимвольно с нормализацией операторов, идентификаторов, etc.
- **`brief`** — заменяется на «далее следует пример кода на <язык>».

## Golden-тесты

`src-tauri/tests/fixtures/pipeline/` содержит 37 пар `<case>.input.txt` / `<case>.expected.txt` (+ `.char_map.json` для регрессии маппинга), покрывающих:

- Числа (`number_plain`, `number_decimal`, `number_large`).
- Размеры (`size_mb`, `size_kb`, `size_gb`, `duration_ms`).
- Версии (`version_patch`, `version_prerelease`, `version_semver`).
- Диапазоны и проценты (`range_simple`, `range_years`, `percentage_int`, `percentage_decimal`).
- Английский (`english_word_common`, `english_word_it`).
- Аббревиатуры (`abbreviation_http`, `abbreviation_upper`).
- Code-style (`camelcase`, `pascalcase`, `snake_case`, `kebab_case`).
- URL/email/IP/paths (`url_https`, `email`, `ip_address`, `filepath_absolute`).
- Markdown (`markdown_header`, `markdown_link`, `markdown_list_number`, `markdown_inline_code`, `markdown_code_block`, `markdown_mermaid`).
- Символы (`greek_letters`, `arrow_symbols`, `math_symbols`, `operators`).
- Mixed (`mixed_paragraph`).

**Прогон:**

```bash
nix-shell --run "cargo test --manifest-path src-tauri/Cargo.toml --test golden"
nix-shell --run "cargo test --manifest-path src-tauri/Cargo.toml -- pipeline"
```

**Добавление нового кейса:** см. [contributing.md](contributing.md).
