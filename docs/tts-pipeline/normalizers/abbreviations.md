# AbbreviationNormalizer

Преобразование аббревиатур в читаемый текст.

## Импорт

```python
from ruvox.tts_pipeline.normalizers import AbbreviationNormalizer
```

## Использование

```python
normalizer = AbbreviationNormalizer()

normalizer.normalize("API")   # → "эй пи ай"
normalizer.normalize("JSON")  # → "джейсон"
normalizer.normalize("HTTP")  # → "эйч ти ти пи"
```

## Два режима произношения

### 1. Как слово (AS_WORD)

Аббревиатуры, которые произносятся как слова:

```python
AS_WORD = {
    "json": "джейсон",
    "yaml": "ямл",
    "ajax": "эйджакс",
    "crud": "крад",
    "cors": "корс",
    "rest": "рест",
    "gif": "гиф",
    "jpeg": "джейпег",
    "png": "пнг",
    "ram": "рам",
    "rom": "ром",
    "lan": "лан",
    "wan": "ван",
    "sql": "эс кью эл",
    "nosql": "ноу эс кью эл",
}
```

### 2. По буквам (LETTER_MAP)

Остальные аббревиатуры читаются по буквам:

```python
LETTER_MAP = {
    'a': 'эй', 'b': 'би', 'c': 'си', 'd': 'ди', 'e': 'и',
    'f': 'эф', 'g': 'джи', 'h': 'эйч', 'i': 'ай', 'j': 'джей',
    'k': 'кей', 'l': 'эл', 'm': 'эм', 'n': 'эн', 'o': 'оу',
    'p': 'пи', 'q': 'кью', 'r': 'ар', 's': 'эс', 't': 'ти',
    'u': 'ю', 'v': 'ви', 'w': 'дабл ю', 'x': 'экс', 'y': 'уай',
    'z': 'зед',
}
```

## API

### normalize

```python
def normalize(self, abbrev: str) -> str
```

**Параметры:**
- `abbrev` — аббревиатура

**Возвращает:**
- Читаемый текст

**Логика:**
1. Проверка в AS_WORD (регистронезависимо)
2. Если не найдено — по буквам через LETTER_MAP

## Примеры

| Аббревиатура | Результат | Режим |
|--------------|-----------|-------|
| `API` | "эй пи ай" | по буквам |
| `HTTP` | "эйч ти ти пи" | по буквам |
| `URL` | "ю ар эл" | по буквам |
| `CSS` | "си эс эс" | по буквам |
| `HTML` | "эйч ти эм эл" | по буквам |
| `JSON` | "джейсон" | как слово |
| `YAML` | "ямл" | как слово |
| `REST` | "рест" | как слово |
| `AJAX` | "эйджакс" | как слово |

## Добавление аббревиатур

### Как слово

```python
normalizer.AS_WORD["NEWABBR".lower()] = "нью аббр"
```

### Через конфигурацию

```python
config = PipelineConfig(
    custom_abbreviations={
        "k8s": "кубернетис",
        "aws": "эй дабл ю эс",
    }
)
```

## Цифры в аббревиатурах

```python
# K8S, MP3, H264 — цифры читаются
normalizer.normalize("MP3")   # → "эм пи три"
normalizer.normalize("H264")  # → "эйч два шесть четыре"
```

## Регистр

Регистр не влияет на результат:

```python
normalizer.normalize("api")   # → "эй пи ай"
normalizer.normalize("API")   # → "эй пи ай"
normalizer.normalize("Api")   # → "эй пи ай"
```

## Тестирование

```bash
uv run pytest tests/tts_pipeline/test_abbreviations.py -v
```

108 тестов покрывают:
- Словарь AS_WORD
- Произношение по буквам
- Смешанные случаи (буквы + цифры)
- Регистр
