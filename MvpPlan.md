# Fast TTS RUS — MVP UI Plan

## Обзор

Desktop-приложение для Linux/Wayland с system tray, глобальными хоткеями и интеграцией с TTS-пайплайном.

---

## Технологический стек

| Компонент | Технология | Причина выбора |
|-----------|------------|----------------|
| GUI Framework | PyQt6 | Нативная поддержка Wayland, системный стиль |
| Аудио | Qt Multimedia (QMediaPlayer) | Playback rate, seek, volume из коробки |
| Глобальные хоткеи | xdg-desktop-portal GlobalShortcuts | Стандарт для Wayland |
| Буфер обмена | QClipboard (PyQt6) | Работает на Wayland |
| Markdown | QTextBrowser + markdown lib | Рендеринг в HTML |
| Хранение | JSON + WAV файлы | Простота, прозрачность |
| Фоновые задачи | QThread / QThreadPool | Интеграция с Qt event loop |

### Зависимости

```toml
# pyproject.toml additions
[project.optional-dependencies]
ui = [
    "PyQt6>=6.6.0",
    "PyQt6-Qt6>=6.6.0",
    "markdown>=3.5",
    "dbus-python>=1.3.2",  # или dasbus для portal
]
```

### NixOS

```nix
# shell.nix additions
python3Packages.pyqt6
python3Packages.dbus-python
qt6.qtmultimedia
qt6.qtsvg            # SVG icons support
xdg-desktop-portal   # runtime
```

---

## Архитектура

```
fast_tts_rus/
├── src/fast_tts_rus/
│   ├── __init__.py              # Re-exports from tts_pipeline
│   │
│   ├── tts_pipeline/            # Существующий TTS пайплайн
│   │   ├── __init__.py
│   │   ├── config.py
│   │   ├── pipeline.py
│   │   └── normalizers/         # Нормализаторы
│   │
│   └── ui/
│       ├── __init__.py
│       ├── main.py              # Entry point, QApplication
│       ├── app.py               # Приложение: tray, портал, координация
│       ├── main_window.py       # Главное окно
│       │
│       ├── widgets/
│       │   ├── __init__.py
│       │   ├── player.py        # Аудиоплеер виджет
│       │   ├── text_viewer.py   # Markdown viewer с подсветкой
│       │   ├── queue_list.py    # Список очереди/истории
│       │   └── progress_bar.py  # Кастомный прогресс-бар
│       │
│       ├── dialogs/
│       │   ├── __init__.py
│       │   └── settings.py      # Диалог настроек
│       │
│       ├── services/
│       │   ├── __init__.py
│       │   ├── tts_worker.py    # Фоновый TTS в QThread
│       │   ├── storage.py       # Хранение истории и аудио
│       │   ├── cleanup.py       # Фоновая очистка
│       │   ├── hotkeys.py       # xdg-desktop-portal интеграция
│       │   └── timestamps.py    # Синхронизация текст ↔ аудио
│       │
│       ├── models/
│       │   ├── __init__.py
│       │   ├── entry.py         # TextEntry dataclass
│       │   └── config.py        # UIConfig dataclass
│       │
│       └── resources/
│           └── icons/
│               ├── tray.svg
│               ├── play.svg
│               ├── pause.svg
│               └── ...
```

---

## Модели данных

### TextEntry (models/entry.py)

```python
from dataclasses import dataclass, field
from datetime import datetime
from enum import Enum
from pathlib import Path

class EntryStatus(Enum):
    PENDING = "pending"          # Ожидает TTS
    PROCESSING = "processing"    # TTS выполняется
    READY = "ready"              # Аудио готово
    ERROR = "error"              # Ошибка TTS

@dataclass
class TextEntry:
    id: str                              # UUID
    original_text: str                   # Исходный текст
    normalized_text: str | None = None   # После нормализации
    status: EntryStatus = EntryStatus.PENDING
    created_at: datetime = field(default_factory=datetime.now)
    audio_generated_at: datetime | None = None
    audio_path: Path | None = None       # Путь к WAV
    timestamps_path: Path | None = None  # Путь к JSON с timestamps
    duration_sec: float | None = None    # Длительность аудио
    was_regenerated: bool = False        # Был ли перегенерирован вручную
    error_message: str | None = None     # Сообщение об ошибке
```

### UIConfig (models/config.py)

```python
from dataclasses import dataclass, field
from pathlib import Path

@dataclass
class UIConfig:
    # Пути
    cache_dir: Path = Path.home() / ".cache" / "fast_tts_rus"

    # Хоткеи (описание для portal)
    hotkey_read_now: str = "Control+t"
    hotkey_read_later: str = "Control+Shift+t"

    # TTS
    speaker: str = "xenia"        # aidar, baya, kseniya, xenia, eugene, random
    speech_rate: float = 1.0      # 0.5 - 2.0
    sample_rate: int = 48000      # 8000, 24000, 48000

    # Очистка
    history_days: int = 14        # Хранить тексты N дней
    audio_max_files: int = 5      # Максимум аудио-файлов
    audio_regenerated_hours: int = 24  # Хранить перегенерированные N часов

    # Поведение
    notify_on_ready: bool = True  # Уведомление при готовности (отложенный режим)

    # Плеер хоткеи (локальные, в окне)
    player_hotkeys: dict = field(default_factory=lambda: {
        "play_pause": "Space",
        "forward_5": "Right",
        "backward_5": "Left",
        "forward_30": "Shift+Right",
        "backward_30": "Shift+Left",
        "speed_up": "]",
        "speed_down": "[",
        "next_entry": "n",
        "prev_entry": "p",
        "repeat_sentence": "r",
    })
```

### Файловая структура хранилища

```
~/.cache/fast_tts_rus/
├── config.json              # UIConfig
├── history.json             # Список TextEntry (без audio_path содержимого)
├── audio/
│   ├── {uuid}.wav           # Аудио файлы
│   └── {uuid}.timestamps.json  # Timestamps для синхронизации
└── logs/
    └── app.log              # Логи (опционально)
```

### history.json формат

```json
{
  "version": 1,
  "entries": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "original_text": "Установите Docker версии >= 20.10",
      "normalized_text": "Установите докер версии больше или равно двадцать точка десять",
      "status": "ready",
      "created_at": "2024-01-15T14:30:00",
      "audio_generated_at": "2024-01-15T14:30:05",
      "audio_path": "audio/550e8400-e29b-41d4-a716-446655440000.wav",
      "timestamps_path": "audio/550e8400-e29b-41d4-a716-446655440000.timestamps.json",
      "duration_sec": 4.5,
      "was_regenerated": false,
      "error_message": null
    }
  ]
}
```

### timestamps.json формат

```json
{
  "words": [
    {"word": "Установите", "start": 0.0, "end": 0.45, "original_pos": [0, 10]},
    {"word": "докер", "start": 0.5, "end": 0.85, "original_pos": [11, 17]},
    {"word": "версии", "start": 0.9, "end": 1.2, "original_pos": [18, 24]}
  ]
}
```

**Поля:**
- `word` — слово из нормализованного текста (то, что произносится)
- `start`, `end` — временные метки в секундах
- `original_pos` — позиция в **исходном** тексте `[start_char, end_char]` для подсветки

**Сложность маппинга:**
При нормализации одно слово может превратиться в несколько:
- `"Docker"` → `"докер"` (1:1, позиция сохраняется)
- `">="` → `"больше или равно"` (1:3, все 3 слова ссылаются на позицию `>=`)
- `"20.10"` → `"двадцать точка десять"` (1:3)

Для реализации потребуется доработка `tts_pipeline`:
```python
# Новый метод в TTSPipeline
def process_with_positions(self, text: str) -> tuple[str, list[PositionMapping]]:
    """Нормализация с сохранением позиций.

    Returns:
        normalized_text: нормализованный текст
        mappings: список маппингов normalized_word → original_pos
    """
```

---

## Компоненты — детальное описание

### 1. Entry Point (ui/main.py)

```python
def main():
    """Entry point для UI приложения."""
    app = QApplication(sys.argv)
    app.setQuitOnLastWindowClosed(False)  # Не закрывать при скрытии окна

    # Системный стиль
    app.setStyle("Fusion")  # или оставить системный

    # Создание и запуск приложения
    tts_app = TTSApplication()
    tts_app.start()

    sys.exit(app.exec())
```

CLI:
```bash
fast-tts-ui              # Запуск GUI
fast-tts-ui --read-now   # Читать из буфера сразу (для внешних хоткеев)
fast-tts-ui --read-later # Читать отложенно
fast-tts-ui --show       # Показать окно
```

---

### 2. TTSApplication (ui/app.py)

**Ответственность:** Координация всех компонентов, tray, глобальные хоткеи.

```python
class TTSApplication(QObject):
    # Сигналы
    read_now_triggered = Signal()
    read_later_triggered = Signal()

    def __init__(self):
        self.config: UIConfig
        self.storage: StorageService
        self.tts_worker: TTSWorker
        self.hotkey_service: HotkeyService
        self.cleanup_worker: CleanupWorker

        self.main_window: MainWindow
        self.tray_icon: QSystemTrayIcon

    def start(self):
        """Инициализация и запуск."""
        self._load_config()
        self._init_services()
        self._init_tray()
        self._init_main_window()
        self._register_hotkeys()
        self._connect_signals()

    def read_now(self):
        """Читать текст из буфера сразу."""
        text = QApplication.clipboard().text()
        if text.strip():
            entry = self.storage.add_entry(text)
            self.tts_worker.process(entry, play_when_ready=True)

    def read_later(self):
        """Добавить текст в очередь."""
        text = QApplication.clipboard().text()
        if text.strip():
            entry = self.storage.add_entry(text)
            self.tts_worker.process(entry, play_when_ready=False)
```

**Tray меню:**
```
▶ Воспроизвести          (активно если есть что играть)
⏸ Пауза                  (активно если играет)
─────────────────────
📢 Читать сразу          Ctrl+T
📋 Читать отложенно      Ctrl+Shift+T
─────────────────────
⚙ Настройки...
📂 Открыть окно
─────────────────────
❌ Выход
```

---

### 3. MainWindow (ui/main_window.py)

**Layout:**

```
┌─────────────────────────────────────────────────────────────────────┐
│  Fast TTS RUS                                              [─][□][×]│
├─────────────────────────────────────────────────────────────────────┤
│  ┌─ Плеер ────────────────────────────────────────────────────────┐ │
│  │                                                                │ │
│  │  [⏮][◀◀]  [▶]  [▶▶][⏭]     02:35 ═══════●═══════════ 08:12  │ │
│  │                                                                │ │
│  │  Скорость: [0.5x] [0.75x] [1x] [1.25x] [1.5x] [2x]   🔊 ━━●━━  │ │
│  │                                                                │ │
│  └────────────────────────────────────────────────────────────────┘ │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌─ Очередь/История ──────────────────┐  ┌─ Текст ─────────────────┐│
│  │                                    │  │                         ││
│  │  ● Установите Docker версии...  ▶  │  │  ## Установка Docker    ││
│  │  ○ API доступен на http://...      │  │                         ││
│  │  ○ Вызови getUserData() через...   │  │  1. Скачайте Docker     ││
│  │  ○ Версия должна быть >= 20.10     │  │     Desktop с           ││
│  │                                    │  │     https://docker.com  ││
│  │  [Перегенерировать] [Удалить]      │  │                         ││
│  │                                    │  │  Текущее слово будет    ││
│  └────────────────────────────────────┘  │  ~~~подчёркнуто~~~      ││
│                                          │                         ││
│                                          └─────────────────────────┘│
├─────────────────────────────────────────────────────────────────────┤
│  Готово | Очередь: 3 | Ctrl+T: читать сразу                         │
└─────────────────────────────────────────────────────────────────────┘
```

**Структура:**
```python
class MainWindow(QMainWindow):
    def __init__(self, app: TTSApplication):
        self.app = app

        # Виджеты
        self.queue_list = QueueListWidget()
        self.text_viewer = TextViewerWidget()
        self.player = PlayerWidget()

        # Layout
        self._setup_ui()
        self._setup_shortcuts()  # Локальные хоткеи

    def closeEvent(self, event):
        """Скрыть в tray вместо закрытия."""
        event.ignore()
        self.hide()
```

---

### 4. QueueListWidget (widgets/queue_list.py)

**Функционал:**
- Отображение списка TextEntry
- Статус: pending (⏳), processing (🔄), ready (✓), error (❌)
- Индикатор текущего воспроизводимого
- Контекстное меню: перегенерировать, удалить, копировать текст
- Двойной клик — воспроизвести
- Сортировка: новые сверху

```python
class QueueListWidget(QListWidget):
    entry_selected = Signal(TextEntry)
    entry_play_requested = Signal(TextEntry)
    entry_regenerate_requested = Signal(TextEntry)
    entry_delete_requested = Signal(TextEntry)

    def update_entries(self, entries: list[TextEntry]): ...
    def set_current_playing(self, entry_id: str | None): ...
    def update_entry_status(self, entry_id: str, status: EntryStatus): ...
```

**Отображение элемента:**
```
┌─────────────────────────────────────────────────┐
│ ✓ Установите Docker версии >= 20.10...    ▶     │
│   4.5 сек • 15 янв 14:30                        │
└─────────────────────────────────────────────────┘
```

---

### 5. TextViewerWidget (widgets/text_viewer.py)

**Функционал:**
- Выбор формата отображения: Markdown (по умолчанию) или Plain Text
- Read-only режим
- Подсветка текущего слова при воспроизведении
- Автоскролл к текущей позиции

```python
from enum import Enum

class TextFormat(Enum):
    MARKDOWN = "markdown"
    PLAIN = "plain"

class TextViewerWidget(QTextBrowser):
    def __init__(self):
        self.current_entry: TextEntry | None = None
        self.timestamps: list[WordTimestamp] | None = None
        self.text_format: TextFormat = TextFormat.MARKDOWN
        self.timestamps_precise: bool = False  # True если от Silero, False если fallback

        self._highlight_format = QTextCharFormat()
        self._highlight_format.setUnderlineStyle(
            QTextCharFormat.UnderlineStyle.SingleUnderline
        )
        self._highlight_format.setBackground(QColor("#FFFF99"))

        self._context_highlight_format = QTextCharFormat()
        self._context_highlight_format.setBackground(QColor("#FFFDE7"))  # светло-жёлтый

    def set_format(self, fmt: TextFormat):
        """Переключить формат отображения."""
        self.text_format = fmt
        if self.current_entry:
            self._render_text()

    def set_entry(self, entry: TextEntry):
        """Установить текст для отображения."""
        self.current_entry = entry
        self._load_timestamps(entry)
        self._render_text()

    def _render_text(self):
        """Отрендерить текст в выбранном формате."""
        if self.text_format == TextFormat.MARKDOWN:
            html = markdown.markdown(self.current_entry.original_text)
            self.setHtml(html)
        else:
            self.setPlainText(self.current_entry.original_text)

    def highlight_at_position(self, position_sec: float):
        """Подсветить слово на указанной позиции аудио."""
        if not self.timestamps:
            return

        word_info = self._find_word_at(position_sec)
        if word_info:
            if self.timestamps_precise:
                # Точные timestamps от Silero — подсвечиваем только текущее слово
                self._highlight_range(
                    word_info.original_pos[0],
                    word_info.original_pos[1]
                )
            else:
                # Fallback (приблизительный расчёт) — подсвечиваем ±2 слова
                self._highlight_with_context(word_info, context_words=2)

            self._ensure_visible(word_info.original_pos[0])
```

**Подсветка (зависит от источника timestamps):**
- **Точные timestamps (от Silero):** только текущее слово — жёлтый фон + подчёркивание
- **Приблизительные timestamps (fallback):** текущее слово + ±2 слова контекста — светло-жёлтый фон для контекста, яркий для предполагаемого текущего

---

### 6. PlayerWidget (widgets/player.py)

**Функционал:**
- QMediaPlayer для воспроизведения
- Прогресс-бар с возможностью перемотки
- Кнопки управления
- Отображение времени
- Регулировка скорости
- Регулировка громкости

```python
class PlayerWidget(QWidget):
    position_changed = Signal(float)  # Для синхронизации с текстом

    def __init__(self):
        self.player = QMediaPlayer()
        self.audio_output = QAudioOutput()
        self.player.setAudioOutput(self.audio_output)

        # UI элементы
        self.btn_prev = QPushButton("⏮")
        self.btn_back_30 = QPushButton("◀◀")
        self.btn_play_pause = QPushButton("▶")
        self.btn_forward_30 = QPushButton("▶▶")
        self.btn_next = QPushButton("⏭")

        self.progress_slider = QSlider(Qt.Horizontal)
        self.time_label = QLabel("00:00 / 00:00")

        self.speed_buttons = SpeedButtonGroup()  # 0.5x, 0.75x, 1x, 1.25x, 1.5x, 2x
        self.volume_slider = QSlider(Qt.Horizontal)

        self._current_speed = 1.0

    def load_entry(self, entry: TextEntry):
        """Загрузить аудио из entry."""
        if entry.audio_path and entry.audio_path.exists():
            self.player.setSource(QUrl.fromLocalFile(str(entry.audio_path)))

    def play(self): ...
    def pause(self): ...
    def toggle_play_pause(self): ...
    def seek(self, position_sec: float): ...
    def seek_relative(self, delta_sec: float): ...
    def set_speed(self, speed: float): ...
    def seek_to_sentence_start(self): ...  # Для хоткея R
```

**Хоткеи (локальные, в окне):**

| Действие | Хоткей | Метод |
|----------|--------|-------|
| Play/Pause | Space | `toggle_play_pause()` |
| +5 сек | → или L | `seek_relative(5)` |
| -5 сек | ← или J | `seek_relative(-5)` |
| +30 сек | Shift+→ | `seek_relative(30)` |
| -30 сек | Shift+← | `seek_relative(-30)` |
| Ускорить | ] | `cycle_speed_up()` |
| Замедлить | [ | `cycle_speed_down()` |
| Следующий | N | `play_next()` |
| Предыдущий | P | `play_prev()` |
| Повтор фразы | R | `seek_to_sentence_start()` |

---

### 7. SettingsDialog (dialogs/settings.py)

```
┌─ Настройки ─────────────────────────────────────────────────────────┐
│                                                                     │
│  ┌─ Глобальные хоткеи ────────────────────────────────────────────┐ │
│  │                                                                │ │
│  │  Читать сразу:      [Ctrl+T          ] [Записать]              │ │
│  │  Читать отложенно:  [Ctrl+Shift+T    ] [Записать]              │ │
│  │                                                                │ │
│  │  ⓘ Хоткеи регистрируются через xdg-desktop-portal             │ │
│  └────────────────────────────────────────────────────────────────┘ │
│                                                                     │
│  ┌─ Голос ────────────────────────────────────────────────────────┐ │
│  │                                                                │ │
│  │  Спикер:   [xenia         ▼]   (aidar, baya, kseniya, xenia)   │ │
│  │  Скорость: [────────●─────]  1.0x                              │ │
│  │                                                                │ │
│  │  [▶ Тест голоса]                                               │ │
│  └────────────────────────────────────────────────────────────────┘ │
│                                                                     │
│  ┌─ Хранение ─────────────────────────────────────────────────────┐ │
│  │                                                                │ │
│  │  Папка кэша:  [~/.cache/fast_tts_rus      ] [📂 Открыть]       │ │
│  │                                                                │ │
│  │  История текстов:     [14    ] дней                            │ │
│  │  Макс. аудио файлов:  [5     ] файлов                          │ │
│  │  Перегенерированные:  [24    ] часов                           │ │
│  │                                                                │ │
│  │  Занято: 45.2 MB   [Очистить сейчас]                           │ │
│  └────────────────────────────────────────────────────────────────┘ │
│                                                                     │
│  ┌─ Уведомления ──────────────────────────────────────────────────┐ │
│  │                                                                │ │
│  │  ☑ Уведомлять при готовности (режим "Читать отложенно")        │ │
│  └────────────────────────────────────────────────────────────────┘ │
│                                                                     │
│                                        [Отмена] [Применить] [OK]    │
└─────────────────────────────────────────────────────────────────────┘
```

---

### 8. HotkeyService (services/hotkeys.py)

**xdg-desktop-portal GlobalShortcuts интеграция:**

```python
import dbus
from dbus.mainloop.glib import DBusGMainLoop

class HotkeyService(QObject):
    read_now_triggered = Signal()
    read_later_triggered = Signal()

    PORTAL_BUS = "org.freedesktop.portal.Desktop"
    PORTAL_PATH = "/org/freedesktop/portal/desktop"
    PORTAL_IFACE = "org.freedesktop.portal.GlobalShortcuts"

    def __init__(self, config: UIConfig):
        self.config = config
        self.session_handle = None

    def register(self):
        """Регистрация хоткеев через портал."""
        bus = dbus.SessionBus()
        portal = bus.get_object(self.PORTAL_BUS, self.PORTAL_PATH)

        # CreateSession
        shortcuts_iface = dbus.Interface(portal, self.PORTAL_IFACE)

        # Описание хоткеев
        shortcuts = [
            {
                "id": "read-now",
                "description": "Читать текст из буфера сразу",
                "preferred-trigger": self.config.hotkey_read_now,
            },
            {
                "id": "read-later",
                "description": "Добавить текст в очередь",
                "preferred-trigger": self.config.hotkey_read_later,
            },
        ]

        # BindShortcuts и подписка на сигнал Activated
        # ...

    def _on_shortcut_activated(self, session_handle, shortcut_id, timestamp, options):
        """Обработчик активации хоткея."""
        if shortcut_id == "read-now":
            self.read_now_triggered.emit()
        elif shortcut_id == "read-later":
            self.read_later_triggered.emit()
```

**Fallback:** Если портал недоступен — показать инструкцию пользователю настроить хоткеи в композиторе с вызовом CLI.

---

### 9. TTSWorker (services/tts_worker.py)

**Фоновая генерация TTS:**

```python
class TTSWorker(QObject):
    # Сигналы
    started = Signal(str)           # entry_id
    progress = Signal(str, float)   # entry_id, progress 0-1
    completed = Signal(str)         # entry_id
    error = Signal(str, str)        # entry_id, error_message

    def __init__(self, config: UIConfig, storage: StorageService):
        self.config = config
        self.storage = storage
        self.pipeline = TTSPipeline()  # Существующий пайплайн
        self.silero_model = None       # Lazy load
        self.thread_pool = QThreadPool()
        self.play_queue: list[str] = []  # entry_ids to play after ready

    def process(self, entry: TextEntry, play_when_ready: bool = False):
        """Запустить TTS для entry в фоне."""
        if play_when_ready:
            self.play_queue.append(entry.id)

        runnable = TTSRunnable(
            entry=entry,
            pipeline=self.pipeline,
            config=self.config,
            storage=self.storage,
        )
        runnable.signals.completed.connect(self._on_completed)
        runnable.signals.error.connect(self._on_error)

        self.thread_pool.start(runnable)

    def _on_completed(self, entry_id: str):
        self.completed.emit(entry_id)

        # Автовоспроизведение если было запрошено
        if entry_id in self.play_queue:
            self.play_queue.remove(entry_id)
            # Emit signal to play
```

**TTSRunnable:**
```python
class TTSRunnable(QRunnable):
    def run(self):
        try:
            # 1. Нормализация текста
            normalized = self.pipeline.process(self.entry.original_text)

            # 2. Синтез с timestamps
            audio, timestamps = self._synthesize_with_timestamps(normalized)

            # 3. Сохранение
            audio_path = self.storage.save_audio(self.entry.id, audio)
            timestamps_path = self.storage.save_timestamps(self.entry.id, timestamps)

            # 4. Обновление entry
            self.entry.normalized_text = normalized
            self.entry.audio_path = audio_path
            self.entry.timestamps_path = timestamps_path
            self.entry.status = EntryStatus.READY
            self.entry.audio_generated_at = datetime.now()

            self.storage.update_entry(self.entry)
            self.signals.completed.emit(self.entry.id)

        except Exception as e:
            self.entry.status = EntryStatus.ERROR
            self.entry.error_message = str(e)
            self.storage.update_entry(self.entry)
            self.signals.error.emit(self.entry.id, str(e))
```

---

### 10. StorageService (services/storage.py)

```python
class StorageService:
    def __init__(self, config: UIConfig):
        self.config = config
        self.cache_dir = config.cache_dir
        self.audio_dir = self.cache_dir / "audio"
        self.history_file = self.cache_dir / "history.json"

        self._ensure_dirs()
        self._entries: dict[str, TextEntry] = {}
        self._load_history()

    # CRUD операции
    def add_entry(self, text: str) -> TextEntry: ...
    def get_entry(self, entry_id: str) -> TextEntry | None: ...
    def update_entry(self, entry: TextEntry): ...
    def delete_entry(self, entry_id: str): ...
    def get_all_entries(self) -> list[TextEntry]: ...

    # Аудио
    def save_audio(self, entry_id: str, audio_data: np.ndarray) -> Path: ...
    def save_timestamps(self, entry_id: str, timestamps: list) -> Path: ...
    def load_timestamps(self, entry_id: str) -> list | None: ...

    # Персистентность
    def _load_history(self): ...
    def _save_history(self): ...
```

---

### 11. CleanupWorker (services/cleanup.py)

```python
class CleanupWorker(QObject):
    cleanup_completed = Signal(int)  # Количество удалённых

    def __init__(self, config: UIConfig, storage: StorageService):
        self.config = config
        self.storage = storage

    def run_cleanup(self):
        """Запустить очистку в фоновом потоке."""
        QThreadPool.globalInstance().start(
            CleanupRunnable(self.config, self.storage, self.cleanup_completed)
        )

class CleanupRunnable(QRunnable):
    def run(self):
        deleted_count = 0
        now = datetime.now()

        entries = self.storage.get_all_entries()

        for entry in entries:
            should_delete_text = False
            should_delete_audio = False

            # Правило 1: тексты старше N дней
            age_days = (now - entry.created_at).days
            if age_days > self.config.history_days:
                should_delete_text = True

            # Правило 2: аудио - оставить только последние N файлов
            # (обрабатывается отдельно по сортировке)

            # Правило 3: перегенерированные аудио - хранить N часов
            if entry.was_regenerated and entry.audio_generated_at:
                age_hours = (now - entry.audio_generated_at).total_seconds() / 3600
                if age_hours > self.config.audio_regenerated_hours:
                    should_delete_audio = True

            if should_delete_text:
                self.storage.delete_entry(entry.id)
                deleted_count += 1
            elif should_delete_audio:
                self.storage.delete_audio(entry.id)

        # Правило 2: оставить только N последних аудио файлов
        deleted_count += self._cleanup_old_audio_files()

        self.signals.cleanup_completed.emit(deleted_count)

    def _cleanup_old_audio_files(self) -> int:
        """Удалить аудио, оставив только последние N."""
        entries_with_audio = [
            e for e in self.storage.get_all_entries()
            if e.audio_path and e.audio_path.exists() and not e.was_regenerated
        ]

        # Сортировка по времени генерации (новые первые)
        entries_with_audio.sort(
            key=lambda e: e.audio_generated_at or e.created_at,
            reverse=True
        )

        deleted = 0
        for entry in entries_with_audio[self.config.audio_max_files:]:
            self.storage.delete_audio(entry.id)
            deleted += 1

        return deleted
```

---

## Порядок реализации

### Фаза 1: Каркас (1-2 дня)

1. **Структура проекта** — создать директории и файлы
2. **models/** — Entry, Config dataclasses
3. **services/storage.py** — базовое хранение
4. **ui/main.py** — entry point
5. **ui/app.py** — skeleton с tray

**Результат:** Приложение запускается, показывает tray, можно открыть пустое окно.

### Фаза 2: Очередь и история (1-2 дня)

1. **widgets/queue_list.py** — отображение списка
2. **Интеграция storage ↔ queue_list**
3. **Добавление из буфера** (через tray меню, без хоткеев)
4. **services/cleanup.py** — фоновая очистка

**Результат:** Можно добавлять тексты, они сохраняются и отображаются.

### Фаза 3: TTS интеграция (3-4 дня)

1. **services/tts_worker.py** — фоновая генерация
2. **Интеграция с существующим пайплайном**
3. **Исследование Silero Timestamps API:**
   - Проверить `model.apply_tts()` на наличие параметра для word-level timestamps
   - Изучить возвращаемые данные (есть ли `word_timestamps` или аналог)
   - Проверить альтернативы: `model.synthesize()`, SSML-разметка
4. **Реализация получения timestamps:**
   - **Если Silero поддерживает:** извлечь timestamps из результата синтеза
   - **Если не поддерживает:** реализовать fallback — расчёт по длительности слов:
     ```python
     # Приблизительный расчёт: длина_слова / общая_длина * duration
     def estimate_timestamps(words: list[str], total_duration: float) -> list[WordTimestamp]:
         total_chars = sum(len(w) for w in words)
         current_time = 0.0
         timestamps = []
         for word in words:
             word_duration = (len(word) / total_chars) * total_duration
             timestamps.append(WordTimestamp(word, current_time, current_time + word_duration))
             current_time += word_duration
         return timestamps
     ```
5. **Маппинг normalized → original позиции:**
   - Сохранять соответствие позиций при нормализации в `tts_pipeline`
   - Добавить метод `pipeline.process_with_mapping()` или аналог
6. **Обновление статусов в UI**

**Результат:** Тексты нормализуются и синтезируются, аудио + timestamps сохраняются.

### Фаза 4: Плеер (2-3 дня)

1. **widgets/player.py** — QMediaPlayer обёртка
2. **Прогресс-бар** с seek
3. **Контролы скорости и громкости**
4. **Локальные хоткеи**
5. **Навигация по очереди** (next/prev)

**Результат:** Полнофункциональный плеер.

### Фаза 5: Текстовый просмотрщик (1-2 дня)

1. **widgets/text_viewer.py** — Markdown рендеринг
2. **Синхронизация с плеером** — подсветка текущего слова
3. **Автоскролл**

**Результат:** При воспроизведении подсвечивается текущее слово.

### Фаза 6: Глобальные хоткеи (1-2 дня)

1. **services/hotkeys.py** — xdg-desktop-portal интеграция
2. **Fallback** — инструкция для CLI
3. **Тестирование** на разных DE (GNOME, KDE, Sway)

**Результат:** Ctrl+T добавляет текст и читает сразу.

### Фаза 7: Настройки (1 день)

1. **dialogs/settings.py** — UI настроек
2. **Сохранение/загрузка конфига**
3. **Применение настроек** без перезапуска

**Результат:** Полноценный диалог настроек.

### Фаза 8: Полировка (1-2 дня)

1. **Уведомления** (QSystemTrayIcon.showMessage)
2. **Иконки** — подобрать или нарисовать
3. **Обработка ошибок** — user-friendly сообщения
4. **Тестирование** end-to-end

**Результат:** MVP готов к использованию.

---

## Риски и митигация

| Риск | Вероятность | Митигация |
|------|-------------|-----------|
| xdg-portal не работает в DE | Средняя | CLI fallback + инструкция |
| Silero не даёт timestamps | Средняя | Fallback: расчёт по длине слов (см. Фазу 3) |
| Qt Multimedia проблемы на Wayland | Низкая | Использовать PipeWire backend |
| Высокое потребление памяти | Средняя | Ленивая загрузка модели, очистка |

---

## Тестирование

### Unit тесты
- models/entry.py — сериализация/десериализация
- services/storage.py — CRUD операции
- services/cleanup.py — логика очистки

### Integration тесты
- TTS worker + storage
- Player + timestamps sync

### Manual тесты
- Tray на GNOME, KDE, Sway
- Хоткеи на разных DE
- Длинные тексты (статьи)

---

## Метрики успеха MVP

1. **Функциональность:**
   - [ ] Добавление текста из буфера работает
   - [ ] TTS генерируется корректно
   - [ ] Воспроизведение с контролами работает
   - [ ] Синхронизация текст-аудио работает
   - [ ] Очистка истории работает
   - [ ] Настройки сохраняются

2. **Производительность:**
   - [ ] UI не блокируется при TTS
   - [ ] Память < 500MB при работе
   - [ ] Запуск < 3 секунд

3. **Usability:**
   - [ ] Интуитивно понятный интерфейс
   - [ ] Хоткеи работают (или есть fallback)
   - [ ] Ошибки показываются понятно
