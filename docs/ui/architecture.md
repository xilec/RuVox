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

Просмотр текста с подсветкой слов и поддержкой Markdown.

```python
class TextFormat(Enum):
    MARKDOWN = "markdown"
    PLAIN = "plain"

class TextViewerWidget(QTextBrowser):
    current_entry: TextEntry | None
    timestamps: list[dict] | None
    text_format: TextFormat
    _markdown_mapper: MarkdownPositionMapper | None

    # Методы
    def set_entry(self, entry: TextEntry, timestamps: list | None) -> None
    def clear_entry(self) -> None
    def set_format(self, fmt: TextFormat) -> None
    def highlight_at_position(self, position_sec: float) -> None
```

**Режимы отображения:**
1. **PLAIN** — plain text с видимыми Markdown маркерами
   - `setPlainText(entry.original_text)`
   - Подсветка напрямую по `original_pos` из timestamps

2. **MARKDOWN** — рендеренный HTML
   - `setHtml(markdown_mapper.build_mapping())`
   - `MarkdownPositionMapper` переводит `original_pos` → `rendered_pos`
   - Подсветка через QTextCursor в отрендеренном документе

**Алгоритм подсветки:**
1. Получить `position_sec` от плеера (каждые 200ms)
2. Найти слово в timestamps с `start <= position_sec < end`
3. Получить `original_pos` из timestamp (позиция в `entry.original_text`)
4. Если Markdown mode:
   - `mapper.get_rendered_range(original_start, original_end)` → `(doc_start, doc_end)`
   - Применить подсветку в позициях `[doc_start:doc_end]` в HTML документе
5. Если Plain mode:
   - Применить подсветку напрямую в `[original_start:original_end]`

**Mermaid-диаграммы (Markdown режим):**
- Блоки ` ```mermaid ``` ` извлекаются перед Markdown-конвертацией
- Рендерятся как изображения через `MermaidRenderer` (hidden QWebEngineView + mermaid.js)
- Клик по изображению открывает `MermaidPreviewDialog` с интерактивным просмотром
- `loadResource()` override предоставляет pixmap по запросу Qt при парсинге HTML
- В TTS pipeline mermaid-блоки заменяются на "Тут мермэйд диаграмма"

**Особенности:**
- Переключение режимов во время воспроизведения — подсветка восстанавливается автоматически
- Автоскролл к текущей позиции
- Кликабельные ссылки в Markdown режиме (`setOpenLinks(False)` + `anchorClicked` для custom routing)
- Graceful degradation при отсутствии mapping

## Утилиты

### MarkdownPositionMapper (`utils/markdown_mapper.py`)

Маппинг позиций между оригинальным Markdown и отрендеренным plain text.

```python
class MarkdownPositionMapper:
    original_text: str               # Оригинальный Markdown
    rendered_plain: str              # Plain text из HTML (через ElementTree)
    position_map: dict[int, int]     # original_pos → rendered_pos

    def build_mapping(self, md_instance=None) -> str:
        """Рендерит HTML и строит position_map."""

    def get_rendered_range(
        self, original_start: int, original_end: int
    ) -> tuple[int, int] | None:
        """Переводит диапазон original → rendered."""
```

**Алгоритм построения маппинга:**

1. **Рендеринг Markdown → HTML**
   ```python
   html = md.convert(original_text)
   ```

2. **Извлечение видимого текста из HTML** (через ElementTree)
   - Парсим HTML как XML
   - Рекурсивно собираем текстовые узлы
   - Игнорируем теги, атрибуты, комментарии
   - Получаем `rendered_plain`

3. **Построение character-level mapping**
   - Для каждого слова в `rendered_plain`:
     - Найти его в `original_text`
     - Записать `position_map[original_pos] = rendered_pos`
   - Используется отслеживание уже использованных диапазонов для корректной работы с повторяющимися словами

4. **Fallback word-level mapping** (для code blocks)
   - Если покрытие < 50%, добавляем word-level маппинг
   - Находим слова через регулярные выражения
   - Сопоставляем по порядку появления

**Пример:**

```markdown
Original:    "Some **bold** text"
             [0   4 56   1012  18]

Rendered:    "Some bold text"
             [0   4 5  9 10 14]

position_map:
  0 → 0   (S)
  1 → 1   (o)
  ...
  7 → 5   (**bold** → bold, начало слова)
  11 → 9  (конец слова "bold")
```

**get_rendered_range(7, 11)** → **(5, 9)**

**Поддерживаемые Markdown элементы:**
- Bold, italic: `**text**`, `_text_`
- Headers: `# H1`, `## H2`
- Inline code: \`code\`
- Code blocks: \`\`\`python
- Links: `[text](url)` — извлекается только `text`
- Lists: `- item`, `1. item`
- Tables

**Тесты:** `tests/ui/test_markdown_mapper.py` (18 тестов)

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
    cache_dir: Path      # ~/.cache/ruvox/
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

### MermaidRenderer (`services/mermaid_renderer.py`)

Рендеринг Mermaid-диаграмм в SVG и pixmap.

```python
class MermaidRenderer(QObject):
    # Сигналы
    svg_ready = pyqtSignal(str, str)  # (code_hash, svg_string)

    # Методы
    def render(self, code: str) -> None       # Очередь на рендеринг
    def get_cached_svg(self, code: str) -> str | None
    def get_cached_pixmap(self, code: str, width: int) -> QPixmap | None
    def mermaid_js_path(self) -> Path | None
    def cleanup(self) -> None
```

**Архитектура:**
1. Скачивает `mermaid.min.js` из CDN при первом использовании
2. Рендерит `<pre class="mermaid">` в hidden QWebEngineView через `mermaid.run()`
3. Захватывает отрендеренную страницу как QPixmap через `QWidget.grab()`
4. Кэширует SVG и pixmap по hash кода диаграммы
5. Fallback: `QSvgRenderer` для простых SVG (без foreignObject/CSS)

**Очередь рендеринга:** обрабатывает блоки по одному, асинхронно через polling JavaScript-результата.

### MermaidPreviewDialog (`dialogs/mermaid_preview.py`)

Интерактивный просмотр Mermaid-диаграмм.

```python
class MermaidPreviewDialog(QDialog):
    def show_diagram(self, code: str, title: str = "") -> None
```

**Возможности:**
- Полноэкранный QWebEngineView с живым рендерингом mermaid.js
- Переключение светлой/тёмной темы
- Zoom ±25% через toolbar-кнопки
- Закрытие: кнопка "Закрыть" или Escape

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
