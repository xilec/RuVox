"""Global hotkey service via xdg-desktop-portal.

This service attempts to register global hotkeys through the
xdg-desktop-portal GlobalShortcuts interface. If the portal
is not available, it provides fallback instructions.

Note: GlobalShortcuts portal requires:
- xdg-desktop-portal >= 1.14
- Desktop environment support (GNOME 44+, KDE Plasma 5.27+, etc.)
"""

from PyQt6.QtCore import QObject, pyqtSignal
from PyQt6.QtDBus import QDBusConnection, QDBusInterface, QDBusMessage

from fast_tts_rus.ui.models.config import UIConfig


class HotkeyService(QObject):
    """Global hotkey service using xdg-desktop-portal.

    Signals:
        read_now_triggered: Emitted when read_now hotkey is activated
        read_later_triggered: Emitted when read_later hotkey is activated
        registration_failed: Emitted with error message if registration fails
    """

    read_now_triggered = pyqtSignal()
    read_later_triggered = pyqtSignal()
    registration_failed = pyqtSignal(str)  # error message

    PORTAL_SERVICE = "org.freedesktop.portal.Desktop"
    PORTAL_PATH = "/org/freedesktop/portal/desktop"
    PORTAL_IFACE = "org.freedesktop.portal.GlobalShortcuts"

    def __init__(self, config: UIConfig, parent=None):
        super().__init__(parent)
        self.config = config
        self._session_handle: str | None = None
        self._registered = False

    def register(self) -> bool:
        """Register global hotkeys.

        Returns:
            True if registration was successful or in progress
        """
        bus = QDBusConnection.sessionBus()
        if not bus.isConnected():
            self._emit_fallback("D-Bus session bus not available")
            return False

        # Check if GlobalShortcuts portal is available
        portal = QDBusInterface(
            self.PORTAL_SERVICE,
            self.PORTAL_PATH,
            self.PORTAL_IFACE,
            bus
        )

        if not portal.isValid():
            self._emit_fallback("GlobalShortcuts portal not available")
            return False

        # The full implementation would involve:
        # 1. CreateSession to get a session handle
        # 2. ListShortcuts to check existing shortcuts
        # 3. BindShortcuts to register our shortcuts
        # 4. Connect to Activated signal
        #
        # This is a complex async process with portal response handling.
        # For MVP, we'll use a simplified approach that shows fallback.

        # Try to check if portal responds
        try:
            # This is a simplified check - full implementation would
            # properly handle the async D-Bus calls
            self._emit_fallback(
                "Portal detected but async registration not yet implemented. "
                "Please configure hotkeys manually in your desktop environment."
            )
            return False
        except Exception as e:
            self._emit_fallback(f"Portal error: {str(e)}")
            return False

    def unregister(self) -> None:
        """Unregister global hotkeys."""
        # Would close the session if we had one
        self._session_handle = None
        self._registered = False

    def is_registered(self) -> bool:
        """Check if hotkeys are registered."""
        return self._registered

    def _emit_fallback(self, reason: str) -> None:
        """Emit registration failed with fallback instructions."""
        message = f"""{reason}

To use global hotkeys, configure them manually in your desktop environment:

For GNOME:
  Settings → Keyboard → View and Customize Shortcuts → Custom Shortcuts
  Add: "Read Now" → fast-tts-ui --read-now
  Add: "Read Later" → fast-tts-ui --read-later

For KDE Plasma:
  System Settings → Shortcuts → Custom Shortcuts
  Add: "Read Now" → fast-tts-ui --read-now
  Add: "Read Later" → fast-tts-ui --read-later

For Sway/i3:
  Add to config:
  bindsym $mod+t exec fast-tts-ui --read-now
  bindsym $mod+Shift+t exec fast-tts-ui --read-later

Suggested shortcuts:
  Read Now: {self.config.hotkey_read_now}
  Read Later: {self.config.hotkey_read_later}
"""
        self.registration_failed.emit(message)

    def get_fallback_instructions(self) -> str:
        """Get manual configuration instructions."""
        return f"""To use global hotkeys, configure them manually in your desktop environment:

Commands to bind:
  Read Now: fast-tts-ui --read-now
  Read Later: fast-tts-ui --read-later

Suggested shortcuts:
  Read Now: {self.config.hotkey_read_now}
  Read Later: {self.config.hotkey_read_later}
"""
