# TrackedText

Модуль для точного отслеживания позиций при трансформации текста.

## Назначение

При нормализации текста (`"123"` → `"сто двадцать три"`) нужно знать, какой позиции в результате соответствует какая позиция в оригинале. Это необходимо для:

- Подсветки текущего слова при воспроизведении
- Навигации к позиции в оригинальном тексте
- Отображения ошибок с указанием исходной позиции

## Импорт

```python
from fast_tts_rus.tts_pipeline import TrackedText, CharMapping
```

## TrackedText

### Создание

```python
tracked = TrackedText("Hello 123 world")
```

### sub — regex замена с отслеживанием

```python
def sub(
    self,
    pattern: str | re.Pattern,
    repl: str | Callable[[Match], str],
    count: int = 0,
    flags: int = 0,
) -> "TrackedText"
```

**Пример:**
```python
tracked = TrackedText("Hello 123 world")
tracked.sub(r"\b123\b", "сто двадцать три")

print(tracked.text)
# → "Hello сто двадцать три world"
```

### replace — простая замена

```python
def replace(self, old: str, new: str, count: int = -1) -> "TrackedText"
```

**Пример:**
```python
tracked = TrackedText("Hello world")
tracked.replace("world", "мир")

print(tracked.text)
# → "Hello мир"
```

### Цепочки вызовов

```python
tracked = TrackedText("Hello 123 world")
tracked.sub(r"\b123\b", "сто двадцать три").replace("world", "мир")

print(tracked.text)
# → "Hello сто двадцать три мир"
```

### build_mapping — построение маппинга

```python
def build_mapping(self) -> CharMapping
```

Создаёт `CharMapping` после всех трансформаций.

**Пример:**
```python
tracked = TrackedText("Test 123")
tracked.sub(r"\b123\b", "сто двадцать три")

mapping = tracked.build_mapping()
print(mapping.original)      # "Test 123"
print(mapping.transformed)   # "Test сто двадцать три"
```

## CharMapping

Посимвольный маппинг между оригиналом и результатом.

### Атрибуты

```python
@dataclass
class CharMapping:
    original: str                      # Исходный текст
    transformed: str                   # Результат
    char_map: list[tuple[int, int]]   # Маппинг позиций
```

`char_map[i]` содержит `(orig_start, orig_end)` — диапазон в оригинале для символа `i` в результате.

### get_original_range

```python
def get_original_range(self, trans_start: int, trans_end: int) -> tuple[int, int]
```

Преобразует диапазон из результата в диапазон оригинала.

**Пример:**
```python
tracked = TrackedText("Test 123")
tracked.sub(r"\b123\b", "сто двадцать три")
mapping = tracked.build_mapping()

# Диапазон "сто" в результате: позиции 5-8
orig_range = mapping.get_original_range(5, 8)
# → (5, 8) — позиция "123" в оригинале
```

### get_original_word_range

```python
def get_original_word_range(self, trans_pos: int) -> tuple[int, int]
```

Возвращает границы слова в оригинале для позиции в результате.

**Пример:**
```python
# Позиция 6 — внутри "сто"
word_start, word_end = mapping.get_original_word_range(6)
# word_start=5, word_end=8 — границы "123"
```

## Как это работает

### Принцип

1. При каждой замене записывается:
   - Позиция в текущем тексте
   - Оригинальный диапазон
   - Новый текст

2. При `build_mapping()` строится массив `char_map`, где для каждого символа результата хранится соответствующий диапазон оригинала.

### Пример внутренней работы

```python
tracked = TrackedText("AB")  # original = "AB"
tracked.replace("A", "123")  # current = "123B"
tracked.replace("B", "456")  # current = "123456"

mapping = tracked.build_mapping()
# char_map:
# [0] '1' → (0, 1)  # maps to 'A'
# [1] '2' → (0, 1)  # maps to 'A'
# [2] '3' → (0, 1)  # maps to 'A'
# [3] '4' → (1, 2)  # maps to 'B'
# [4] '5' → (1, 2)  # maps to 'B'
# [5] '6' → (1, 2)  # maps to 'B'
```

## Интеграция с Pipeline

`TTSPipeline.process_with_char_mapping()` использует `TrackedText` для всех трансформаций:

```python
pipeline = TTSPipeline()
result, mapping = pipeline.process_with_char_mapping("Test 123 API")

# result = "тест сто двадцать три эй пи ай"

# Маппинг для каждого слова
from fast_tts_rus.tts_pipeline import tokenize_words

for word in tokenize_words(result):
    orig_range = mapping.get_original_range(word.start, word.end)
    orig_text = "Test 123 API"[orig_range[0]:orig_range[1]]
    print(f"'{word.text}' → '{orig_text}'")

# 'тест' → 'Test'
# 'сто' → '123'
# 'двадцать' → '123'
# 'три' → '123'
# 'эй' → 'API'
# 'пи' → 'API'
# 'ай' → 'API'
```

## Использование для подсветки

```python
def highlight_word(original_text: str, mapping: CharMapping,
                   word_start: int, word_end: int) -> str:
    """Подсветить слово в оригинальном тексте."""
    orig_start, orig_end = mapping.get_original_range(word_start, word_end)

    return (
        original_text[:orig_start] +
        "**" + original_text[orig_start:orig_end] + "**" +
        original_text[orig_end:]
    )

# При воспроизведении слова "сто" (позиции 5-8)
highlighted = highlight_word("Test 123 API", mapping, 5, 8)
# → "Test **123** API"
```

## Тестирование

```bash
uv run pytest tests/tts_pipeline/test_tracked_text.py -v
```
