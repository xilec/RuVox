"""Entry point for the UI application."""

import logging
import sys
from pathlib import Path

# Инициализация логирования ДО импорта PyQt6
from fast_tts_rus.ui.services.logging_service import setup_logging, setup_qt_logging, shutdown_logging
setup_logging()

from PyQt6.QtWidgets import QApplication
from PyQt6.QtNetwork import QLocalServer, QLocalSocket

# QWebEngineView requires early import (before QApplication is created).
# Graceful: if WebEngine is not installed, mermaid rendering will be unavailable.
try:
    from PyQt6.QtWebEngineWidgets import QWebEngineView  # noqa: F401
except ImportError:
    pass

# Настройка перехвата Qt сообщений после импорта PyQt6
setup_qt_logging()

from fast_tts_rus.ui.app import TTSApplication

logger = logging.getLogger(__name__)

APP_ID = "fast-tts-rus"


def main() -> int:
    """Entry point for UI application."""
    logger.info("Запуск Fast TTS RUS")

    app = QApplication(sys.argv)
    app.setApplicationName("Fast TTS RUS")
    app.setQuitOnLastWindowClosed(False)  # Don't close when window is hidden

    # Parse command line arguments
    args = sys.argv[1:]
    command = None
    if "--read-now" in args:
        command = "read-now"
    elif "--read-later" in args:
        command = "read-later"
    elif "--show" in args:
        command = "show"

    # Single instance check
    socket = QLocalSocket()
    socket.connectToServer(APP_ID)
    if socket.waitForConnected(500):
        # Application already running - send command and exit
        logger.info(f"Приложение уже запущено, отправка команды: {command or 'show'}")
        if command:
            socket.write(command.encode())
        else:
            socket.write(b"show")
        socket.waitForBytesWritten(1000)
        socket.disconnectFromServer()
        return 0

    # Create server for receiving commands from other instances
    server = QLocalServer()
    server.removeServer(APP_ID)  # Remove stale socket if exists
    if not server.listen(APP_ID):
        logger.error(f"Не удалось создать локальный сервер: {server.errorString()}")

    # Create and start application
    tts_app = TTSApplication()
    tts_app.start()

    # Handle commands from other instances
    def handle_remote_command():
        conn = server.nextPendingConnection()
        if conn and conn.waitForReadyRead(1000):
            cmd = conn.readAll().data().decode()
            logger.debug(f"Получена команда: {cmd}")
            if cmd == "read-now":
                tts_app.read_now()
            elif cmd == "read-later":
                tts_app.read_later()
            elif cmd == "show":
                tts_app.show_window()
        if conn:
            conn.disconnectFromServer()

    server.newConnection.connect(handle_remote_command)

    # Execute initial command if any
    if command == "read-now":
        tts_app.read_now()
    elif command == "read-later":
        tts_app.read_later()

    logger.info("Приложение запущено")
    result = app.exec()
    shutdown_logging()
    return result


if __name__ == "__main__":
    sys.exit(main())
