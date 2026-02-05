"""Main application window."""

from PyQt6.QtCore import Qt
from PyQt6.QtWidgets import (
    QMainWindow,
    QWidget,
    QVBoxLayout,
    QHBoxLayout,
    QSplitter,
    QLabel,
    QStatusBar,
)


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

        # Queue list placeholder (left)
        queue_placeholder = QLabel("[ Очередь/История ]")
        queue_placeholder.setAlignment(Qt.AlignmentFlag.AlignCenter)
        queue_placeholder.setStyleSheet(
            "background-color: #f8f8f8; border: 1px solid #ccc;"
        )
        queue_placeholder.setMinimumWidth(200)
        splitter.addWidget(queue_placeholder)

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
        status_bar = QStatusBar()
        self.setStatusBar(status_bar)
        status_bar.showMessage("Готово | Очередь: 0")

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
