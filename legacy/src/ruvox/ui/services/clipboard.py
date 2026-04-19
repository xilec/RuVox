"""Clipboard access service with Wayland support.

On Wayland, clipboard access is restricted to focused applications.
This module provides fallback methods using wl-paste for non-focused access.
"""

import logging
import os
import subprocess
from typing import Optional

from PyQt6.QtWidgets import QApplication

logger = logging.getLogger(__name__)


def get_clipboard_text() -> Optional[str]:
    """Get text from clipboard with Wayland fallback.

    Tries Qt clipboard first, then falls back to wl-paste on Wayland
    if Qt returns empty (which happens when app doesn't have focus).

    Returns:
        Clipboard text or None if empty/error
    """
    # Try Qt clipboard first
    clipboard = QApplication.clipboard()
    text = clipboard.text()

    if text and text.strip():
        logger.debug("Got clipboard text via Qt")
        return text.strip()

    # On Wayland, try wl-paste as fallback
    if _is_wayland():
        wayland_text = _get_clipboard_wayland()
        if wayland_text and wayland_text.strip():
            logger.debug("Got clipboard text via wl-paste")
            return wayland_text.strip()

    logger.debug("Clipboard is empty")
    return None


def _is_wayland() -> bool:
    """Check if running on Wayland."""
    return bool(os.environ.get("WAYLAND_DISPLAY"))


def _get_clipboard_wayland() -> Optional[str]:
    """Get clipboard text using wl-paste."""
    try:
        result = subprocess.run(
            ["wl-paste", "--no-newline"],
            capture_output=True,
            text=True,
            timeout=2,
        )
        if result.returncode == 0:
            return result.stdout
        else:
            logger.debug("wl-paste returned error: %s", result.stderr)
            return None
    except FileNotFoundError:
        logger.debug("wl-paste not found")
        return None
    except subprocess.TimeoutExpired:
        logger.warning("wl-paste timeout")
        return None
    except Exception as e:
        logger.warning("wl-paste error: %s", e)
        return None
