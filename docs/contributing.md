# Contributing

Руководство по внесению вклада в проект.

## Способы помочь

1. **Добавление IT-терминов** — расширение словаря
2. **Исправление произношения** — улучшение существующих терминов
3. **Баг-репорты** — сообщения о проблемах
4. **Новые функции** — предложения и реализация
5. **Документация** — улучшение описаний

## Добавление IT-терминов

### 1. Найти файл

```
src/fast_tts_rus/tts_pipeline/normalizers/english.py
```

### 2. Добавить термин

```python
IT_TERMS = {
    # ... существующие термины ...

    # Добавить в алфавитном порядке
    "kubernetes": "кубернетис",
    "terraform": "терраформ",
}
```

### 3. Добавить тест

```
tests/tts_pipeline/test_english.py
```

```python
def test_new_term():
    normalizer = EnglishNormalizer()
    assert normalizer.normalize("kubernetes") == "кубернетис"
```

### 4. Проверить

```bash
uv run pytest tests/tts_pipeline/test_english.py -v
```

### 5. Создать PR

```bash
git checkout -b feat/add-kubernetes-term
git add src/ tests/
git commit -m "Add kubernetes term to IT dictionary"
git push -u origin feat/add-kubernetes-term
```

## Добавление аббревиатур

### Как слово

```python
# abbreviations.py
AS_WORD = {
    "newabbr": "нью аббр",  # Произносится как слово
}
```

### По буквам

Аббревиатуры, не добавленные в AS_WORD, автоматически произносятся по буквам.

## Исправление произношения

### 1. Создать issue

Опишите:
- Текущее произношение
- Ожидаемое произношение
- Источник (как принято говорить)

### 2. Или сразу PR

```python
# Было
"docker": "докер",

# Стало (например)
"docker": "докэр",
```

## Баг-репорты

### Что включить

1. **Входной текст** — что обрабатывали
2. **Ожидаемый результат** — что должно быть
3. **Фактический результат** — что получилось
4. **Версия** — `git log -1 --oneline`
5. **Окружение** — ОС, Python версия

### Пример

```markdown
**Входной текст:**
`Версия >= 2.0`

**Ожидаемый результат:**
"Версия больше или равно два точка ноль"

**Фактический результат:**
"Версия >= два точка ноль"

**Версия:** abc1234
**Окружение:** NixOS, Python 3.11
```

## Новые функции

### 1. Создать issue

Опишите:
- Что хотите добавить
- Зачем это нужно
- Примеры использования

### 2. Обсудить подход

Дождитесь feedback перед реализацией.

### 3. Реализовать

- Напишите тесты первыми (TDD)
- Следуйте существующему стилю
- Добавьте документацию

## Code Style

### Python

```python
# Type hints для публичных методов
def normalize(self, word: str) -> str:
    """Нормализует английское слово.

    Args:
        word: Английское слово

    Returns:
        Русская транслитерация
    """
    ...

# Константы UPPER_CASE
IT_TERMS = {...}

# Приватные методы с _
def _helper(self) -> None:
    ...
```

### Тесты

```python
class TestEnglishNormalizer:
    """Тесты для EnglishNormalizer."""

    def test_known_term(self):
        """Тест словарного термина."""
        normalizer = EnglishNormalizer()
        assert normalizer.normalize("feature") == "фича"

    def test_unknown_word(self):
        """Тест fallback транслитерации."""
        ...
```

### Коммиты

```
feat: Add kubernetes term to IT dictionary
fix: Correct pronunciation of "queue"
docs: Update normalizers documentation
test: Add tests for URL normalization
refactor: Extract common logic in NumberNormalizer
```

## Pull Request

### Чеклист

- [ ] Тесты проходят: `uv run pytest`
- [ ] Новый код покрыт тестами
- [ ] Документация обновлена (если нужно)
- [ ] Коммит сообщения понятные

### Описание PR

```markdown
## Что сделано

Добавлен термин "kubernetes" в словарь IT_TERMS.

## Зачем

Kubernetes — популярный инструмент, часто встречается в документации.

## Тестирование

- Добавлен тест `test_kubernetes_term`
- Все существующие тесты проходят
```

## Вопросы

Если что-то непонятно — создайте issue с вопросом.
