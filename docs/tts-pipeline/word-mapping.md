# WordMapping

Эвристический word-level маппинг между оригинальным и нормализованным текстом.

> **Примечание:** Для точного маппинга используйте [TrackedText](tracked-text.md) через `process_with_char_mapping()`.

## Назначение

`WordMapping` использует эвристики для сопоставления слов:
- Точное совпадение
- Транслитерация (feature → фича)
- Числовые слова (123 → сто двадцать три)

## Импорт

```python
from ruvox.tts_pipeline import WordMapping, build_word_mapping, tokenize_words
```

## tokenize_words

Разбивает текст на слова с позициями.

```python
def tokenize_words(text: str) -> list[WordSpan]
```

**Пример:**
```python
words = tokenize_words("Hello world")
# [WordSpan(text='Hello', start=0, end=5),
#  WordSpan(text='world', start=6, end=11)]
```

## build_word_mapping

Строит маппинг между оригиналом и результатом.

```python
def build_word_mapping(original: str, transformed: str) -> WordMapping
```

**Пример:**
```python
mapping = build_word_mapping(
    "Hello world",
    "хелло ворлд"
)

# Получить оригинальный диапазон для первого слова результата
orig_range = mapping.get_original_range_for_word(0)
# → (0, 5) — позиция "Hello"
```

## WordMapping

### Атрибуты

```python
@dataclass
class WordMapping:
    original_text: str
    transformed_text: str
    original_words: list[WordSpan]
    transformed_words: list[WordSpan]
    word_map: dict[int, tuple[int, int]]  # trans_idx → (orig_start_idx, orig_end_idx)
```

### get_original_range_for_word

```python
def get_original_range_for_word(self, trans_word_idx: int) -> tuple[int, int] | None
```

Возвращает символьный диапазон в оригинале для слова по индексу.

### get_original_range_for_position

```python
def get_original_range_for_position(self, trans_start: int, trans_end: int) -> tuple[int, int] | None
```

Возвращает диапазон в оригинале для позиции в результате.

## Эвристики сопоставления

1. **Точное совпадение** — одинаковые слова
2. **Транслитерация** — английское слово → кириллица с той же первой буквой
3. **Числовые слова** — русские числительные сопоставляются с цифрами
4. **Fallback** — несопоставленные слова привязываются к последнему найденному

## Ограничения

- Эвристический подход не всегда точен
- При сложных трансформациях возможны ошибки
- Для точного маппинга используйте `TrackedText`

## Когда использовать

- Быстрый приблизительный маппинг
- Когда точность не критична
- Для обратной совместимости

Для новых реализаций рекомендуется `process_with_char_mapping()`.
