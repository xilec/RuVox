"""Global hotkey service using evdev for direct keyboard input.

This service reads keyboard events directly from /dev/input devices,
bypassing Wayland compositor restrictions. Works on both X11 and Wayland.

Requirements:
- evdev library (pip install evdev or evdev-binary)
- User must be in 'input' group: sudo usermod -aG input $USER
- Logout/login required after adding to input group
"""

import logging
import threading
from pathlib import Path

from PyQt6.QtCore import QObject, pyqtSignal

from ruvox.ui.models.config import UIConfig

logger = logging.getLogger(__name__)

# Check if evdev is available
try:
    import evdev
    from evdev import ecodes

    EVDEV_AVAILABLE = True
except ImportError:
    EVDEV_AVAILABLE = False
    evdev = None
    ecodes = None
    logger.warning("evdev not available, global hotkeys disabled")


class HotkeyService(QObject):
    """Global hotkey service using evdev for direct keyboard input.

    Signals:
        read_now_triggered: Emitted when read_now hotkey is activated
        read_later_triggered: Emitted when read_later hotkey is activated
        registration_failed: Emitted with error message if registration fails
    """

    read_now_triggered = pyqtSignal()
    read_later_triggered = pyqtSignal()
    registration_failed = pyqtSignal(str)  # error message

    # Modifier key codes
    CTRL_KEYS = set()
    ALT_KEYS = set()
    SHIFT_KEYS = set()

    def __init__(self, config: UIConfig, parent=None):
        super().__init__(parent)
        self.config = config
        self._registered = False
        self._running = False
        self._fallback_emitted = False
        self._stop_event = threading.Event()
        self._threads: list[threading.Thread] = []
        self._devices: list = []

        # Modifier state tracking (shared across all devices)
        self._ctrl_pressed = False
        self._alt_pressed = False
        self._shift_pressed = False
        self._lock = threading.Lock()

        # Parse hotkey configurations
        self._read_now_key = None
        self._read_now_mods = set()
        self._read_later_key = None
        self._read_later_mods = set()

        # Initialize key code sets if evdev is available
        if EVDEV_AVAILABLE:
            self.CTRL_KEYS = {ecodes.KEY_LEFTCTRL, ecodes.KEY_RIGHTCTRL}
            self.ALT_KEYS = {ecodes.KEY_LEFTALT, ecodes.KEY_RIGHTALT}
            self.SHIFT_KEYS = {ecodes.KEY_LEFTSHIFT, ecodes.KEY_RIGHTSHIFT}

    def _parse_hotkey(self, hotkey_str: str) -> tuple[int | None, set[str]]:
        """Parse hotkey string like 'CTRL+ALT+R' into key code and modifiers.

        Returns:
            Tuple of (key_code, set of modifier names)
        """
        if not EVDEV_AVAILABLE:
            return None, set()

        parts = [p.strip().upper() for p in hotkey_str.split("+")]
        modifiers = set()
        key_code = None

        for part in parts:
            if part in ("CTRL", "CONTROL"):
                modifiers.add("CTRL")
            elif part in ("ALT",):
                modifiers.add("ALT")
            elif part in ("SHIFT",):
                modifiers.add("SHIFT")
            else:
                # Try to find key code
                key_name = f"KEY_{part}"
                key_code = getattr(ecodes, key_name, None)
                if key_code is None:
                    logger.warning("Unknown key: %s", part)

        return key_code, modifiers

    def _find_keyboards(self) -> list:
        """Find all keyboard devices."""
        if not EVDEV_AVAILABLE:
            return []

        keyboards = []
        input_dir = Path("/dev/input")

        if not input_dir.exists():
            logger.warning("/dev/input does not exist")
            return []

        for path in input_dir.glob("event*"):
            device = None
            try:
                device = evdev.InputDevice(str(path))
                capabilities = device.capabilities()

                # Check if device has key events (EV_KEY)
                if ecodes.EV_KEY in capabilities:
                    keys = capabilities[ecodes.EV_KEY]
                    # Check if it has common keyboard keys (letters)
                    if ecodes.KEY_A in keys and ecodes.KEY_Z in keys:
                        logger.debug("Found keyboard: %s (%s)", device.name, device.path)
                        keyboards.append(device)
                        device = None  # Ownership transferred to keyboards list
            except PermissionError:
                logger.debug("Permission denied for %s", path)
            except OSError as e:
                logger.debug("Cannot open %s: %s", path, e)
            finally:
                # Close device if not added to keyboards list
                if device is not None:
                    try:
                        device.close()
                    except Exception:
                        pass

        return keyboards

    def _check_hotkey(self, key_code: int) -> str | None:
        """Check if current key + modifiers match any hotkey.

        Returns:
            'read_now', 'read_later', or None
        """
        with self._lock:
            current_mods = set()
            if self._ctrl_pressed:
                current_mods.add("CTRL")
            if self._alt_pressed:
                current_mods.add("ALT")
            if self._shift_pressed:
                current_mods.add("SHIFT")

        # Check read_now hotkey
        if self._read_now_key is not None and key_code == self._read_now_key and current_mods == self._read_now_mods:
            return "read_now"

        # Check read_later hotkey
        if (
            self._read_later_key is not None
            and key_code == self._read_later_key
            and current_mods == self._read_later_mods
        ):
            return "read_later"

        return None

    def _update_modifiers(self, key_code: int, pressed: bool) -> None:
        """Update modifier state."""
        with self._lock:
            if key_code in self.CTRL_KEYS:
                self._ctrl_pressed = pressed
            elif key_code in self.ALT_KEYS:
                self._alt_pressed = pressed
            elif key_code in self.SHIFT_KEYS:
                self._shift_pressed = pressed

    def _listen_device(self, device) -> None:
        """Listen for events on a single device (runs in thread)."""
        logger.debug("Starting listener for %s", device.name)

        try:
            for event in device.read_loop():
                if self._stop_event.is_set():
                    break

                # Only process key events
                if event.type != ecodes.EV_KEY:
                    continue

                key_code = event.code

                # Key pressed
                if event.value == 1:
                    # Update modifier state
                    self._update_modifiers(key_code, True)

                    # Check if this triggers a hotkey
                    hotkey = self._check_hotkey(key_code)
                    if hotkey == "read_now":
                        logger.debug("Read now hotkey triggered")
                        # Emit signal (thread-safe in PyQt)
                        self.read_now_triggered.emit()
                    elif hotkey == "read_later":
                        logger.debug("Read later hotkey triggered")
                        self.read_later_triggered.emit()

                # Key released
                elif event.value == 0:
                    self._update_modifiers(key_code, False)

        except OSError as e:
            if not self._stop_event.is_set():
                logger.warning("Device %s disconnected: %s", device.name, e)
        except Exception as e:
            if not self._stop_event.is_set():
                logger.error("Error reading %s: %s", device.name, e)
        # Note: device is closed in unregister(), not here

        logger.debug("Listener stopped for %s", device.name)

    def register(self) -> bool:
        """Register global hotkeys via evdev.

        Returns:
            True if registration was successful
        """
        if not EVDEV_AVAILABLE:
            self._emit_fallback("evdev library not available")
            return False

        # Parse hotkey configurations
        self._read_now_key, self._read_now_mods = self._parse_hotkey(self.config.hotkey_read_now)
        self._read_later_key, self._read_later_mods = self._parse_hotkey(self.config.hotkey_read_later)

        if self._read_now_key is None and self._read_later_key is None:
            self._emit_fallback("Failed to parse hotkey configurations")
            return False

        # Find keyboard devices
        self._devices = self._find_keyboards()

        if not self._devices:
            self._emit_fallback(
                "No keyboard devices found. Make sure you are in the 'input' group:\n"
                "  sudo usermod -aG input $USER\n"
                "Then logout and login again."
            )
            return False

        # Start listener threads
        self._stop_event.clear()
        self._running = True
        for device in self._devices:
            thread = threading.Thread(
                target=self._listen_device,
                args=(device,),
                daemon=True,
                name=f"hotkey-{device.name}",
            )
            thread.start()
            self._threads.append(thread)

        self._registered = True
        logger.info(
            "Registered hotkeys on %d keyboard(s): %s",
            len(self._devices),
            ", ".join(d.name for d in self._devices),
        )
        return True

    def unregister(self) -> None:
        """Unregister global hotkeys and stop listeners."""
        if not self._registered:
            return

        self._running = False
        self._stop_event.set()

        # 1. Close devices first to unblock read_loop()
        for device in self._devices:
            try:
                device.close()
            except Exception:
                pass

        # 2. Wait for threads to finish (increased timeout)
        for thread in self._threads:
            thread.join(timeout=2.0)

        # 3. Clear lists
        self._threads.clear()
        self._devices.clear()
        self._registered = False

        # Reset modifier state
        with self._lock:
            self._ctrl_pressed = False
            self._alt_pressed = False
            self._shift_pressed = False

        logger.info("Hotkey service unregistered")

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

To use global hotkeys with evdev:

1. Add yourself to the 'input' group:
   sudo usermod -aG input $USER

2. Logout and login again (or reboot)

3. Restart RuVox

Alternatively, configure hotkeys manually in your desktop environment:

For KDE Plasma:
  System Settings → Shortcuts → Custom Shortcuts
  Add: "Read Now" → ruvox --read-now
  Add: "Read Later" → ruvox --read-later

For GNOME:
  Settings → Keyboard → View and Customize Shortcuts → Custom Shortcuts
  Add: "Read Now" → ruvox --read-now
  Add: "Read Later" → ruvox --read-later

Suggested shortcuts:
  Read Now: {self.config.hotkey_read_now}
  Read Later: {self.config.hotkey_read_later}
"""
        self.registration_failed.emit(message)

    def get_fallback_instructions(self) -> str:
        """Get manual configuration instructions."""
        return f"""To use global hotkeys, configure them manually in your desktop environment:

Commands to bind:
  Read Now: ruvox --read-now
  Read Later: ruvox --read-later

Suggested shortcuts:
  Read Now: {self.config.hotkey_read_now}
  Read Later: {self.config.hotkey_read_later}
"""
