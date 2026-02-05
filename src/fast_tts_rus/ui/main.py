"""Entry point for the UI application."""

import sys
from pathlib import Path

from PyQt6.QtWidgets import QApplication
from PyQt6.QtNetwork import QLocalServer, QLocalSocket

from fast_tts_rus.ui.app import TTSApplication


APP_ID = "fast-tts-rus"


def main() -> int:
    """Entry point for UI application."""
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
        print(f"Failed to create local server: {server.errorString()}", file=sys.stderr)

    # Create and start application
    tts_app = TTSApplication()
    tts_app.start()

    # Handle commands from other instances
    def handle_remote_command():
        conn = server.nextPendingConnection()
        if conn and conn.waitForReadyRead(1000):
            cmd = conn.readAll().data().decode()
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

    return app.exec()


if __name__ == "__main__":
    sys.exit(main())
