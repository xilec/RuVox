# Архитектура UI

## Обзор

Приложение построено на PyQt6 и состоит из:
- **Главное окно** — основной интерфейс
- **Системный трей** — фоновая работа
- **Сервисы** — TTS, хранение, хоткеи

## Диаграмма компонентов

```
┌─────────────────────────────────────────────────────────────────┐
│                        Application                              │
│  ┌─────────────┐  ┌──────────────┐  ┌───────────────────────┐   │
│  │ MainWindow  │  │ SystemTray   │  │      Services         │   │
│  │             │  │              │  │  ┌─────────────────┐  │   │
│  │ ┌─────────┐ │  │ - Show/Hide  │  │  │   TTSWorker     │  │   │
│  │ │ Player  │ │  │ - Menu       │  │  │   (QThreadPool) │  │   │
│  │ └─────────┘ │  │              │  │  └─────────────────┘  │   │
│  │ ┌─────────┐ │  └──────────────┘  │  ┌─────────────────┐  │   │
│  │ │QueueList│ │                    │  │    Storage      │  │   │
│  │ └─────────┘ │                    │  │  (JSON + WAV)   │  │   │
│  │ ┌─────────┐ │                    │  └─────────────────┘  │   │
│  │ │TextViewer││                    │  ┌─────────────────┐  │   │
│  │ └─────────┘ │                    │  │  HotkeyService  │  │   │
│  └─────────────┘                    │  │  (D-Bus portal) │  │   │
│                                     │  └─────────────────┘  │   │
└─────────────────────────────────────────────────────────────────┘
```

## Виджеты

### PlayerWidget (`widgets/player.py`)

Плеер для воспроизведения аудио.

```python
class PlayerWidget(QWidget):
    # Сигналы
    playback_started = pyqtSignal(str)    # entry_id
    playback_stopped = pyqtSignal()
    position_changed = pyqtSignal(float)  # seconds
    next_requested = pyqtSignal()
    prev_requested = pyqtSignal()

    # Методы
    def load_entry(self, entry: TextEntry, audio_dir: Path) -> bool
    def play(self) -> None
    def pause(self) -> None
    def stop(self) -> None
    def toggle_play_pause(self) -> None
    def seek_relative(self, seconds: float) -> None
    def speed_up(self) -> None
    def speed_down(self) -> None
```

**Использует:** libmpv (python-mpv) с алгоритмом scaletempo2 для качественного звука при ускоренном воспроизведении audio

### QueueListWidget (`widgets/queue_list.py`)

Список записей в очереди.

```python
class QueueListWidget(QListWidget):
    # Сигналы
    entry_selected = pyqtSignal(TextEntry)
    entry_play_requested = pyqtSignal(TextEntry)
    entry_regenerate_requested = pyqtSignal(TextEntry)
    entry_delete_requested = pyqtSignal(TextEntry)

    # Методы
    def add_entry(self, entry: TextEntry) -> None
    def remove_entry(self, entry_id: str) -> None
    def update_entry(self, entry: TextEntry) -> None
    def update_entries(self, entries: list[TextEntry]) -> None
    def set_current_playing(self, entry_id: str | None) -> None
```

**Отображает:**
- Статус записи (иконка: ожидание, обработка, готово, ошибка)
- Превью текста
- Время создания
- Индикатор воспроизведения

### TextViewerWidget (`widgets/text_viewer.py`)

Просмотр текста с подсветкой слов.

```python
class TextViewerWidget(QTextBrowser):
    # Методы
    def set_entry(self, entry: TextEntry, timestamps: list | None) -> None
    def clear_entry(self) -> None
    def set_format(self, fmt: TextFormat) -> None  # MARKDOWN / PLAIN
    def highlight_at_position(self, position_sec: float) -> None
```

**Особенности:**
- Рендеринг Markdown (заголовки, списки, код)
- Подсветка текущего слова жёлтым цветом
- Автоскролл к текущей позиции
- Подсветка только для воспроизводимой записи

## Сервисы

### TTSWorker (`services/tts_worker.py`)

Фоновый синтез речи.

```python
class TTSWorker(QObject):
    # Сигналы
    started = pyqtSignal(str)           # entry_id
    progress = pyqtSignal(str, float)   # entry_id, 0-1
    completed = pyqtSignal(str)         # entry_id
    error = pyqtSignal(str, str)        # entry_id, message
    model_loading = pyqtSignal()
    model_loaded = pyqtSignal()
    play_requested = pyqtSignal(str)    # entry_id

    # Методы
    def process(self, entry: TextEntry, play_when_ready: bool = False) -> None
    def ensure_model_loaded(self) -> bool
```

**Процесс обработки:**
1. Нормализация текста через TTSPipeline
2. Синтез аудио через Silero
3. Генерация временных меток слов
4. Сохранение в Storage

### Storage (`services/storage.py`)

Хранение записей и аудио.

```python
class StorageService:
    # Пути
    cache_dir: Path      # ~/.cache/fast-tts-rus/
    audio_dir: Path      # cache_dir/audio/
    history_file: Path   # cache_dir/history.json

    # Методы
    def add_entry(self, entry: TextEntry) -> None
    def get_entry(self, entry_id: str) -> TextEntry | None
    def get_all_entries(self) -> list[TextEntry]
    def update_entry(self, entry: TextEntry) -> None
    def delete_entry(self, entry_id: str) -> None

    def save_audio(self, entry_id: str, audio: np.ndarray, sample_rate: int) -> Path
    def save_timestamps(self, entry_id: str, timestamps: list) -> Path
    def load_timestamps(self, entry_id: str) -> list | None
    def delete_audio(self, entry_id: str) -> None
```

### HotkeyService (`services/hotkeys.py`)

Глобальные горячие клавиши через xdg-desktop-portal.

```python
class HotkeyService(QObject):
    # Сигналы
    read_now_activated = pyqtSignal()
    read_later_activated = pyqtSignal()

    # Методы
    def register_shortcuts(self) -> bool
    def unregister_shortcuts(self) -> None
```

**Реализация (Wayland/KDE):**
- Использует D-Bus portal `org.freedesktop.portal.GlobalShortcuts`
- Библиотека dasbus для сериализации сложных типов
- GLib для обработки сигналов

### ClipboardService (`services/clipboard.py`)

Работа с буфером обмена.

```python
def get_clipboard_text() -> str | None
```

**Особенности:**
- Основной метод: `QApplication.clipboard().text()`
- Fallback для Wayland: `wl-paste` через subprocess

## Модели данных

### TextEntry (`models/entry.py`)

```python
@dataclass
class TextEntry:
    id: str                              # UUID
    original_text: str                   # Исходный текст
    normalized_text: str | None          # Нормализованный
    status: EntryStatus                  # PENDING, PROCESSING, READY, ERROR
    created_at: datetime
    audio_path: Path | None
    timestamps_path: Path | None
    duration_sec: float | None
    played_at: datetime | None
    error_message: str | None
    was_regenerated: bool

class EntryStatus(Enum):
    PENDING = "pending"
    PROCESSING = "processing"
    READY = "ready"
    ERROR = "error"
```

### UIConfig (`models/config.py`)

```python
@dataclass
class UIConfig:
    speaker: str = "xenia"
    sample_rate: int = 48000
    hotkey_read_now: str = "Control+grave"
    hotkey_read_later: str = "Control+Shift+grave"
    cleanup_played_after_days: int = 7
    cleanup_unplayed_after_days: int = 30
    window_geometry: tuple | None = None

    @classmethod
    def load(cls) -> "UIConfig"
    def save(self) -> None
```

## Поток данных

```
                    ┌──────────────┐
                    │  Clipboard   │
                    └──────┬───────┘
                           │ get_clipboard_text()
                           ▼
┌──────────────┐    ┌──────────────┐
│   Hotkey     │───▶│     App      │
│   Service    │    │              │
└──────────────┘    └──────┬───────┘
                           │ process()
                           ▼
                    ┌──────────────┐
                    │  TTSWorker   │
                    │              │
                    │ 1. Normalize │──▶ TTSPipeline
                    │ 2. Synthesize│──▶ Silero
                    │ 3. Timestamps│──▶ CharMapping
                    └──────┬───────┘
                           │ save()
                           ▼
                    ┌──────────────┐
                    │   Storage    │
                    │              │
                    │ - history.json
                    │ - audio/*.wav
                    │ - *.timestamps.json
                    └──────┬───────┘
                           │ load()
                           ▼
                    ┌──────────────┐
                    │  MainWindow  │
                    │              │
                    │ - QueueList  │
                    │ - TextViewer │
                    │ - Player     │
                    └──────────────┘
```

## Многопоточность

- **Main thread** — UI, события
- **QThreadPool** — TTS синтез (TTSRunnable)
- **Model loading** — отдельный runnable

Синхронизация через Qt signals/slots (thread-safe).
