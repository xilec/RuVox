# EnglishNormalizer

Транслитерация английских слов в русское фонетическое написание.

## Импорт

```python
from ruvox.tts_pipeline.normalizers import EnglishNormalizer
```

## Использование

```python
normalizer = EnglishNormalizer()

normalizer.normalize("feature")  # → "фича"
normalizer.normalize("branch")   # → "бранч"
normalizer.normalize("unknown")  # → "анноун" (fallback)
```

## API

### normalize

```python
def normalize(self, word: str, track_unknown: bool = False) -> str
```

**Параметры:**
- `word` — английское слово
- `track_unknown` — запоминать неизвестные слова

**Возвращает:**
- Русская транслитерация

### add_custom_terms

```python
def add_custom_terms(self, terms: dict[str, str]) -> None
```

Добавляет кастомные термины.

### get_unknown_words

```python
def get_unknown_words(self) -> dict[str, str]
```

Возвращает слова, обработанные через fallback.

### clear_unknown_words

```python
def clear_unknown_words(self) -> None
```

Очищает список неизвестных слов.

## Словарь IT_TERMS

150+ IT-терминов с устоявшимся произношением:

```python
IT_TERMS = {
    # Разработка
    "feature": "фича",
    "bug": "баг",
    "fix": "фикс",
    "commit": "коммит",
    "branch": "бранч",
    "merge": "мёрдж",
    "push": "пуш",
    "pull": "пулл",
    "deploy": "деплой",
    "release": "релиз",

    # Технологии
    "docker": "докер",
    "kubernetes": "кубернетис",
    "python": "пайтон",
    "javascript": "джаваскрипт",
    "react": "риакт",
    "node": "нода",
    "github": "гитхаб",
    "gitlab": "гитлаб",

    # Архитектура
    "backend": "бэкенд",
    "frontend": "фронтенд",
    "database": "датабейс",
    "cache": "кэш",
    "queue": "кью",
    "proxy": "прокси",
    "server": "сервер",
    "client": "клиент",

    # Процессы
    "review": "ревью",
    "refactor": "рефакторинг",
    "debug": "дебаг",
    "test": "тест",
    "build": "билд",
    "pipeline": "пайплайн",

    # ... и ещё 100+ терминов
}
```

## Fallback транслитерация

Для слов не из словаря применяется автоматическая транслитерация:

```python
# Таблица соответствий
TRANSLIT_MAP = {
    'a': 'а', 'b': 'б', 'c': 'к', 'd': 'д', 'e': 'е',
    'f': 'ф', 'g': 'г', 'h': 'х', 'i': 'и', 'j': 'дж',
    'k': 'к', 'l': 'л', 'm': 'м', 'n': 'н', 'o': 'о',
    'p': 'п', 'q': 'к', 'r': 'р', 's': 'с', 't': 'т',
    'u': 'у', 'v': 'в', 'w': 'в', 'x': 'кс', 'y': 'и',
    'z': 'з',
}

# Особые сочетания
'th' → 'т'
'sh' → 'ш'
'ch' → 'ч'
'ph' → 'ф'
'ck' → 'к'
```

## Многословные фразы

```python
PHRASES = {
    "pull request": "пулл реквест",
    "code review": "код ревью",
    "hot fix": "хот фикс",
    "open source": "опен сорс",
}
```

## Добавление терминов

### Временно (в runtime)

```python
normalizer = EnglishNormalizer()
normalizer.IT_TERMS["myterm"] = "май терм"
```

### Через конфигурацию Pipeline

```python
config = PipelineConfig(
    custom_it_terms={
        "terraform": "терраформ",
        "ansible": "ансибл",
    }
)
pipeline = TTSPipeline(config)
```

### Постоянно (в код)

Добавьте в `src/ruvox/tts_pipeline/normalizers/english.py`:

```python
IT_TERMS = {
    # ... существующие
    "newterm": "нью терм",
}
```

## Отслеживание неизвестных слов

```python
pipeline = TTSPipeline()
pipeline.process("Using terraform and ansible")

unknown = pipeline.get_unknown_words()
# {'terraform': 'тэрраформ', 'ansible': 'ансибл'}

# Вывести предупреждения
pipeline.print_warnings()
# Следующие слова были транслитерированы автоматически:
#   ansible → ансибл
#   terraform → тэрраформ
# Добавьте их в словарь IT_TERMS для точного произношения.
```

## Тестирование

```bash
uv run pytest tests/tts_pipeline/test_english.py -v
```

121 тест покрывает:
- Словарные термины
- Fallback транслитерацию
- Многословные фразы
- Регистр
- Особые случаи
