# Fast TTS RUS - Инструкции для разработки

> **ВАЖНО:** Всегда отвечай на русском языке.

## Обзор проекта

**Fast TTS RUS** — приложение для озвучивания технических текстов. Включает:
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
uv run fast-tts-ui             # Запуск UI
uv run pytest                  # Тесты
```

## Структура проекта

```
src/fast_tts_rus/
├── tts_pipeline/           # Нормализация текста
│   ├── pipeline.py         # TTSPipeline
│   ├── tracked_text.py     # Отслеживание позиций
│   └── normalizers/        # Нормализаторы
└── ui/                     # Desktop приложение
    ├── widgets/            # Qt виджеты
    ├── services/           # TTS, storage, hotkeys
    └── models/             # Entry, Config
```

## Основные команды

```bash
# Тесты
./scripts/test.sh              # Все тесты
./scripts/test.sh module numbers  # Конкретный модуль
./scripts/test.sh match 'url'  # По паттерну

# Приложение
uv run fast-tts-ui             # UI
uv run python scripts/tts_generate.py FILE  # Генерация аудио
```

## Ключевые классы

### TTSPipeline

```python
from fast_tts_rus.tts_pipeline import TTSPipeline

pipeline = TTSPipeline()
result = pipeline.process("Text with API and 123")

# С маппингом позиций (для подсветки слов)
result, mapping = pipeline.process_with_char_mapping(text)
```

### UI Application

```python
# Точка входа: src/fast_tts_rus/ui/main.py
# Главное окно: MainWindow
# Сервисы: TTSWorker, Storage, HotkeyService
```

## Правила разработки

- [Логирование и обработка ошибок](ai/rules/error_handling_and_logs_rules.md)

## Тесты

| Модуль | Тестов | Файл |
|--------|--------|------|
| english | 121 | test_english.py |
| abbreviations | 108 | test_abbreviations.py |
| numbers | 108 | test_numbers.py |
| urls | 62 | test_urls.py |
| symbols | 74 | test_symbols.py |
| code | 104 | test_code.py |
| pipeline | 55 | test_pipeline.py |
| tracked_text | 31 | test_tracked_text.py |
