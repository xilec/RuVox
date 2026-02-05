"""Main application class coordinating all UI components."""

from pathlib import Path

from PyQt6.QtCore import QObject, pyqtSignal
from PyQt6.QtGui import QIcon, QAction
from PyQt6.QtWidgets import (
    QApplication,
    QSystemTrayIcon,
    QMenu,
    QMessageBox,
)

from fast_tts_rus.ui.models.config import UIConfig
from fast_tts_rus.ui.services.storage import StorageService
from fast_tts_rus.ui.services.cleanup import CleanupWorker


class TTSApplication(QObject):
    """Main application coordinating all components.

    Manages:
    - System tray icon and menu
    - Configuration
    - Storage service
    - Main window (lazy loaded)
    - TTS worker (to be implemented)
    - Hotkey service (to be implemented)
    """

    # Signals
    read_now_triggered = pyqtSignal()
    read_later_triggered = pyqtSignal()

    def __init__(self, parent: QObject | None = None):
        super().__init__(parent)

        self.config: UIConfig | None = None
        self.storage: StorageService | None = None
        self.cleanup_worker: CleanupWorker | None = None
        self.tray_icon: QSystemTrayIcon | None = None
        self._main_window = None  # Lazy loaded

    def start(self) -> None:
        """Initialize and start the application."""
        self._load_config()
        self._init_services()
        self._init_tray()
        self._connect_signals()

    def _load_config(self) -> None:
        """Load configuration from file."""
        # First create default config to get cache_dir
        default_config = UIConfig()
        config_path = default_config.cache_dir / "config.json"
        self.config = UIConfig.load(config_path)

    def _init_services(self) -> None:
        """Initialize services."""
        self.storage = StorageService(self.config)
        self.cleanup_worker = CleanupWorker(self.config, self.storage, self)
        # Run initial cleanup
        self.cleanup_worker.run_cleanup()

    def _init_tray(self) -> None:
        """Initialize system tray icon and menu."""
        self.tray_icon = QSystemTrayIcon(self)

        # Set icon (use a placeholder for now)
        # TODO: Use proper SVG icon
        icon = QIcon.fromTheme("audio-speakers", QIcon.fromTheme("multimedia-audio-player"))
        if icon.isNull():
            # Fallback to application icon
            icon = QApplication.instance().windowIcon()
        self.tray_icon.setIcon(icon)
        self.tray_icon.setToolTip("Fast TTS RUS")

        # Create menu
        menu = QMenu()

        # Playback controls
        self.action_play = QAction("Воспроизвести", self)
        self.action_play.setEnabled(False)  # Enable when there's something to play
        menu.addAction(self.action_play)

        self.action_pause = QAction("Пауза", self)
        self.action_pause.setEnabled(False)
        menu.addAction(self.action_pause)

        menu.addSeparator()

        # Main actions
        action_read_now = QAction("Читать сразу", self)
        action_read_now.setShortcut(self.config.hotkey_read_now)
        action_read_now.triggered.connect(self.read_now)
        menu.addAction(action_read_now)

        action_read_later = QAction("Читать отложенно", self)
        action_read_later.setShortcut(self.config.hotkey_read_later)
        action_read_later.triggered.connect(self.read_later)
        menu.addAction(action_read_later)

        menu.addSeparator()

        # Settings and window
        action_settings = QAction("Настройки...", self)
        action_settings.triggered.connect(self._show_settings)
        menu.addAction(action_settings)

        action_show = QAction("Открыть окно", self)
        action_show.triggered.connect(self.show_window)
        menu.addAction(action_show)

        menu.addSeparator()

        # Exit
        action_quit = QAction("Выход", self)
        action_quit.triggered.connect(self._quit)
        menu.addAction(action_quit)

        self.tray_icon.setContextMenu(menu)

        # Double-click to show window
        self.tray_icon.activated.connect(self._on_tray_activated)

        self.tray_icon.show()

    def _connect_signals(self) -> None:
        """Connect internal signals."""
        self.read_now_triggered.connect(self._on_read_now)
        self.read_later_triggered.connect(self._on_read_later)

    def _on_tray_activated(self, reason: QSystemTrayIcon.ActivationReason) -> None:
        """Handle tray icon activation."""
        if reason == QSystemTrayIcon.ActivationReason.DoubleClick:
            self.show_window()

    # Public methods

    def read_now(self) -> None:
        """Read text from clipboard immediately."""
        clipboard = QApplication.clipboard()
        text = clipboard.text()

        if not text or not text.strip():
            self.tray_icon.showMessage(
                "Fast TTS RUS",
                "Буфер обмена пуст",
                QSystemTrayIcon.MessageIcon.Warning,
                2000,
            )
            return

        entry = self.storage.add_entry(text.strip())
        # Update window if open
        if self._main_window is not None:
            self._main_window.add_entry(entry)
        # TODO: Start TTS processing with play_when_ready=True
        self.tray_icon.showMessage(
            "Fast TTS RUS",
            f"Добавлено: {text[:50]}..." if len(text) > 50 else f"Добавлено: {text}",
            QSystemTrayIcon.MessageIcon.Information,
            2000,
        )

    def read_later(self) -> None:
        """Add text from clipboard to queue."""
        clipboard = QApplication.clipboard()
        text = clipboard.text()

        if not text or not text.strip():
            self.tray_icon.showMessage(
                "Fast TTS RUS",
                "Буфер обмена пуст",
                QSystemTrayIcon.MessageIcon.Warning,
                2000,
            )
            return

        entry = self.storage.add_entry(text.strip())
        # Update window if open
        if self._main_window is not None:
            self._main_window.add_entry(entry)
        # TODO: Start TTS processing with play_when_ready=False
        self.tray_icon.showMessage(
            "Fast TTS RUS",
            f"В очередь: {text[:50]}..." if len(text) > 50 else f"В очередь: {text}",
            QSystemTrayIcon.MessageIcon.Information,
            2000,
        )

    def show_window(self) -> None:
        """Show main window."""
        if self._main_window is None:
            # Lazy load main window to reduce startup time
            from fast_tts_rus.ui.main_window import MainWindow
            self._main_window = MainWindow(self)
            # Load existing entries
            self._main_window.load_entries()

        self._main_window.show()
        self._main_window.raise_()
        self._main_window.activateWindow()

    # Private methods

    def _on_read_now(self) -> None:
        """Handle read_now signal."""
        self.read_now()

    def _on_read_later(self) -> None:
        """Handle read_later signal."""
        self.read_later()

    def _show_settings(self) -> None:
        """Show settings dialog."""
        # TODO: Implement settings dialog
        QMessageBox.information(
            None,
            "Настройки",
            "Диалог настроек будет реализован позже.",
        )

    def _quit(self) -> None:
        """Quit the application."""
        # Save config before exit
        if self.config:
            self.config.save()

        QApplication.quit()
