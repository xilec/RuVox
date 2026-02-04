# Fast TTS - Project Documentation for Development

## Overview

**Fast TTS** — пайплайн предобработки текста для TTS Silero. Преобразует технический текст (документация, статьи, мануалы) в форму, пригодную для качественного озвучивания русской TTS-моделью.

### Проблема

Silero TTS не умеет корректно читать:
- Английские слова и IT-термины (`feature` → молчание или искажение)
- Аббревиатуры (`API`, `HTTP`, `JSON`)
- URL, email, IP-адреса
- Идентификаторы кода (`getUserData`, `my_variable`)
- Спецсимволы и операторы (`->`, `>=`, `!=`)

### Решение

Пайплайн нормализации, который преобразует все нестандартные элементы в читаемый русский текст:
```
"Вызови getUserData() через API" → "Вызови гет юзер дата через эй пи ай"
```

---

## Architecture

### Pipeline Flow

```
Input Text (Markdown/Plain)
         │
         ▼
┌─────────────────────────────────────────────────────────────┐
│  STAGE 0: PREPROCESSING                                      │
│  - Remove BOM, invisible chars                               │
│  - Normalize whitespace                                      │
│  - Unify quotes and dashes                                   │
└─────────────────────────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────────────────────────┐
│  STAGE 1: STRUCTURAL PARSING                                 │
│  - Extract code blocks (```...```)                           │
│  - Extract inline code (`...`)                               │
│  - Parse markdown structure                                  │
└─────────────────────────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────────────────────────┐
│  STAGE 2: TOKENIZATION                                       │
│  - Split into tokens                                         │
│  - Classify token types (URL, email, number, etc.)           │
│  - Assign processing priorities                              │
└─────────────────────────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────────────────────────┐
│  STAGE 3: NORMALIZATION                                      │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐           │
│  │ English │ │ Abbrev  │ │ Numbers │ │  URLs   │           │
│  │ Words   │ │         │ │         │ │ Paths   │           │
│  └────┬────┘ └────┬────┘ └────┬────┘ └────┬────┘           │
│       │           │           │           │                 │
│  ┌────┴────┐ ┌────┴────┐ ┌────┴────┐ ┌────┴────┐           │
│  │IT Dict  │ │Abbrev   │ │num2words│ │URL      │           │
│  │+ G2P    │ │Dict     │ │(russian)│ │Parser   │           │
│  └─────────┘ └─────────┘ └─────────┘ └─────────┘           │
│                                                              │
│  ┌─────────┐ ┌─────────┐                                    │
│  │ Symbols │ │  Code   │                                    │
│  │Operators│ │Identif. │                                    │
│  └─────────┘ └─────────┘                                    │
└─────────────────────────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────────────────────────┐
│  STAGE 4: POST-PROCESSING                                    │
│  - Join tokens                                               │
│  - Clean up spacing                                          │
│  - Final punctuation normalization                           │
└─────────────────────────────────────────────────────────────┘
         │
         ▼
    Output Text (TTS-ready)
```

### Token Types (Priority Order)

| Priority | Type | Pattern Example | Transformation |
|----------|------|-----------------|----------------|
| 1 | CODE_BLOCK | ` ```python...``` ` | full/brief mode |
| 2 | INLINE_CODE | `` `code` `` | normalize content |
| 3 | URL | `https://...` | spell out fully |
| 4 | EMAIL | `user@domain.com` | use "собака" |
| 5 | IP_ADDRESS | `192.168.1.1` | numbers + "точка" |
| 6 | FILE_PATH | `/home/user/file.txt` | use "слэш" |
| 7 | VERSION | `v2.3.1` | numbers + "точка" |
| 8 | SIZE_UNIT | `100MB` | number + unit word |
| 9 | PERCENTAGE | `99.9%` | number + "процентов" |
| 10 | RANGE | `10-20` | "от X до Y" |
| 11 | DATE | `2024-01-15` | full date words |
| 12 | TIME | `14:30` | hours + minutes |
| 13 | ABBREVIATION | `API`, `HTTP` | dict or spell out |
| 14 | CAMEL_CASE | `getUserData` | split + transliterate |
| 15 | SNAKE_CASE | `get_user_data` | split + transliterate |
| 16 | KEBAB_CASE | `my-component` | split + transliterate |
| 17 | FLOAT | `3.14` | number + "точка" + digits |
| 18 | NUMBER | `123` | num2words russian |
| 19 | OPERATOR | `->`, `>=` | word description |
| 20 | ENGLISH | `feature` | IT dict or G2P |
| 21 | RUSSIAN | `привет` | pass through |

---

## Project Structure

```
fast_tts/
├── pyproject.toml           # Project config, dependencies
├── shell.nix                 # NixOS dev environment
├── README.md                 # User documentation
├── AGENTS.md                 # This file - dev documentation
│
├── scripts/
│   └── test.sh              # Test runner helper
│
├── src/fast_tts/
│   ├── __init__.py          # Package exports
│   ├── config.py            # PipelineConfig dataclass
│   ├── pipeline.py          # Main TTSPipeline class
│   │
│   └── normalizers/
│       ├── __init__.py      # Normalizer exports
│       ├── english.py       # EnglishNormalizer - IT terms + G2P
│       ├── abbreviations.py # AbbreviationNormalizer
│       ├── numbers.py       # NumberNormalizer - all numeric types
│       ├── urls.py          # URLPathNormalizer - URLs, emails, IPs, paths
│       ├── symbols.py       # SymbolNormalizer - operators, punctuation
│       └── code.py          # CodeIdentifierNormalizer, CodeBlockHandler
│
└── tests/
    ├── conftest.py          # Pytest fixtures
    ├── test_english.py      # 121 tests
    ├── test_abbreviations.py # 108 tests
    ├── test_numbers.py      # 108 tests
    ├── test_urls.py         # 62 tests
    ├── test_symbols.py      # 74 tests
    ├── test_code.py         # 104 tests
    └── test_pipeline.py     # 55 integration tests
                             # ─────────────────
                             # TOTAL: 632 tests
```

---

## Module Specifications

### 1. EnglishNormalizer (`normalizers/english.py`)

**Purpose:** Transliterate English words to Russian phonetic spelling.

**Interface:**
```python
class EnglishNormalizer:
    def __init__(self):
        self.it_dict: dict[str, str] = {}  # IT terms dictionary

    def normalize(self, word: str) -> str:
        """Convert English word to Russian phonetic spelling."""
```

**Requirements:**
- Dictionary of 150+ IT terms with established pronunciation
- Case-insensitive matching
- Multi-word phrases support ("pull request" → "пулл реквест")
- G2P fallback for unknown words (optional, via g2p-en)

**Key Dictionary Entries:**
```python
{
    "feature": "фича",
    "branch": "бранч",
    "merge": "мёрдж",
    "cache": "кэш",
    "queue": "кью",
    "deploy": "деплой",
    "docker": "докер",
    "kubernetes": "кубернетис",
    "python": "пайтон",
    "javascript": "джаваскрипт",
    "react": "риэкт",
    "github": "гитхаб",
    # ... 150+ more
}
```

**Test file:** `tests/test_english.py` (121 tests)

---

### 2. AbbreviationNormalizer (`normalizers/abbreviations.py`)

**Purpose:** Convert abbreviations to speakable text.

**Interface:**
```python
class AbbreviationNormalizer:
    def __init__(self):
        self.as_word: dict[str, str] = {}    # Pronounced as word
        self.letter_map: dict[str, str] = {} # Letter pronunciations

    def normalize(self, abbrev: str) -> str:
        """Convert abbreviation to speakable text."""
```

**Requirements:**
- Two modes: as word ("JSON" → "джейсон") or spelled out ("API" → "эй пи ай")
- Complete English alphabet mapping to Russian
- Case-insensitive

**Key Mappings:**
```python
# As word
AS_WORD = {
    "json": "джейсон",
    "yaml": "ямл",
    "rest": "рест",
    "ajax": "эйджакс",
    "crud": "крад",
    "cors": "корс",
    "gif": "гиф",
    "ram": "рам",
    "sql": "эс кью эл",  # or "сиквел"
}

# Letter map
LETTER_MAP = {
    'a': 'эй', 'b': 'би', 'c': 'си', 'd': 'ди', 'e': 'и',
    'f': 'эф', 'g': 'джи', 'h': 'эйч', 'i': 'ай', 'j': 'джей',
    'k': 'кей', 'l': 'эл', 'm': 'эм', 'n': 'эн', 'o': 'оу',
    'p': 'пи', 'q': 'кью', 'r': 'ар', 's': 'эс', 't': 'ти',
    'u': 'ю', 'v': 'ви', 'w': 'дабл ю', 'x': 'экс', 'y': 'уай',
    'z': 'зед',
}
```

**Test file:** `tests/test_abbreviations.py` (108 tests)

---

### 3. NumberNormalizer (`normalizers/numbers.py`)

**Purpose:** Convert all numeric types to Russian words.

**Interface:**
```python
class NumberNormalizer:
    def normalize_number(self, num_str: str) -> str: ...
    def normalize_float(self, float_str: str) -> str: ...
    def normalize_percentage(self, pct_str: str) -> str: ...
    def normalize_range(self, range_str: str) -> str: ...
    def normalize_size(self, size_str: str) -> str: ...
    def normalize_version(self, ver_str: str) -> str: ...
    def normalize_date(self, date_str: str) -> str: ...
    def normalize_time(self, time_str: str) -> str: ...
```

**Requirements:**
- Use `num2words` library with `lang='ru'`
- Proper Russian declension for units (1 процент, 2 процента, 5 процентов)
- Support comma as decimal separator (European style)
- Date formats: ISO (2024-01-15), European (15.01.2024)

**Examples:**
```python
"123"      → "сто двадцать три"
"3.14"     → "три точка один четыре"
"50%"      → "пятьдесят процентов"
"10-20"    → "от десяти до двадцати"
"100MB"    → "сто мегабайт"
"v2.3.1"   → "два точка три точка один"
"2024-01-15" → "пятнадцатое января две тысячи двадцать четвёртого года"
"14:30"    → "четырнадцать часов тридцать минут"
```

**Dependencies:** `num2words>=0.5.12`

**Test file:** `tests/test_numbers.py` (108 tests)

---

### 4. URLPathNormalizer (`normalizers/urls.py`)

**Purpose:** Normalize URLs, emails, IPs, and file paths.

**Interface:**
```python
class URLPathNormalizer:
    def normalize_url(self, url: str) -> str: ...
    def normalize_email(self, email: str) -> str: ...
    def normalize_ip(self, ip: str) -> str: ...
    def normalize_filepath(self, path: str) -> str: ...
```

**Requirements:**
- Full URL expansion (protocol + domain + path)
- Common TLD pronunciation (com→ком, org→орг, ru→ру)
- Email: use "собака" for @
- IP: read octets as numbers
- Paths: "слэш" for /, "бэкслэш" for \

**Examples:**
```python
"https://github.com/user/repo"
→ "эйч ти ти пи эс двоеточие слэш слэш github точка ком слэш user слэш repo"

"user@example.com"
→ "user собака example точка ком"

"192.168.1.1"
→ "сто девяносто два точка сто шестьдесят восемь точка один точка один"

"/home/user/config.yaml"
→ "слэш home слэш user слэш config точка yaml"
```

**Test file:** `tests/test_urls.py` (62 tests)

---

### 5. SymbolNormalizer (`normalizers/symbols.py`)

**Purpose:** Convert operators and special characters to words.

**Interface:**
```python
class SymbolNormalizer:
    def normalize(self, symbol: str) -> str:
        """Convert symbol/operator to speakable text."""
```

**Requirements:**
- Operators: `->` (стрелка), `>=` (больше или равно), `!=` (не равно)
- Brackets with direction: `(` (открывающая скобка)
- Common symbols: `@` (собака), `#` (решётка), `&` (амперсанд)

**Key Mappings:**
```python
OPERATORS = {
    "->": "стрелка",
    "=>": "толстая стрелка",
    ">=": "больше или равно",
    "<=": "меньше или равно",
    "!=": "не равно",
    "==": "равно равно",
    "&&": "и",
    "||": "или",
    "::": "двойное двоеточие",
}

BRACKETS = {
    "(": "открывающая скобка",
    ")": "закрывающая скобка",
    "[": "открывающая квадратная скобка",
    # ...
}
```

**Test file:** `tests/test_symbols.py` (74 tests)

---

### 6. CodeIdentifierNormalizer & CodeBlockHandler (`normalizers/code.py`)

**Purpose:** Handle code identifiers and code blocks.

**Interface:**
```python
class CodeIdentifierNormalizer:
    def normalize_camel_case(self, identifier: str) -> str: ...
    def normalize_snake_case(self, identifier: str) -> str: ...
    def normalize_kebab_case(self, identifier: str) -> str: ...

class CodeBlockHandler:
    def __init__(self, mode: str = "full"): ...
    def set_mode(self, mode: str) -> None: ...
    def process(self, code: str, language: str | None = None) -> str: ...
```

**Requirements:**
- Split identifiers on case/underscore/hyphen boundaries
- Transliterate each word via EnglishNormalizer
- CodeBlockHandler modes:
  - `full`: read code content (normalize and speak)
  - `brief`: "далее следует пример кода на пайтон"

**Examples:**
```python
"getUserData"     → "гет юзер дата"
"get_user_data"   → "гет юзер дата"
"my-component"    → "май компонент"

# Brief mode for code block
"```python\nprint('hello')\n```"
→ "далее следует пример кода на пайтон"
```

**Test file:** `tests/test_code.py` (104 tests)

---

### 7. TTSPipeline (`pipeline.py`)

**Purpose:** Main orchestrator combining all normalizers.

**Interface:**
```python
class TTSPipeline:
    def __init__(self, config: PipelineConfig | None = None): ...
    def process(self, text: str) -> str: ...
    def set_code_mode(self, mode: str) -> None: ...
```

**Requirements:**
- Coordinate all normalizers in correct priority order
- Handle markdown structure (code blocks, headings, lists)
- Preserve Russian text unchanged
- Configurable via PipelineConfig

**Test file:** `tests/test_pipeline.py` (55 integration tests)

---

### 8. PipelineConfig (`config.py`)

**Interface:**
```python
@dataclass
class PipelineConfig:
    code_block_mode: str = "full"      # "full" or "brief"
    url_detail_level: str = "full"     # "full", "domain_only", "minimal"
    read_operators: bool = True         # Whether to read operators
    ip_read_mode: str = "numbers"       # "numbers" or "digits"
    custom_it_terms: dict = field(default_factory=dict)
    custom_abbreviations: dict = field(default_factory=dict)
    debug: bool = False
```

---

## Development Workflow

### Environment Setup

**ВАЖНО:** Разработка ведётся на NixOS. Используется пакетный менеджер `nix` для управления окружением.

```bash
cd /home/evgen/work/tts_fast_tests/fast_tts
nix-shell  # Enters dev environment with Python 3.11 + uv
```

После входа в `nix-shell` доступен `uv` для управления Python-зависимостями:

```bash
uv sync                    # Install all dependencies
uv sync --extra g2p        # With G2P support
uv sync --extra tts        # With TTS (torch, silero)
uv run pytest              # Run tests
uv run python script.py    # Run scripts
```

### Running Tests

```bash
# All tests
./scripts/test.sh

# Quick mode
./scripts/test.sh quick

# Specific module
./scripts/test.sh module numbers
./scripts/test.sh module english
./scripts/test.sh module abbreviations

# Pattern matching
./scripts/test.sh match 'integer'
./scripts/test.sh match 'url'

# Stop on first failure
./scripts/test.sh stop

# Only previously failed
./scripts/test.sh failed

# With coverage
./scripts/test.sh cov
```

### TDD Workflow

1. **Pick a module** to implement (start with simplest: `numbers` or `symbols`)
2. **Run module tests**: `./scripts/test.sh module numbers`
3. **See failing tests** with expected inputs/outputs
4. **Implement** to make tests pass
5. **Refactor** while keeping tests green
6. **Move to next module**

### Recommended Implementation Order

1. **NumberNormalizer** - straightforward, uses num2words
2. **SymbolNormalizer** - simple dictionary lookup
3. **AbbreviationNormalizer** - dictionary + letter spelling
4. **EnglishNormalizer** - dictionary + optional G2P
5. **URLPathNormalizer** - string parsing + other normalizers
6. **CodeIdentifierNormalizer** - regex split + EnglishNormalizer
7. **CodeBlockHandler** - mode switching + optional full processing
8. **TTSPipeline** - integration of all components

---

## Dependencies

### Required
```
num2words>=0.5.12     # Russian number-to-words
```

### Optional (for G2P)
```
g2p-en>=2.1.0         # English grapheme-to-phoneme
eng-to-ipa>=0.0.2     # English to IPA conversion
```

### Development
```
pytest>=8.0           # Testing
pytest-cov>=4.0       # Coverage
```

---

## Test Coverage Targets

| Module | Tests | Target Coverage |
|--------|-------|-----------------|
| numbers | 108 | 95% |
| symbols | 74 | 95% |
| abbreviations | 108 | 95% |
| english | 121 | 90% (G2P optional) |
| urls | 62 | 95% |
| code | 104 | 95% |
| pipeline | 55 | 90% |

---

## Example Transformations

### Input
```markdown
## Установка Docker

1. Скачайте Docker Desktop с https://docker.com/download
2. Запустите `docker --version` для проверки
3. Версия должна быть >= 20.10.0

```bash
curl -fsSL https://get.docker.com | sh
```

API доступен на http://localhost:8080/api
```

### Output (code_block_mode="brief")
```
Установка докер

Первое. Скачайте докер десктоп с эйч ти ти пи эс двоеточие слэш слэш docker точка ком слэш download
Второе. Запустите докер минус минус вершн для проверки
Третье. Версия должна быть больше или равно двадцать точка десять точка ноль

далее следует пример кода на баш

эй пи ай доступен на эйч ти ти пи двоеточие слэш слэш localhost двоеточие восемь тысяч восемьдесят слэш api
```

---

## References

- [Pipeline.md](../Pipeline.md) - Detailed pipeline specification
- [TranslitEnAnalysis.md](../TranslitEnAnalysis.md) - Research on transliteration
- [Silero TTS](https://github.com/snakers4/silero-models) - Target TTS system
- [num2words](https://pypi.org/project/num2words/) - Number conversion library
- [g2p-en](https://pypi.org/project/g2p-en/) - English G2P (optional)
