# Разработка

Руководство по настройке окружения и разработке.

## Требования

- **NixOS** (рекомендуется) или Linux
- **Python 3.11+**
- **Git**

## Настройка окружения

### NixOS (рекомендуется)

```bash
# Клонирование
git clone <repo-url>
cd fast_tts_rus

# Вход в dev-окружение
nix-shell

# Установка зависимостей
uv sync                    # Базовые
uv sync --extra ui         # + UI (PyQt6)
uv sync --extra tts        # + TTS (torch, silero)
uv sync --extra g2p        # + G2P (опционально)
```

`nix-shell` предоставляет:
- Python 3.11
- uv (пакетный менеджер)
- Qt6
- GStreamer
- wl-clipboard

### Другие дистрибутивы

```bash
# Системные зависимости (Ubuntu)
sudo apt install python3.11 python3.11-venv \
    qt6-base-dev qt6-multimedia-dev \
    gstreamer1.0-plugins-good gstreamer1.0-plugins-bad \
    wl-clipboard

# Python окружение
python3.11 -m venv .venv
source .venv/bin/activate
pip install uv
uv sync --extra ui --extra tts
```

## Структура проекта

```
fast_tts_rus/
├── src/fast_tts_rus/
│   ├── tts_pipeline/       # Движок нормализации
│   │   ├── normalizers/    # Нормализаторы
│   │   ├── pipeline.py     # TTSPipeline
│   │   └── tracked_text.py # TrackedText
│   └── ui/                 # Desktop приложение
│       ├── widgets/        # Qt виджеты
│       ├── services/       # Сервисы
│       └── models/         # Модели данных
├── tests/
│   └── tts_pipeline/       # Тесты
├── docs/                   # Документация
├── scripts/                # Утилиты
├── pyproject.toml          # Конфигурация проекта
└── shell.nix               # NixOS окружение
```

## Запуск тестов

### Все тесты

```bash
uv run pytest
```

### Быстрый режим

```bash
./scripts/test.sh quick
```

### Конкретный модуль

```bash
./scripts/test.sh module numbers
./scripts/test.sh module english
./scripts/test.sh module pipeline
```

### По паттерну

```bash
./scripts/test.sh match 'url'
./scripts/test.sh match 'camel_case'
```

### С покрытием

```bash
./scripts/test.sh cov
```

### Остановка на первой ошибке

```bash
./scripts/test.sh stop
```

### Только упавшие

```bash
./scripts/test.sh failed
```

## Запуск приложения

```bash
# UI приложение
uv run fast-tts-ui

# Генерация аудио из файла
uv run python scripts/tts_generate.py input.txt
```

## TDD Workflow

1. **Выбрать модуль** для реализации
2. **Запустить тесты**: `./scripts/test.sh module <name>`
3. **Увидеть ожидаемые входы/выходы** в failing тестах
4. **Реализовать** до прохождения тестов
5. **Рефакторинг** при зелёных тестах
6. **Перейти к следующему** модулю

### Рекомендуемый порядок

1. NumberNormalizer — простой, использует num2words
2. SymbolNormalizer — словарь
3. AbbreviationNormalizer — словарь + буквы
4. EnglishNormalizer — словарь + G2P
5. URLPathNormalizer — парсинг + другие нормализаторы
6. CodeIdentifierNormalizer — regex + EnglishNormalizer
7. TTSPipeline — интеграция

## Отладка

### Логи приложения

```bash
tail -f ~/.cache/fast-tts-rus/logs/app.log
```

### Debug скрипты

```bash
# Отладка маппинга
uv run python scripts/test_mapping_debug.py
```

### Python debugger

```python
import pdb; pdb.set_trace()
# или
breakpoint()
```

## Зависимости

### pyproject.toml

```toml
[project]
dependencies = [
    "num2words>=0.5.12",
]

[project.optional-dependencies]
ui = [
    "PyQt6>=6.6.0",
    "markdown>=3.5",
    "dasbus>=1.7",
]
tts = [
    "torch>=2.0",
    "numpy>=1.24",
    "scipy>=1.10",
]
g2p = [
    "g2p-en>=2.1.0",
    "eng-to-ipa>=0.0.2",
]
```

### Добавление зависимости

```bash
# Редактировать pyproject.toml, затем:
uv sync
```

## Code Style

- **Python 3.11+** — используйте современный синтаксис
- **Type hints** — для публичных API
- **Docstrings** — для классов и публичных методов
- **Форматирование** — согласованное с существующим кодом

## Тестирование изменений

Перед коммитом:

```bash
# Все тесты
uv run pytest

# Проверка UI (если менялось)
uv run fast-tts-ui
```

## Git workflow

```bash
# Новая ветка
git checkout -b feature/my-feature

# Коммит
git add <files>
git commit -m "Описание изменений"

# Push
git push -u origin feature/my-feature
```
