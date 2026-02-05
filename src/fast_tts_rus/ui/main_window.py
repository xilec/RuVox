"""Main application window."""

from PyQt6.QtCore import Qt
from PyQt6.QtWidgets import (
    QMainWindow,
    QWidget,
    QVBoxLayout,
    QSplitter,
    QLabel,
    QStatusBar,
)

from fast_tts_rus.ui.widgets.queue_list import QueueListWidget


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

    def _setup_ui(self) -> None:
        """Setup the user interface."""
        central_widget = QWidget()
        self.setCentralWidget(central_widget)

        main_layout = QVBoxLayout(central_widget)
        main_layout.setContentsMargins(8, 8, 8, 8)
        main_layout.setSpacing(8)

        # Player placeholder (top)
        player_placeholder = QLabel("[ Плеер - будет реализован ]")
        player_placeholder.setAlignment(Qt.AlignmentFlag.AlignCenter)
        player_placeholder.setStyleSheet(
            "background-color: #f0f0f0; border: 1px solid #ccc; padding: 20px;"
        )
        player_placeholder.setMinimumHeight(80)
        main_layout.addWidget(player_placeholder)

        # Splitter for queue and text viewer
        splitter = QSplitter(Qt.Orientation.Horizontal)

        # Queue list (left)
        self.queue_list = QueueListWidget()
        self.queue_list.setMinimumWidth(200)
        splitter.addWidget(self.queue_list)

        # Connect queue signals
        self._connect_queue_signals()

        # Text viewer placeholder (right)
        text_placeholder = QLabel("[ Текст ]")
        text_placeholder.setAlignment(Qt.AlignmentFlag.AlignCenter)
        text_placeholder.setStyleSheet(
            "background-color: #ffffff; border: 1px solid #ccc;"
        )
        splitter.addWidget(text_placeholder)

        # Set initial splitter sizes (1:2 ratio)
        splitter.setSizes([300, 600])

        main_layout.addWidget(splitter, 1)  # stretch factor 1

        # Status bar
        self.status_bar = QStatusBar()
        self.setStatusBar(self.status_bar)
        self._update_status_bar()

    def _connect_queue_signals(self) -> None:
        """Connect queue list signals."""
        self.queue_list.entry_selected.connect(self._on_entry_selected)
        self.queue_list.entry_play_requested.connect(self._on_entry_play_requested)
        self.queue_list.entry_regenerate_requested.connect(self._on_entry_regenerate_requested)
        self.queue_list.entry_delete_requested.connect(self._on_entry_delete_requested)

    def _on_entry_selected(self, entry) -> None:
        """Handle entry selection - show text in viewer."""
        # TODO: Update text viewer
        pass

    def _on_entry_play_requested(self, entry) -> None:
        """Handle play request."""
        # TODO: Start playback
        pass

    def _on_entry_regenerate_requested(self, entry) -> None:
        """Handle regenerate request."""
        # TODO: Queue for TTS regeneration
        pass

    def _on_entry_delete_requested(self, entry) -> None:
        """Handle delete request."""
        if self.app.storage:
            self.app.storage.delete_entry(entry.id)
            self.queue_list.remove_entry(entry.id)
            self._update_status_bar()

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

    def add_entry(self, entry) -> None:
        """Add a new entry to the queue list."""
        self.queue_list.add_entry(entry)
        self._update_status_bar()

    def _setup_shortcuts(self) -> None:
        """Setup keyboard shortcuts."""
        # TODO: Implement player hotkeys
        pass

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

    def closeEvent(self, event) -> None:
        """Handle close event - hide to tray instead of closing."""
        self._save_geometry()
        event.ignore()
        self.hide()
