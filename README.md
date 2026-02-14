# RuVox

Приложение для озвучивания технических текстов на русском языке. Преобразует английские термины, аббревиатуры, код и числа в читаемый русский текст и передаёт его в [Silero TTS](https://github.com/snakers4/silero-models).

```
"Вызови getUserData() через API"  →  "Вызови гет юзер дата через эй пи ай"
```

## Возможности

- **Нормализация текста** — английские слова, camelCase/snake_case, аббревиатуры (API, HTTP), числа, даты, URL, email
- **Desktop UI** (PyQt6) — системный трей, очередь озвучивания, плеер с управлением
- **Markdown** — отображение и озвучивание Markdown-документов
- **Mermaid-диаграммы** — визуализация диаграмм в интерфейсе
- **Подсветка слов** — синхронная подсветка читаемого слова в тексте
- **Глобальные хоткеи** — озвучивание из буфера обмена по горячей клавише
- **Аудио** — воспроизведение через mpv (scaletempo2)

## Требования

- **ОС:** Linux (X11/Wayland)
- **Python:** 3.11+
- **Nix:** рекомендуется для dev-окружения (shell.nix настраивает все системные зависимости)
- **Без Nix:** потребуется вручную установить Qt6, libmpv, PulseAudio/PipeWire, D-Bus, GObject Introspection и другие системные библиотеки (см. `buildInputs` в `shell.nix`)

## Установка

```bash
# С Nix (рекомендуется) — автоматически создаёт venv и ставит зависимости
nix-shell

# Без Nix — вручную
uv venv
uv sync --all-extras           # Все зависимости (ui, tts, g2p, dev)
```

> **NixOS:** shell.nix собирает пакет `regex` из исходников (`--no-binary-package regex`), т.к. предкомпилированные manylinux-колёса несовместимы с glibc NixOS.

## Использование

```bash
# Запуск UI приложения
uv run ruvox

# Тесты
uv run pytest

# Генерация аудио из файла
uv run python scripts/tts_generate.py FILE
```

## Стек

| Компонент | Технология |
|-----------|-----------|
| TTS-движок | [Silero TTS](https://github.com/snakers4/silero-models) (PyTorch) |
| UI | PyQt6 + PyQt6-WebEngine |
| Аудио | libmpv (python-mpv) |
| G2P | g2p-en (опционально, для улучшенной транслитерации) |
| Хоткеи | D-Bus (dasbus + PyGObject) |

## Документация

Подробная документация: [`docs/`](docs/index.md)

Инструкции для разработчиков: [`AGENTS.md`](AGENTS.md)
