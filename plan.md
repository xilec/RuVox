# План реализации Fast TTS Pipeline

## Цель

Реализовать пайплайн предобработки текста для TTS Silero, который преобразует технические тексты (документация, статьи, мануалы) в форму, пригодную для качественного озвучивания.

## Текущее состояние

- [x] Анализ проблемы (TranslitEnAnalysis.md)
- [x] Проектирование пайплайна (Pipeline.md)
- [x] Структура проекта инициализирована
- [x] 626 тестов написаны (TDD)
- [x] Документация для разработки (AGENTS.md)
- [x] Реализация модулей (8/8)

## Фазы реализации

### Фаза 1: Базовые нормализаторы (независимые)

Модули без внешних зависимостей на другие нормализаторы.

#### 1.1 NumberNormalizer
**Файл:** `src/fast_tts/normalizers/numbers.py`
**Тесты:** `tests/test_numbers.py` (108 тестов)
**Зависимости:** `num2words`

**Задачи:**
- [ ] Реализовать `normalize_number()` — целые числа через num2words
- [ ] Реализовать `normalize_float()` — дробные с "точка"
- [ ] Реализовать `normalize_percentage()` — с правильным склонением "процент/процента/процентов"
- [ ] Реализовать `normalize_range()` — "от X до Y"
- [ ] Реализовать `normalize_size()` — единицы измерения (KB, MB, GB, ms, sec)
- [ ] Реализовать `normalize_version()` — версии софта
- [ ] Реализовать `normalize_date()` — ISO и европейский формат
- [ ] Реализовать `normalize_time()` — часы и минуты со склонением

**Критерий готовности:** `./scripts/test.sh module numbers` — все тесты зелёные

---

#### 1.2 SymbolNormalizer
**Файл:** `src/fast_tts/normalizers/symbols.py`
**Тесты:** `tests/test_symbols.py` (74 теста)
**Зависимости:** нет

**Задачи:**
- [ ] Создать словарь OPERATORS (->. =>, >=, <=, !=, ==, &&, ||, etc.)
- [ ] Создать словарь BRACKETS с направлением (открывающая/закрывающая)
- [ ] Создать словарь PUNCTUATION (!, ?, ..., -, _, /, \)
- [ ] Создать словарь SPECIAL (@, #, $, *, &)
- [ ] Реализовать `normalize()` с поиском по словарям

**Критерий готовности:** `./scripts/test.sh module symbols` — все тесты зелёные

---

#### 1.3 AbbreviationNormalizer
**Файл:** `src/fast_tts/normalizers/abbreviations.py`
**Тесты:** `tests/test_abbreviations.py` (108 тестов)
**Зависимости:** нет

**Задачи:**
- [ ] Создать словарь AS_WORD (JSON, YAML, REST, AJAX, CRUD, GIF, RAM, etc.)
- [ ] Создать словарь LETTER_MAP (a→эй, b→би, c→си, ..., w→дабл ю, ...)
- [ ] Реализовать `normalize()`:
  - Проверить в AS_WORD (case-insensitive)
  - Иначе — побуквенное произношение через LETTER_MAP

**Критерий готовности:** `./scripts/test.sh module abbreviations` — все тесты зелёные

---

### Фаза 2: EnglishNormalizer (ключевой модуль)

#### 2.1 EnglishNormalizer
**Файл:** `src/fast_tts/normalizers/english.py`
**Тесты:** `tests/test_english.py` (121 тест)
**Зависимости:** опционально g2p-en

**Задачи:**
- [ ] Создать словарь IT_TERMS (150+ терминов):
  - Git/VCS: feature, branch, merge, commit, push, pull, checkout, rebase
  - Dev process: review, deploy, release, debug, bug, fix, sprint, scrum
  - Architecture: framework, library, callback, promise, handler, middleware
  - Data: cache, queue, array, string, boolean, null, default
  - Infrastructure: docker, kubernetes, nginx, backup, server, client
  - Testing: test, mock, stub, spec
  - Build: build, bundle, compile, lint, webpack
  - Languages: python, javascript, typescript, rust, golang
  - Frameworks: react, angular, vue, django, flask, fastapi
  - Tools: github, gitlab, jira, slack, figma, postman
- [ ] Создать словарь MULTI_WORD_PHRASES (pull request, code review, etc.)
- [ ] Реализовать `normalize()`:
  - Проверить в MULTI_WORD_PHRASES (longest match first)
  - Проверить в IT_TERMS (case-insensitive)
  - Опционально: G2P fallback для неизвестных слов
- [ ] Добавить поддержку custom_it_terms из конфига

**Критерий готовности:** `./scripts/test.sh module english` — все тесты зелёные (кроме G2P если не реализован)

---

### Фаза 3: Составные нормализаторы

Модули, использующие базовые нормализаторы.

#### 3.1 URLPathNormalizer
**Файл:** `src/fast_tts/normalizers/urls.py`
**Тесты:** `tests/test_urls.py` (62 теста)
**Зависимости:** NumberNormalizer, внутренние словари

**Задачи:**
- [ ] Создать словарь PROTOCOLS (http→эйч ти ти пи, https, ftp, ssh, git)
- [ ] Создать словарь TLD_MAP (com→ком, org→орг, ru→ру, io→ай оу, dev→дев)
- [ ] Реализовать `normalize_url()`:
  - Парсинг через urllib.parse
  - Протокол + "двоеточие слэш слэш"
  - Домен с "точка" между частями
  - Путь с "слэш" между сегментами
  - Порт через NumberNormalizer
- [ ] Реализовать `normalize_email()`:
  - user + "собака" + domain + "точка" + tld
- [ ] Реализовать `normalize_ip()`:
  - Октеты через NumberNormalizer + "точка"
- [ ] Реализовать `normalize_filepath()`:
  - Определить разделитель (/ или \)
  - "слэш" или "бэкслэш"
  - Обработка ~, ., ..
  - Расширения файлов с "точка"

**Критерий готовности:** `./scripts/test.sh module urls` — все тесты зелёные

---

#### 3.2 CodeIdentifierNormalizer
**Файл:** `src/fast_tts/normalizers/code.py`
**Тесты:** `tests/test_code.py` (104 теста, часть)
**Зависимости:** EnglishNormalizer

**Задачи:**
- [ ] Реализовать `normalize_camel_case()`:
  - Regex split на границах регистра: `[A-Z]?[a-z]+|[A-Z]+(?=[A-Z][a-z]|\d|\W|$)|\d+`
  - Каждое слово через EnglishNormalizer
  - Числа через NumberNormalizer
- [ ] Реализовать `normalize_snake_case()`:
  - Split по `_`
  - Фильтрация пустых (для `__init__`)
  - Каждое слово через EnglishNormalizer
- [ ] Реализовать `normalize_kebab_case()`:
  - Split по `-`
  - Каждое слово через EnglishNormalizer

**Критерий готовности:** Тесты camelCase, snake_case, kebab_case проходят

---

#### 3.3 CodeBlockHandler
**Файл:** `src/fast_tts/normalizers/code.py`
**Тесты:** `tests/test_code.py` (оставшаяся часть)
**Зависимости:** весь пайплайн (для full mode)

**Задачи:**
- [ ] Создать словарь LANGUAGE_NAMES (python→пайтон, js→джаваскрипт, etc.)
- [ ] Реализовать `set_mode()` — переключение full/brief
- [ ] Реализовать `process()`:
  - Brief mode: "далее следует пример кода на {язык}" или "далее следует блок кода"
  - Full mode: вызов пайплайна нормализации для содержимого

**Критерий готовности:** `./scripts/test.sh module code` — все тесты зелёные

---

### Фаза 4: Интеграция пайплайна

#### 4.1 Tokenizer (новый модуль)
**Файл:** `src/fast_tts/tokenizer.py`

**Задачи:**
- [ ] Определить enum TokenType со всеми типами
- [ ] Создать dataclass Token (text, token_type, start, end, metadata)
- [ ] Реализовать Tokenizer.tokenize():
  - Паттерны в порядке приоритета
  - Возврат списка Token

---

#### 4.2 Preprocessor (новый модуль)
**Файл:** `src/fast_tts/preprocessor.py`

**Задачи:**
- [ ] Реализовать `preprocess()`:
  - Удаление BOM
  - Нормализация пробелов
  - Унификация кавычек («» → "", "" → "")
  - Унификация тире (— → -, – → -)
  - Удаление множественных переносов

---

#### 4.3 StructureParser (новый модуль)
**Файл:** `src/fast_tts/parser.py`

**Задачи:**
- [ ] Реализовать парсинг markdown:
  - Извлечение code blocks (```...```)
  - Извлечение inline code (`...`)
  - Парсинг заголовков (##)
  - Парсинг списков (-, *, 1.)
  - Парсинг ссылок [text](url)

---

#### 4.4 TTSPipeline
**Файл:** `src/fast_tts/pipeline.py`
**Тесты:** `tests/test_pipeline.py` (55 тестов)

**Задачи:**
- [ ] Инициализация всех нормализаторов
- [ ] Реализовать `process()`:
  1. Preprocess входного текста
  2. Structure parsing (выделение code blocks, etc.)
  3. Для каждого сегмента:
     - Code block → CodeBlockHandler
     - Text → tokenize → normalize each token → join
  4. Post-process (очистка пробелов, пунктуация)
- [ ] Реализовать `set_code_mode()`
- [ ] Поддержка custom dictionaries из config

**Критерий готовности:** `./scripts/test.sh module pipeline` — все тесты зелёные

---

### Фаза 5: Финализация

#### 5.1 Оптимизация
- [ ] Профилирование на больших текстах
- [ ] Кэширование результатов (опционально)
- [ ] Ленивая инициализация G2P

#### 5.2 Документация
- [ ] Обновить README.md с примерами использования
- [ ] Docstrings для публичных методов
- [ ] Примеры в examples/

#### 5.3 Интеграция с Silero
- [ ] Тестирование на реальных текстах
- [ ] Сравнение качества до/после
- [ ] Интеграционный скрипт

---

## Метрики прогресса

| Фаза | Модуль | Тестов | Статус |
|------|--------|--------|--------|
| 1.1 | NumberNormalizer | 108 | ✅ DONE |
| 1.2 | SymbolNormalizer | 68 | ✅ DONE |
| 1.3 | AbbreviationNormalizer | 108 | ✅ DONE |
| 2.1 | EnglishNormalizer | 121 | ✅ DONE |
| 3.1 | URLPathNormalizer | 62 | ✅ DONE |
| 3.2 | CodeIdentifierNormalizer | ~50 | ✅ DONE |
| 3.3 | CodeBlockHandler | ~54 | ✅ DONE |
| 4.4 | TTSPipeline | 55 | ✅ DONE |
| **TOTAL** | | **626** | **100%** |

## Команды для отслеживания

```bash
# Текущий прогресс (passed/total)
./scripts/test.sh quick 2>&1 | tail -1

# Прогресс по модулю
./scripts/test.sh module numbers 2>&1 | tail -1

# Подробный отчёт
./scripts/test.sh cov
```

## Приоритеты

1. **Критично:** NumberNormalizer, SymbolNormalizer, AbbreviationNormalizer, EnglishNormalizer
2. **Важно:** URLPathNormalizer, CodeIdentifierNormalizer
3. **Желательно:** CodeBlockHandler (full mode), G2P fallback
4. **Опционально:** Оптимизация, кэширование

## Риски

| Риск | Вероятность | Митигация |
|------|-------------|-----------|
| G2P качество для неизвестных слов | Средняя | Расширение словаря IT-терминов |
| Сложные вложенные структуры | Низкая | Приоритизация токенов |
| Производительность на больших текстах | Низкая | Профилирование, кэширование |
| Неучтённые паттерны | Средняя | Тестирование на реальных текстах |
