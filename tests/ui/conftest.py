"""Shared fixtures for UI tests.

QWebEngineView (Chromium) requires a non-empty argv in QApplication.
This conftest ensures QApplication is always created with proper args
before any module-level fixtures run.
"""

import pytest
from PyQt6.QtWidgets import QApplication


@pytest.fixture(scope="session", autouse=True)
def qapp():
    """Create or retrieve QApplication instance for all UI tests.

    Session-scoped and autouse: runs before any test and provides
    a QApplication with non-empty argv (required by QWebEngineView/Chromium).
    """
    app = QApplication.instance()
    if app is None:
        app = QApplication(["test"])
    return app
