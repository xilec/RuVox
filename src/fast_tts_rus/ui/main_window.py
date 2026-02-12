"""Main application window."""

import logging

from PyQt6.QtCore import Qt
from PyQt6.QtWidgets import (
    QMainWindow,
    QWidget,
    QVBoxLayout,
    QHBoxLayout,
    QSplitter,
    QLabel,
    QComboBox,
    QStatusBar,
)

from fast_tts_rus.ui.services.logging_service import safe_slot
from fast_tts_rus.ui.widgets.queue_list import QueueListWidget
from fast_tts_rus.ui.widgets.player import PlayerWidget
from fast_tts_rus.ui.widgets.text_viewer import TextViewerWidget, TextFormat
from fast_tts_rus.ui.models.entry import TextEntry, EntryStatus

logger = logging.getLogger(__name__)


class MainWindow(QMainWindow):
    """Main application window.

    Layout:
    - Top: Player widget
    - Middle: Queue list (left) + Text viewer (right)
    - Bottom: Status bar
    """

    def __init__(self, app):
        super().__init__()

        self.app = app

        self.setWindowTitle("Fast TTS RUS")
        self.setMinimumSize(600, 400)
        self.resize(900, 600)

        # Restore geometry if saved
        self._restore_geometry()

        self._setup_ui()
        self._setup_shortcuts()
        self._connect_tts_signals()

    def _setup_ui(self) -> None:
        """Setup the user interface."""
        central_widget = QWidget()
        self.setCentralWidget(central_widget)

        main_layout = QVBoxLayout(central_widget)
        main_layout.setContentsMargins(8, 8, 8, 8)
        main_layout.setSpacing(8)

        # Player widget (top)
        self.player = PlayerWidget()
        self.player.setMinimumHeight(80)
        self._connect_player_signals()
        main_layout.addWidget(self.player)

        # Splitter for queue and text viewer
        splitter = QSplitter(Qt.Orientation.Horizontal)

        # Queue list (left)
        self.queue_list = QueueListWidget()
        self.queue_list.setMinimumWidth(200)
        splitter.addWidget(self.queue_list)

        # Connect queue signals
        self._connect_queue_signals()

        # Text viewer panel (right) with format selector
        text_panel = QWidget()
        text_panel_layout = QVBoxLayout(text_panel)
        text_panel_layout.setContentsMargins(0, 0, 0, 0)
        text_panel_layout.setSpacing(4)

        # Text viewer
        self.text_viewer = TextViewerWidget()
        text_panel_layout.addWidget(self.text_viewer, 1)  # stretch factor 1

        # Format selector (bottom right)
        format_row = QHBoxLayout()
        format_row.setContentsMargins(0, 0, 0, 0)
        format_row.addStretch()

        format_label = QLabel("Формат:")
        format_row.addWidget(format_label)

        self.format_selector = QComboBox()
        self.format_selector.addItem("📝 Plain Text", TextFormat.PLAIN.value)
        self.format_selector.addItem("📄 Markdown", TextFormat.MARKDOWN.value)
        self.format_selector.setMinimumWidth(150)
        self.format_selector.currentIndexChanged.connect(self._on_format_changed)
        format_row.addWidget(self.format_selector)

        text_panel_layout.addLayout(format_row)

        splitter.addWidget(text_panel)

        # Set initial splitter sizes (1:2 ratio)
        splitter.setSizes([300, 600])

        main_layout.addWidget(splitter, 1)  # stretch factor 1

        # Status bar
        self.status_bar = QStatusBar()
        self.setStatusBar(self.status_bar)
        self._update_status_bar()

    def _connect_player_signals(self) -> None:
        """Connect player widget signals."""
        self.player.next_requested.connect(self._play_next)
        self.player.prev_requested.connect(self._play_prev)
        self.player.playback_started.connect(self._on_playback_started)
        self.player.playback_stopped.connect(self._on_playback_stopped)
        self.player.position_changed.connect(self._on_playback_position_changed)

    def _connect_queue_signals(self) -> None:
        """Connect queue list signals."""
        self.queue_list.entry_selected.connect(self._on_entry_selected)
        self.queue_list.entry_play_requested.connect(self._on_entry_play_requested)
        self.queue_list.entry_regenerate_requested.connect(self._on_entry_regenerate_requested)
        self.queue_list.entry_delete_requested.connect(self._on_entry_delete_requested)

    @safe_slot
    def _on_entry_selected(self, entry: TextEntry) -> None:
        """Handle entry selection - show text in viewer."""
        # Load timestamps if available
        timestamps = None
        if self.app.storage and entry.timestamps_path:
            timestamps = self.app.storage.load_timestamps(entry.id)
        self.text_viewer.set_entry(entry, timestamps)

    @safe_slot
    def _on_entry_play_requested(self, entry: TextEntry) -> None:
        """Handle play request."""
        if entry.status != EntryStatus.READY:
            logger.debug(f"Пропуск воспроизведения: статус {entry.status}")
            return

        if self.app.storage:
            audio_dir = self.app.storage.audio_dir
            if self.player.load_entry(entry, audio_dir):
                logger.info(f"Воспроизведение: {entry.id[:8]}...")
                # Load text and timestamps into viewer
                timestamps = self.app.storage.load_timestamps(entry.id)
                self.text_viewer.set_entry(entry, timestamps)
                # Start playback
                self.player.play()
                self.queue_list.set_current_playing(entry.id)

    @safe_slot
    def _on_entry_regenerate_requested(self, entry: TextEntry) -> None:
        """Handle regenerate request."""
        if not self.app.tts_worker or not self.app.storage:
            return

        logger.info(f"Перегенерация: {entry.id[:8]}...")

        # Stop playback if this entry is currently playing
        if self.player.current_entry and self.player.current_entry.id == entry.id:
            self.player.stop()

        # Delete existing audio file (this also resets entry status in storage)
        self.app.storage.delete_audio(entry.id)

        # Get the updated entry from storage
        updated_entry = self.app.storage.get_entry(entry.id)
        if not updated_entry:
            logger.error(f"Запись не найдена после delete_audio: {entry.id}")
            return

        # Mark as regenerated for cleanup rules
        updated_entry.was_regenerated = True
        self.app.storage.update_entry(updated_entry)
        self.queue_list.update_entry(updated_entry)

        # Start TTS processing
        self.app.tts_worker.process(updated_entry, play_when_ready=False)

    @safe_slot
    def _on_entry_delete_requested(self, entry: TextEntry) -> None:
        """Handle delete request."""
        if self.app.storage:
            logger.info(f"Удаление: {entry.id[:8]}...")
            # Stop playback if deleting current entry
            if self.player.current_entry and self.player.current_entry.id == entry.id:
                self.player.stop()
            self.app.storage.delete_entry(entry.id)
            self.queue_list.remove_entry(entry.id)
            self._update_status_bar()

    @safe_slot
    def _on_playback_started(self, entry_id: str) -> None:
        """Handle playback started."""
        self.queue_list.set_current_playing(entry_id)

    @safe_slot
    def _on_playback_stopped(self) -> None:
        """Handle playback stopped."""
        self.queue_list.set_current_playing(None)

    @safe_slot
    def _on_playback_position_changed(self, position_sec: float) -> None:
        """Handle playback position change - update text highlighting."""
        # Only highlight if the viewer is showing the currently playing entry
        if (self.player.current_entry and
            self.text_viewer.current_entry and
            self.player.current_entry.id == self.text_viewer.current_entry.id):
            self.text_viewer.highlight_at_position(position_sec)

    @safe_slot
    def _play_next(self) -> None:
        """Play next entry in queue."""
        entries = self._get_ready_entries()
        if not entries:
            return

        current_id = self.player.current_entry.id if self.player.current_entry else None

        # Find current index
        current_idx = -1
        for i, entry in enumerate(entries):
            if entry.id == current_id:
                current_idx = i
                break

        # Play next
        next_idx = current_idx + 1
        if next_idx < len(entries):
            self._on_entry_play_requested(entries[next_idx])

    @safe_slot
    def _play_prev(self) -> None:
        """Play previous entry in queue."""
        entries = self._get_ready_entries()
        if not entries:
            return

        current_id = self.player.current_entry.id if self.player.current_entry else None

        # Find current index
        current_idx = len(entries)
        for i, entry in enumerate(entries):
            if entry.id == current_id:
                current_idx = i
                break

        # Play previous
        prev_idx = current_idx - 1
        if prev_idx >= 0:
            self._on_entry_play_requested(entries[prev_idx])

    def _get_ready_entries(self) -> list[TextEntry]:
        """Get list of entries that are ready to play."""
        if not self.app.storage:
            return []
        return [
            e for e in self.app.storage.get_all_entries()
            if e.status == EntryStatus.READY
        ]

    def play_entry(self, entry_id: str) -> None:
        """Play entry by ID (called from app on play_requested)."""
        if not self.app.storage:
            return
        entry = self.app.storage.get_entry(entry_id)
        if entry:
            self._on_entry_play_requested(entry)

    def _update_status_bar(self) -> None:
        """Update status bar with current state."""
        queue_count = 0
        if self.app.storage:
            entries = self.app.storage.get_all_entries()
            queue_count = len(entries)
        self.status_bar.showMessage(f"Готово | Очередь: {queue_count}")

    def load_entries(self) -> None:
        """Load entries from storage into queue list."""
        if self.app.storage:
            entries = self.app.storage.get_all_entries()
            self.queue_list.update_entries(entries)
            self._update_status_bar()

        # Restore saved text format
        self._restore_text_format()

    def add_entry(self, entry) -> None:
        """Add a new entry to the queue list."""
        self.queue_list.add_entry(entry)
        self._update_status_bar()

    def _connect_tts_signals(self) -> None:
        """Connect TTS worker signals for status updates."""
        if self.app.tts_worker:
            self.app.tts_worker.started.connect(self._on_tts_started)
            self.app.tts_worker.progress.connect(self._on_tts_progress)

    @safe_slot
    def _on_tts_started(self, entry_id: str) -> None:
        """Handle TTS processing started."""
        self.queue_list.update_entry_status(entry_id, EntryStatus.PROCESSING)
        self._update_status_bar()

    @safe_slot
    def _on_tts_progress(self, entry_id: str, progress: float) -> None:
        """Handle TTS progress update."""
        # Could update a progress indicator in the queue item
        pass

    def _setup_shortcuts(self) -> None:
        """Setup keyboard shortcuts for player."""
        from PyQt6.QtGui import QShortcut, QKeySequence

        # Play/Pause
        QShortcut(QKeySequence(Qt.Key.Key_Space), self, self.player.toggle_play_pause)

        # Seek
        QShortcut(QKeySequence(Qt.Key.Key_Left), self, lambda: self.player.seek_relative(-5))
        QShortcut(QKeySequence(Qt.Key.Key_Right), self, lambda: self.player.seek_relative(5))
        QShortcut(QKeySequence("Shift+Left"), self, lambda: self.player.seek_relative(-30))
        QShortcut(QKeySequence("Shift+Right"), self, lambda: self.player.seek_relative(30))

        # Speed
        QShortcut(QKeySequence(Qt.Key.Key_BracketLeft), self, self.player.speed_down)
        QShortcut(QKeySequence(Qt.Key.Key_BracketRight), self, self.player.speed_up)

        # Navigation
        QShortcut(QKeySequence(Qt.Key.Key_N), self, self._play_next)
        QShortcut(QKeySequence(Qt.Key.Key_P), self, self._play_prev)

    def _restore_geometry(self) -> None:
        """Restore window geometry from config."""
        if self.app.config and self.app.config.window_geometry:
            x, y, w, h = self.app.config.window_geometry
            self.setGeometry(x, y, w, h)

    def _save_geometry(self) -> None:
        """Save window geometry to config."""
        if self.app.config:
            geom = self.geometry()
            self.app.config.window_geometry = (
                geom.x(),
                geom.y(),
                geom.width(),
                geom.height(),
            )

    @safe_slot
    def _on_format_changed(self, index: int) -> None:
        """Handle text format change from combo box.

        Args:
            index: Index of selected item in combo box
        """
        format_value = self.format_selector.itemData(index)
        if not format_value:
            return

        # Convert to TextFormat enum
        try:
            fmt = TextFormat(format_value)
        except ValueError:
            logger.error(f"Invalid text format: {format_value}")
            return

        # Apply to text viewer
        self.text_viewer.set_format(fmt)
        logger.info(f"Text format changed to: {fmt.value}")

        # Save to config
        if self.app.config:
            self.app.config.text_format = format_value
            self.app.config.save()

    def _restore_text_format(self) -> None:
        """Restore saved text format from config."""
        if not self.app.config:
            return

        # Get saved format
        saved_format = self.app.config.text_format

        # Convert to TextFormat enum
        try:
            fmt = TextFormat(saved_format)
        except ValueError:
            logger.warning(f"Invalid saved format {saved_format!r}, using default")
            fmt = TextFormat.PLAIN

        # Set in text viewer
        self.text_viewer.set_format(fmt)

        # Update combo box selection (without triggering signal)
        index = self.format_selector.findData(fmt.value)
        if index >= 0:
            self.format_selector.blockSignals(True)
            self.format_selector.setCurrentIndex(index)
            self.format_selector.blockSignals(False)

        logger.debug(f"Restored text format: {fmt.value}")

    def closeEvent(self, event) -> None:
        """Handle close event - hide to tray instead of closing."""
        self._save_geometry()
        event.ignore()
        self.hide()
