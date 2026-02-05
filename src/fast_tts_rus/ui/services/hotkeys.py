"""Global hotkey service via xdg-desktop-portal GlobalShortcuts.

This service registers global hotkeys through the xdg-desktop-portal
GlobalShortcuts interface, which works on Wayland compositors.

Requirements:
- xdg-desktop-portal >= 1.14
- Desktop environment with GlobalShortcuts support:
  - GNOME 44+
  - KDE Plasma 5.27+
  - Other portal-compatible desktops
- dasbus library (for D-Bus communication)
- PyGObject (gi) - required by dasbus
"""

import logging
import uuid

from PyQt6.QtCore import QObject, QTimer, pyqtSignal

from fast_tts_rus.ui.models.config import UIConfig

logger = logging.getLogger(__name__)

# Check if dasbus is available
try:
    from dasbus.connection import SessionMessageBus
    from dasbus.typing import get_variant, Str

    DASBUS_AVAILABLE = True
except ImportError:
    DASBUS_AVAILABLE = False
    logger.warning("dasbus not available, global hotkeys will use fallback")


class HotkeyService(QObject):
    """Global hotkey service using xdg-desktop-portal GlobalShortcuts.

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
    SHORTCUTS_IFACE = "org.freedesktop.portal.GlobalShortcuts"
    SESSION_IFACE = "org.freedesktop.portal.Session"

    # Shortcut IDs
    SHORTCUT_READ_NOW = "read-now"
    SHORTCUT_READ_LATER = "read-later"

    def __init__(self, config: UIConfig, parent=None):
        super().__init__(parent)
        self.config = config
        self._session_handle: str | None = None
        self._session_token: str | None = None
        self._registered = False
        self._fallback_emitted = False
        self._bus = None
        self._portal = None
        self._poll_timer: QTimer | None = None

    def register(self) -> bool:
        """Register global hotkeys via portal.

        Returns:
            True if registration was initiated successfully
        """
        if not DASBUS_AVAILABLE:
            self._emit_fallback("dasbus library not available")
            return False

        try:
            self._bus = SessionMessageBus()
        except Exception as e:
            logger.warning("Failed to connect to D-Bus: %s", e)
            self._emit_fallback(f"D-Bus connection failed: {e}")
            return False

        # Get the GlobalShortcuts portal interface
        try:
            self._portal = self._bus.get_proxy(
                self.PORTAL_SERVICE,
                self.PORTAL_PATH,
                interface_name=self.SHORTCUTS_IFACE,
            )
        except Exception as e:
            logger.warning("Failed to get GlobalShortcuts proxy: %s", e)
            self._emit_fallback(f"GlobalShortcuts portal not available: {e}")
            return False

        # Create session and bind shortcuts
        try:
            self._create_session_and_bind()
            return True
        except Exception as e:
            logger.warning("Failed to register hotkeys: %s", e)
            self._emit_fallback(f"Failed to register hotkeys: {e}")
            return False

    def _get_sender_name(self) -> str:
        """Get the sender name for D-Bus (connection name without leading colon)."""
        name = self._bus.connection.get_unique_name()
        if name.startswith(":"):
            name = name[1:]
        return name.replace(".", "_")

    def _create_session_and_bind(self) -> None:
        """Create a GlobalShortcuts session and bind shortcuts."""
        import time

        self._session_token = f"fast_tts_session_{uuid.uuid4().hex[:8]}"
        handle_token = f"fast_tts_req_{uuid.uuid4().hex[:8]}"

        options = {
            "handle_token": get_variant(Str, handle_token),
            "session_handle_token": get_variant(Str, self._session_token),
        }

        logger.debug("Creating GlobalShortcuts session...")
        request_path = self._portal.CreateSession(options)
        logger.debug("CreateSession request: %s", request_path)

        # Construct session handle from our tokens
        # The portal constructs the handle as: /org/freedesktop/portal/desktop/session/{sender}/{session_token}
        sender = self._get_sender_name()
        self._session_handle = (
            f"/org/freedesktop/portal/desktop/session/{sender}/{self._session_token}"
        )
        logger.debug("Session handle: %s", self._session_handle)

        # Give the portal a moment to process the session creation
        # This is a workaround for not properly handling the Response signal
        time.sleep(0.2)

        # Bind shortcuts
        self._bind_shortcuts()

        # Start listening for activations
        self._start_activation_polling()

    def _bind_shortcuts(self) -> None:
        """Bind shortcuts to the session."""
        if not self._session_handle or not self._portal:
            return

        shortcuts = [
            (
                self.SHORTCUT_READ_NOW,
                {
                    "description": get_variant(Str, "Read clipboard now"),
                    "preferred_trigger": get_variant(Str, self.config.hotkey_read_now),
                },
            ),
            (
                self.SHORTCUT_READ_LATER,
                {
                    "description": get_variant(Str, "Read clipboard later"),
                    "preferred_trigger": get_variant(
                        Str, self.config.hotkey_read_later
                    ),
                },
            ),
        ]

        bind_token = f"fast_tts_bind_{uuid.uuid4().hex[:8]}"
        options = {
            "handle_token": get_variant(Str, bind_token),
        }

        logger.debug("Binding shortcuts: %s", [s[0] for s in shortcuts])
        request_path = self._portal.BindShortcuts(
            self._session_handle,
            shortcuts,
            "",  # parent_window
            options,
        )
        logger.debug("BindShortcuts request: %s", request_path)
        self._registered = True

    def _start_activation_polling(self) -> None:
        """Start polling for shortcut activations.

        Note: This is a workaround because integrating GLib signals with Qt
        event loop requires more complex setup. For proper integration,
        we would need to use QSocketNotifier or GLib-Qt integration.
        """
        from gi.repository import Gio, GLib

        try:
            # Subscribe to the Activated signal on the session path
            # using low-level D-Bus signal subscription
            connection = self._bus.connection
            connection.signal_subscribe(
                self.PORTAL_SERVICE,  # sender
                self.SHORTCUTS_IFACE,  # interface
                "Activated",  # signal name
                self._session_handle,  # object path
                None,  # arg0
                Gio.DBusSignalFlags.NONE,
                self._on_activated_signal,
                None,  # user_data
            )
            logger.debug("Subscribed to Activated signal on %s", self._session_handle)

            # Start polling GLib events to receive signals
            self._poll_timer = QTimer(self)
            self._poll_timer.timeout.connect(self._poll_glib_events)
            self._poll_timer.start(50)  # Poll every 50ms
        except Exception as e:
            logger.warning("Failed to subscribe to Activated signal: %s", e)

    def _on_activated_signal(
        self,
        connection,
        sender_name: str,
        object_path: str,
        interface_name: str,
        signal_name: str,
        parameters,
        user_data,
    ) -> None:
        """Handle raw D-Bus Activated signal."""
        try:
            # Parameters: (session_handle, shortcut_id, timestamp, options)
            session_handle = parameters[0]
            shortcut_id = parameters[1]
            logger.debug("Shortcut activated: %s", shortcut_id)

            if shortcut_id == self.SHORTCUT_READ_NOW:
                self.read_now_triggered.emit()
            elif shortcut_id == self.SHORTCUT_READ_LATER:
                self.read_later_triggered.emit()
        except Exception as e:
            logger.warning("Error handling Activated signal: %s", e)

    def _poll_glib_events(self) -> None:
        """Poll GLib main context for pending events."""
        try:
            from gi.repository import GLib

            context = GLib.MainContext.default()
            while context.pending():
                context.iteration(False)
        except Exception:
            pass

    def unregister(self) -> None:
        """Unregister global hotkeys and close session."""
        if self._poll_timer:
            self._poll_timer.stop()
            self._poll_timer = None

        if self._session_handle and self._bus:
            try:
                session_proxy = self._bus.get_proxy(
                    self.PORTAL_SERVICE,
                    self._session_handle,
                    interface_name=self.SESSION_IFACE,
                )
                session_proxy.Close()
                logger.debug("Session closed")
            except Exception as e:
                logger.debug("Failed to close session: %s", e)

        self._session_handle = None
        self._registered = False
        self._portal = None
        self._bus = None

    def is_registered(self) -> bool:
        """Check if hotkeys are registered."""
        return self._registered

    def _emit_fallback(self, reason: str) -> None:
        """Emit registration failed with fallback instructions.

        Only emits once to avoid multiple notifications.
        """
        if self._fallback_emitted:
            return
        self._fallback_emitted = True

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

For Sway/Hyprland:
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
