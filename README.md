# Fast TTS RUS

Приложение для озвучивания технических текстов на русском языке. Включает:
- **Desktop UI** (PyQt6) — системный трей, очередь, аудио плеер
- **TTS Pipeline** — нормализация текста для Silero TTS

## Проблема и решение

```
"Вызови getUserData() через API"
        ↓
"Вызови гет юзер дата через эй пи ай"
```

## Установка

```bash
nix-shell                      # Dev окружение
uv sync --extra ui --extra tts # Зависимости
```

## Использование

```bash
# Запуск UI приложения
uv run fast-tts-ui

# Тесты
uv run pytest

# Генерация аудио из файла
uv run python scripts/tts_generate.py FILE
```

## Документация

Подробная документация: [`docs/`](docs/index.md)

Инструкции для разработчиков: [`AGENTS.md`](AGENTS.md)
