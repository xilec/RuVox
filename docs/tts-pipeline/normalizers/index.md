# Нормализаторы

Компоненты для преобразования различных типов токенов в читаемый текст.

## Обзор

| Нормализатор | Файл | Назначение |
|--------------|------|------------|
| [EnglishNormalizer](english.md) | `english.py` | Английские слова и IT-термины |
| [AbbreviationNormalizer](abbreviations.md) | `abbreviations.py` | Аббревиатуры (API, HTTP) |
| [NumberNormalizer](numbers.md) | `numbers.py` | Числа, даты, проценты |
| [URLPathNormalizer](urls.md) | `urls.py` | URL, email, IP, пути |
| [SymbolNormalizer](symbols.md) | `symbols.py` | Операторы и символы |
| [CodeIdentifierNormalizer](code.md) | `code.py` | Идентификаторы кода |

## Архитектура

```
TTSPipeline
    │
    ├── EnglishNormalizer
    │   └── IT_TERMS dictionary (150+ слов)
    │
    ├── AbbreviationNormalizer
    │   ├── AS_WORD dictionary (JSON, YAML, etc.)
    │   └── LETTER_MAP (A→эй, B→би, etc.)
    │
    ├── NumberNormalizer
    │   └── num2words library
    │
    ├── URLPathNormalizer
    │   └── TLD dictionary (com→ком, etc.)
    │
    ├── SymbolNormalizer
    │   ├── OPERATORS (->→стрелка, etc.)
    │   └── GREEK_LETTERS (α→альфа, etc.)
    │
    └── CodeIdentifierNormalizer
        └── EnglishNormalizer (для слов)
```

## Порядок применения в Pipeline

```python
# 1. Структурные элементы
_process_code_blocks()      # ```code```
_process_inline_code()      # `code`
_process_markdown()         # заголовки, списки, ссылки

# 2. Специальные форматы
_process_urls()             # https://...
_process_emails()           # user@domain.com
_process_ips()              # 192.168.1.1
_process_paths()            # /home/user/file

# 3. Числовые форматы
_process_sizes()            # 100MB
_process_versions()         # v2.3.1
_process_ranges()           # 10-20
_process_percentages()      # 50%

# 4. Операторы и символы
_process_operators()        # >=, ->, etc.
_process_symbols()          # Greek, math symbols

# 5. Идентификаторы
_process_code_identifiers() # camelCase, snake_case

# 6. Слова
_process_english_words()    # feature → фича
_process_numbers()          # 123 → сто двадцать три
```

## Добавление кастомных терминов

### Через конфигурацию

```python
from ruvox.tts_pipeline import TTSPipeline, PipelineConfig

config = PipelineConfig(
    custom_it_terms={
        "kubernetes": "кубернетис",
        "nginx": "энжинкс",
    },
    custom_abbreviations={
        "K8S": "кубернетис",
    },
)

pipeline = TTSPipeline(config)
```

### Через прямой доступ

```python
pipeline = TTSPipeline()
pipeline.english_normalizer.IT_TERMS["myterm"] = "май терм"
pipeline.abbrev_normalizer.AS_WORD["myabbr"] = "май аббр"
```

## Тестирование

Каждый нормализатор имеет отдельный файл тестов:

```bash
# Все тесты нормализаторов
uv run pytest tests/tts_pipeline/test_*.py -v

# Конкретный нормализатор
uv run pytest tests/tts_pipeline/test_english.py -v
uv run pytest tests/tts_pipeline/test_numbers.py -v
```

| Модуль | Тестов |
|--------|--------|
| english | 121 |
| abbreviations | 108 |
| numbers | 108 |
| urls | 62 |
| symbols | 74 |
| code | 104 |
