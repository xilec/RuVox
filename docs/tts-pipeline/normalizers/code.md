# CodeIdentifierNormalizer

Обработка идентификаторов кода и блоков кода.

## Импорт

```python
from fast_tts_rus.tts_pipeline.normalizers import CodeIdentifierNormalizer, CodeBlockHandler
```

## CodeIdentifierNormalizer

Разбиение и транслитерация идентификаторов.

### Использование

```python
normalizer = CodeIdentifierNormalizer()

normalizer.normalize_camel_case("getUserData")   # → "гет юзер дата"
normalizer.normalize_snake_case("get_user_data") # → "гет юзер дата"
normalizer.normalize_kebab_case("my-component")  # → "май компонент"
```

### API

#### normalize_camel_case

```python
def normalize_camel_case(self, identifier: str) -> str
```

Разбивает camelCase и PascalCase.

```python
normalizer.normalize_camel_case("getUserData")     # → "гет юзер дата"
normalizer.normalize_camel_case("XMLHttpRequest")  # → "экс эм эл хттп реквест"
normalizer.normalize_camel_case("useState")        # → "юз стейт"
```

#### normalize_snake_case

```python
def normalize_snake_case(self, identifier: str) -> str
```

Разбивает snake_case.

```python
normalizer.normalize_snake_case("get_user_data")  # → "гет юзер дата"
normalizer.normalize_snake_case("__init__")       # → "инит"
normalizer.normalize_snake_case("MAX_VALUE")      # → "макс вэлью"
```

#### normalize_kebab_case

```python
def normalize_kebab_case(self, identifier: str) -> str
```

Разбивает kebab-case.

```python
normalizer.normalize_kebab_case("my-component")    # → "май компонент"
normalizer.normalize_kebab_case("button-primary")  # → "баттон праймари"
normalizer.normalize_kebab_case("vue-router")      # → "вью роутер"
```

### Алгоритм разбиения

**camelCase:**
1. Разбиение на границе строчная→прописная: `getUser` → `get`, `User`
2. Группировка последовательных прописных: `XMLHttp` → `XML`, `Http`

**snake_case:**
1. Разбиение по `_`
2. Удаление пустых частей (для `__init__`)

**kebab-case:**
1. Разбиение по `-`

### Транслитерация слов

Каждое слово обрабатывается через `EnglishNormalizer`:
- Словарные термины: `get` → "гет", `user` → "юзер"
- Fallback: автоматическая транслитерация

## CodeBlockHandler

Обработка блоков кода в Markdown.

### Использование

```python
handler = CodeBlockHandler(mode="brief")

code = "print('hello')"
handler.process(code, language="python")
# → "далее следует пример кода на пайтон"
```

### Режимы

#### brief — краткий

Заменяет блок кода на описание:

```python
handler = CodeBlockHandler(mode="brief")
handler.process("def foo(): pass", "python")
# → "далее следует пример кода на пайтон"

handler.process("npm install", "bash")
# → "далее следует пример кода на баш"
```

**Языки:**

| Язык | Произношение |
|------|--------------|
| python | пайтон |
| javascript | джаваскрипт |
| typescript | тайпскрипт |
| bash | баш |
| shell | шелл |
| sql | эс кью эл |
| json | джейсон |
| yaml | ямл |
| html | эйч ти эм эл |
| css | си эс эс |

#### full — полный

Нормализует и озвучивает содержимое:

```python
handler = CodeBlockHandler(mode="full")
handler.process("print('hello')", "python")
# → "принт открывающая скобка кавычка хелло кавычка закрывающая скобка"
```

### API

#### set_mode

```python
def set_mode(self, mode: str) -> None
```

Переключает режим (`"full"` или `"brief"`).

#### process

```python
def process(self, code: str, language: str | None = None) -> str
```

Обрабатывает блок кода.

## Inline код

Inline код (`` `code` ``) обрабатывается в Pipeline:

```python
pipeline.process("Запустите `docker run`")
# → "Запустите докер ран"

pipeline.process("Используйте `getUserData()`")
# → "Используйте гет юзер дата"
```

### Обработка inline кода

1. Определение типа идентификатора:
   - `_` → snake_case
   - `-` → kebab-case
   - смешанный регистр → camelCase

2. Разбиение и транслитерация

3. Специальные символы:
   - Греческие буквы: `α` → "альфа"
   - Операторы: `->` → "стрелка"

## Словарь CODE_WORDS

Специфичные для кода слова:

```python
CODE_WORDS = {
    "def": "деф",
    "class": "класс",
    "import": "импорт",
    "from": "фром",
    "return": "ретурн",
    "if": "иф",
    "else": "элс",
    "for": "фор",
    "while": "вайл",
    "try": "трай",
    "except": "эксепт",
    "with": "виз",
    "as": "эз",
    "in": "ин",
    "not": "нот",
    "and": "энд",
    "or": "ор",
    "true": "тру",
    "false": "фолс",
    "none": "нан",
    "null": "налл",
    "self": "селф",
    "this": "зис",
    "new": "нью",
    "var": "вар",
    "let": "лет",
    "const": "конст",
    "function": "фанкшн",
    "async": "эсинк",
    "await": "эвейт",
}
```

## Тестирование

```bash
uv run pytest tests/tts_pipeline/test_code.py -v
```

104 теста покрывают:
- camelCase разбиение
- snake_case разбиение
- kebab-case разбиение
- Блоки кода (brief/full)
- Inline код
- Специальные случаи
