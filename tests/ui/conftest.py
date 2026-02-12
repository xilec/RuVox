"""Shared fixtures for UI tests.

QWebEngineView (Chromium) requires a non-empty argv in QApplication.
This conftest ensures QApplication is always created with proper args
before any module-level fixtures run.
"""

import pytest
from PyQt6.QtWidgets import QApplication


@pytest.fixture(scope="session", autouse=True)
def _ensure_qapp_with_args():
    """Create QApplication with argv if not yet created.

    Session-scoped and autouse so it runs before any module-scoped qapp fixtures.
    """
    app = QApplication.instance()
    if app is None:
        app = QApplication(["test"])
    return app
