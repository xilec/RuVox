# Установка и запуск

## Требования

- **Python 3.11+**
- **NixOS** (рекомендуется) или Linux с установленными зависимостями
- **~500MB** свободного места (модель Silero загружается при первом запуске)

## Установка на NixOS

```bash
# Клонирование репозитория
git clone <repo-url>
cd ruvox

# Вход в dev-окружение
nix-shell

# Установка Python-зависимостей
uv sync --extra ui --extra tts

# Запуск
uv run ruvox
```

## Установка на других дистрибутивах

### Системные зависимости

```bash
# Ubuntu/Debian
sudo apt install python3.11 python3.11-venv \
    qt6-base-dev libmpv-dev \
    wl-clipboard  # для Wayland

# Fedora
sudo dnf install python3.11 \
    qt6-qtbase-devel mpv-devel \
    wl-clipboard
```

### Python-зависимости

```bash
# Создание виртуального окружения
python3.11 -m venv .venv
source .venv/bin/activate

# Установка (с pip)
pip install -e ".[ui,tts]"

# Или с uv
pip install uv
uv sync --extra ui --extra tts
```

## Запуск

```bash
# Через uv (рекомендуется)
uv run ruvox

# Через установленный пакет
ruvox

# Напрямую
python -m ruvox.ui.main
```

## Первый запуск

При первом запуске:

1. **Загрузка модели** — Silero TTS (~100MB) загружается из интернета
2. **Создание директорий** — `~/.cache/ruvox/` для аудио и логов
3. **Настройка хоткеев** — регистрация глобальных горячих клавиш

## Директории данных

```
~/.cache/ruvox/
├── audio/              # Сгенерированные аудиофайлы
│   ├── <uuid>.wav      # Аудио
│   └── <uuid>.timestamps.json  # Временные метки слов
├── logs/
│   └── app.log         # Логи приложения
└── history.json        # История записей
```

## Системный трей

Приложение работает в системном трее:

- **Левый клик** — показать/скрыть окно
- **Правый клик** — контекстное меню:
  - Читать сразу
  - Читать отложено
  - Настройки
  - Выход

## Автозапуск (опционально)

### KDE Plasma

```bash
# Создать файл автозапуска
mkdir -p ~/.config/autostart
cat > ~/.config/autostart/ruvox.desktop << EOF
[Desktop Entry]
Type=Application
Name=RuVox
Exec=/path/to/ruvox
Hidden=false
NoDisplay=false
X-KDE-autostart-phase=2
EOF
```

### systemd user service

```bash
# ~/.config/systemd/user/ruvox.service
[Unit]
Description=RuVox
After=graphical-session.target

[Service]
ExecStart=/path/to/ruvox
Restart=on-failure

[Install]
WantedBy=default.target
```

```bash
systemctl --user enable ruvox
systemctl --user start ruvox
```

## Решение проблем

### Модель не загружается

```bash
# Проверьте подключение к интернету
# Модель загружается с torch.hub

# Ручная загрузка
python -c "import torch; torch.hub.load('snakers4/silero-models', 'silero_tts', language='ru', speaker='v5_ru')"
```

### Нет звука

```bash
# Проверьте наличие libmpv
python -c "import mpv; p = mpv.MPV(video=False); p.terminate(); print('OK')"

# Установите mpv/libmpv
sudo apt install libmpv-dev   # Ubuntu/Debian
sudo dnf install mpv-devel    # Fedora
```

### Хоткеи не работают (Wayland)

Глобальные хоткеи на Wayland требуют xdg-desktop-portal:

```bash
# Проверьте portal
systemctl --user status xdg-desktop-portal

# KDE
sudo apt install xdg-desktop-portal-kde
```

### Буфер обмена пустой (Wayland)

Приложение использует `wl-paste` как fallback:

```bash
# Установите wl-clipboard
sudo apt install wl-clipboard

# Проверьте
wl-paste
```
