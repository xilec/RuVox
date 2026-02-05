# TTSPipeline

Основной класс для нормализации текста.

## Импорт

```python
from fast_tts_rus.tts_pipeline import TTSPipeline, PipelineConfig
```

## Базовое использование

```python
pipeline = TTSPipeline()

# Нормализация текста
result = pipeline.process("Вызови getUserData() через API")
# → "Вызови гет юзер дата через эй пи ай"
```

## API

### Конструктор

```python
TTSPipeline(config: PipelineConfig | None = None)
```

**Параметры:**
- `config` — конфигурация pipeline (опционально)

### process

```python
def process(self, text: str) -> str
```

Нормализует текст для TTS.

**Параметры:**
- `text` — исходный текст

**Возвращает:**
- Нормализованный текст

**Пример:**
```python
pipeline.process("Версия >= 2.0")
# → "Версия больше или равно два точка ноль"
```

### process_with_char_mapping

```python
def process_with_char_mapping(self, text: str) -> tuple[str, CharMapping]
```

Нормализует текст с отслеживанием позиций.

**Параметры:**
- `text` — исходный текст

**Возвращает:**
- Кортеж (нормализованный текст, CharMapping)

**Пример:**
```python
result, mapping = pipeline.process_with_char_mapping("Test 123")
# result = "тест сто двадцать три"

# Найти оригинальную позицию для "сто"
orig_range = mapping.get_original_range(5, 8)
# → (5, 8) — позиция "123" в оригинале
```

### process_with_mapping

```python
def process_with_mapping(self, text: str) -> tuple[str, WordMapping]
```

Нормализует текст с эвристическим word-level маппингом.

> **Примечание:** Используйте `process_with_char_mapping` для точного маппинга.

### set_code_mode

```python
def set_code_mode(self, mode: str) -> None
```

Переключает режим обработки блоков кода.

**Параметры:**
- `mode` — `"full"` или `"brief"`

**Пример:**
```python
pipeline.set_code_mode("brief")
pipeline.process("```python\nprint('hello')\n```")
# → "далее следует пример кода на пайтон"
```

### get_unknown_words

```python
def get_unknown_words(self) -> dict[str, str]
```

Возвращает слова, транслитерированные через fallback (не найденные в словаре).

**Возвращает:**
- Словарь {оригинал: транслитерация}

**Пример:**
```python
pipeline.process("Используем Kubernetes и Terraform")
unknown = pipeline.get_unknown_words()
# → {"terraform": "тэрраформ"}  # если нет в словаре
```

### get_warnings

```python
def get_warnings(self) -> list[str]
```

Возвращает предупреждения о неизвестных словах.

### print_warnings

```python
def print_warnings(self) -> None
```

Выводит предупреждения в stderr.

## PipelineConfig

```python
from fast_tts_rus.tts_pipeline import PipelineConfig

config = PipelineConfig(
    code_block_mode="brief",      # "full" или "brief"
    read_operators=True,          # читать ли операторы
    custom_it_terms={             # дополнительные термины
        "kubernetes": "кубернетис",
        "terraform": "терраформ",
    },
)

pipeline = TTSPipeline(config)
```

### Параметры конфигурации

| Параметр | Тип | По умолчанию | Описание |
|----------|-----|--------------|----------|
| `code_block_mode` | str | `"full"` | Режим блоков кода |
| `read_operators` | bool | `True` | Читать операторы словами |
| `custom_it_terms` | dict | `{}` | Дополнительные IT-термины |
| `custom_abbreviations` | dict | `{}` | Дополнительные аббревиатуры |

## Примеры

### Технический текст

```python
text = """
## Установка Docker

1. Скачайте с https://docker.com
2. Запустите `docker --version`
3. Версия должна быть >= 20.10.0
"""

result = pipeline.process(text)
```

**Результат:**
```
Установка докер

Первое: Скачайте с эйч ти ти пи эс двоеточие слэш слэш docker точка ком
Второе: Запустите докер минус минус вершн
Третье: Версия должна быть больше или равно двадцать точка десять точка ноль
```

### Код с идентификаторами

```python
pipeline.process("Функция getUserData возвращает user_profile")
# → "Функция гет юзер дата возвращает юзер профайл"
```

### С маппингом для подсветки

```python
text = "Осталось 42 дня"
result, mapping = pipeline.process_with_char_mapping(text)
# result = "Осталось сорок два дня"

# При воспроизведении слова "сорок" (позиция 9-14 в result)
orig_start, orig_end = mapping.get_original_range(9, 14)
# orig_start=9, orig_end=11 — позиция "42" в оригинале

# Подсветить в оригинальном тексте
highlight = text[orig_start:orig_end]  # "42"
```

## Доступ к нормализаторам

```python
pipeline = TTSPipeline()

# IT-термины
pipeline.english_normalizer.IT_TERMS["newterm"] = "нью терм"

# Аббревиатуры
pipeline.abbrev_normalizer.AS_WORD["newabbr"] = "нью аббр"

# Обработка
result = pipeline.process("Using newterm with NEWABBR")
```

## Тестирование

```bash
# Все тесты pipeline
uv run pytest tests/tts_pipeline/test_pipeline.py -v

# Конкретный тест
uv run pytest tests/tts_pipeline/test_pipeline.py -k "test_mixed_text"
```
