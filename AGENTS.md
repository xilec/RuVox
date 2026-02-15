# RuVox - Инструкции для разработки

> **ВАЖНО:** Всегда отвечай на русском языке.

## Обзор проекта

**RuVox** — приложение для озвучивания технических текстов. Включает:
- **Desktop UI** (PyQt6) — системный трей, очередь, плеер
- **TTS Pipeline** — нормализация текста для Silero TTS

### Проблема → Решение

```
"Вызови getUserData() через API"
        ↓
"Вызови гет юзер дата через эй пи ай"
```

## Документация

Подробная документация: [`docs/`](docs/index.md)

| Раздел | Описание |
|--------|----------|
| [Сценарии использования](docs/use-cases.md) | Как использовать приложение |
| [UI](docs/ui/index.md) | Архитектура интерфейса |
| [TTS Pipeline](docs/tts-pipeline/index.md) | Движок нормализации |
| [Разработка](docs/development.md) | Настройка окружения |
| [Contributing](docs/contributing.md) | Как внести вклад |

## Быстрый старт

```bash
nix-shell                      # Dev окружение
uv sync --extra ui --extra tts # Зависимости
uv run ruvox             # Запуск UI
uv run pytest                  # Тесты
```

## Структура проекта

```
src/ruvox/
├── tts_pipeline/              # Нормализация текста
│   ├── pipeline.py            # TTSPipeline
│   ├── tracked_text.py        # Отслеживание позиций символов + CharMapping
│   ├── word_mapping.py        # WordMapping для слов
│   ├── config.py              # PipelineConfig
│   └── normalizers/           # Нормализаторы
│       ├── english.py         # Английские слова
│       ├── abbreviations.py   # Аббревиатуры (API, HTTP)
│       ├── numbers.py         # Числа, даты, размеры
│       ├── symbols.py         # Операторы, символы
│       ├── urls.py            # URL, email, IP
│       └── code.py            # camelCase, snake_case
└── ui/                        # Desktop приложение
    ├── app.py                 # QApplication
    ├── main.py                # Точка входа
    ├── main_window.py         # MainWindow
    ├── widgets/               # Qt виджеты
    │   ├── player.py          # Аудио плеер
    │   ├── queue_list.py      # Список очереди
    │   └── text_viewer.py     # Просмотр текста + Mermaid
    ├── dialogs/               # Диалоговые окна
    │   ├── settings.py        # Настройки
    │   └── mermaid_preview.py # Интерактивный просмотр Mermaid
    ├── services/              # Сервисы
    │   ├── tts_worker.py      # TTS генерация
    │   ├── storage.py         # Хранение истории
    │   ├── hotkeys.py         # Глобальные хоткеи
    │   ├── clipboard.py       # Работа с буфером
    │   ├── cleanup.py         # Очистка кэша
    │   ├── mermaid_renderer.py # Рендеринг Mermaid → SVG/pixmap
    │   └── logging_service.py # Логирование
    └── models/                # Модели данных
        ├── entry.py           # TextEntry
        └── config.py          # UIConfig
```

## Основные команды

```bash
# Тесты
./scripts/test.sh              # Все тесты
./scripts/test.sh module numbers  # Конкретный модуль
./scripts/test.sh match 'url'  # По паттерну

# Приложение
uv run ruvox             # UI
uv run python scripts/tts_generate.py FILE  # Генерация аудио
```

## Ключевые классы

### TTSPipeline

```python
from ruvox.tts_pipeline import TTSPipeline

pipeline = TTSPipeline()
result = pipeline.process("Text with API and 123")

# С маппингом позиций (для подсветки слов)
result, mapping = pipeline.process_with_char_mapping(text)
```

### UI Application

```python
# Точка входа: src/ruvox/ui/main.py
# Главное окно: MainWindow
# Сервисы: TTSWorker, Storage, HotkeyService
```

## Правила разработки

- Код должен соответствовать стилю, проверяемому `ruff`. Перед коммитом запускай `ruff check .` и `ruff format --check .`, исправляй ошибки.
- Опциональная проверка типов: `mypy src/ruvox/tts_pipeline/`. Проверяет только аннотированный код, неаннотированные функции пропускает.
- [Логирование и обработка ошибок](ai/rules/error_handling_and_logs_rules.md)

### TTS Pipeline: Mermaid-диаграммы

Mermaid-блоки (` ```mermaid ... ``` `) **не озвучиваются**. Pipeline заменяет их на маркер `"Тут мермэйд диаграмма"`, чтобы обозначить наличие диаграммы. Пользователь может приостановить чтение и рассмотреть диаграмму в UI.

### TTS Pipeline: английский текст

Silero TTS **не умеет читать английский**. Весь английский текст должен быть транслитерирован в кириллицу до передачи в TTS-движок. Это значит:

- Во всех местах, где может появиться английский текст (ссылки, код, заголовки и т.д.), должен быть **fallback для транслитерации**
- Если обработка текста (например, удаление Markdown-синтаксиса) оставляет английские слова, они должны оставаться доступными для дальнейшей нормализации английским нормализатором
- В `TrackedText`: нельзя заменять фрагмент целиком, если внутри есть английский текст, который ещё не транслитерирован. Вместо этого удалять синтаксис (скобки, URL и т.д.) отдельными операциями, оставляя текстовое содержимое нетронутым

## Тесты

Актуальный список тестов (без запуска):

```bash
uv run pytest --collect-only -q
```
