"""Main application class coordinating all UI components."""

import logging
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
from fast_tts_rus.ui.services.logging_service import safe_slot

logger = logging.getLogger(__name__)
from fast_tts_rus.ui.services.storage import StorageService
from fast_tts_rus.ui.services.cleanup import CleanupWorker
from fast_tts_rus.ui.services.tts_worker import TTSWorker
from fast_tts_rus.ui.services.hotkeys import HotkeyService
from fast_tts_rus.ui.services.clipboard import get_clipboard_text


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
        self.tts_worker: TTSWorker | None = None
        self.hotkey_service: HotkeyService | None = None
        self.tray_icon: QSystemTrayIcon | None = None
        self._main_window = None  # Lazy loaded
        self._hotkey_warning_shown = False  # Show hotkey warning only once

    def start(self) -> None:
        """Initialize and start the application."""
        self._load_config()
        self._init_services()
        self._init_tray()
        self._connect_signals()
        self._register_hotkeys()

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
        self.tts_worker = TTSWorker(self.config, self.storage, self)
        self.hotkey_service = HotkeyService(self.config, self)
        # Run initial cleanup
        self.cleanup_worker.run_cleanup()

    def _init_tray(self) -> None:
        """Initialize system tray icon and menu."""
        self.tray_icon = QSystemTrayIcon(self)

        # Set custom icon
        icon_path = Path(__file__).parent / "resources" / "tray_icon.svg"
        if icon_path.exists():
            icon = QIcon(str(icon_path))
        else:
            # Fallback to theme icon
            icon = QIcon.fromTheme("audio-speakers", QIcon.fromTheme("multimedia-audio-player"))
        if icon.isNull():
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

        # Hotkey service signals
        self.hotkey_service.read_now_triggered.connect(self.read_now)
        self.hotkey_service.read_later_triggered.connect(self.read_later)
        self.hotkey_service.registration_failed.connect(self._on_hotkey_registration_failed)

        # TTS worker signals
        self.tts_worker.completed.connect(self._on_tts_completed)
        self.tts_worker.error.connect(self._on_tts_error)
        self.tts_worker.play_requested.connect(self._on_play_requested)
        self.tts_worker.model_loading.connect(self._on_model_loading)
        self.tts_worker.model_loaded.connect(self._on_model_loaded)
        self.tts_worker.model_error.connect(self._on_model_error)

    @safe_slot
    def _on_tray_activated(self, reason: QSystemTrayIcon.ActivationReason) -> None:
        """Handle tray icon activation."""
        # DoubleClick - standard on most platforms
        # Trigger - used on some Linux DEs (e.g., KDE, some GNOME extensions)
        if reason in (
            QSystemTrayIcon.ActivationReason.DoubleClick,
            QSystemTrayIcon.ActivationReason.Trigger,
        ):
            self.show_window()

    # Public methods

    def read_now(self) -> None:
        """Read text from clipboard immediately."""
        self._process_clipboard(play_when_ready=True)

    def read_later(self) -> None:
        """Add text from clipboard to queue."""
        self._process_clipboard(play_when_ready=False)

    def _process_clipboard(self, play_when_ready: bool) -> None:
        """Read clipboard and queue for TTS processing."""
        text = get_clipboard_text()

        if not text:
            self.tray_icon.showMessage(
                "Fast TTS RUS",
                "Буфер обмена пуст",
                QSystemTrayIcon.MessageIcon.Warning,
                2000,
            )
            return

        entry = self.storage.add_entry(text)
        if self._main_window is not None:
            self._main_window.add_entry(entry)
        self.tts_worker.process(entry, play_when_ready=play_when_ready)

        prefix = "Обработка" if play_when_ready else "В очередь"
        preview = f"{text[:50]}..." if len(text) > 50 else text
        self.tray_icon.showMessage(
            "Fast TTS RUS",
            f"{prefix}: {preview}",
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

    @safe_slot
    def _on_read_now(self) -> None:
        """Handle read_now signal."""
        self.read_now()

    @safe_slot
    def _on_read_later(self) -> None:
        """Handle read_later signal."""
        self.read_later()

    @safe_slot
    def _on_tts_completed(self, entry_id: str) -> None:
        """Handle TTS completion."""
        entry = self.storage.get_entry(entry_id)
        if entry and self._main_window is not None:
            self._main_window.queue_list.update_entry(entry)

        # Enable play button if this is the first ready entry
        self.action_play.setEnabled(True)

        # Show notification if configured
        if self.config.notify_on_ready:
            self.tray_icon.showMessage(
                "Fast TTS RUS",
                "Аудио готово к воспроизведению",
                QSystemTrayIcon.MessageIcon.Information,
                2000,
            )

    @safe_slot
    def _on_tts_error(self, entry_id: str, error_msg: str) -> None:
        """Handle TTS error."""
        entry = self.storage.get_entry(entry_id)
        if entry and self._main_window is not None:
            self._main_window.queue_list.update_entry(entry)

        # Show notification if configured
        if self.config.notify_on_error:
            self.tray_icon.showMessage(
                "Fast TTS RUS",
                f"Ошибка: {error_msg[:50]}",
                QSystemTrayIcon.MessageIcon.Critical,
                3000,
            )

    @safe_slot
    def _on_play_requested(self, entry_id: str) -> None:
        """Handle play request from TTS worker."""
        # Show window and play
        self.show_window()
        if self._main_window:
            self._main_window.play_entry(entry_id)

    @safe_slot
    def _on_model_loading(self) -> None:
        """Handle model loading started."""
        self.tray_icon.showMessage(
            "Fast TTS RUS",
            "Загрузка модели TTS...",
            QSystemTrayIcon.MessageIcon.Information,
            3000,
        )

    @safe_slot
    def _on_model_loaded(self) -> None:
        """Handle model loaded."""
        self.tray_icon.showMessage(
            "Fast TTS RUS",
            "Модель TTS загружена",
            QSystemTrayIcon.MessageIcon.Information,
            2000,
        )

    @safe_slot
    def _on_model_error(self, error_msg: str) -> None:
        """Handle model loading error."""
        self.tray_icon.showMessage(
            "Fast TTS RUS",
            f"Ошибка загрузки модели: {error_msg[:50]}",
            QSystemTrayIcon.MessageIcon.Critical,
            5000,
        )

    def _register_hotkeys(self) -> None:
        """Try to register global hotkeys."""
        # This will emit registration_failed if unsuccessful
        self.hotkey_service.register()

    @safe_slot
    def _on_hotkey_registration_failed(self, message: str) -> None:
        """Handle hotkey registration failure.

        Shows a brief notification - user can see full instructions
        in the settings dialog.
        """
        # Only show once
        if self._hotkey_warning_shown:
            return
        self._hotkey_warning_shown = True

        # Only show brief message, full instructions available in settings
        self.tray_icon.showMessage(
            "Fast TTS RUS",
            "Глобальные хоткеи недоступны. См. настройки для ручной настройки.",
            QSystemTrayIcon.MessageIcon.Information,
            3000,
        )

    def _show_settings(self) -> None:
        """Show settings dialog."""
        from fast_tts_rus.ui.dialogs.settings import SettingsDialog
        dialog = SettingsDialog(self.config, self.hotkey_service)
        dialog.exec()

    def _quit(self) -> None:
        """Quit the application."""
        if self.hotkey_service:
            self.hotkey_service.unregister()

        if self.tts_worker:
            self.tts_worker.shutdown()

        if self.config:
            self.config.save()

        from fast_tts_rus.ui.services.logging_service import shutdown_logging
        shutdown_logging()

        QApplication.quit()
