"""Shared fixtures for UI tests.

QWebEngineView (Chromium) requires a non-empty argv in QApplication.
This conftest ensures QApplication is always created with proper args
before any module-level fixtures run.
"""

from pathlib import Path

import numpy as np
import pytest
from PyQt6.QtWidgets import QApplication

from ruvox.ui.models.config import UIConfig
from ruvox.ui.services.storage import StorageService


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


@pytest.fixture
def temp_cache_dir(tmp_path: Path) -> Path:
    """Create a temporary cache directory."""
    cache_dir = tmp_path / "cache"
    cache_dir.mkdir(parents=True, exist_ok=True)
    return cache_dir


@pytest.fixture
def config(temp_cache_dir: Path) -> UIConfig:
    """Create UIConfig with temporary cache directory."""
    return UIConfig(cache_dir=temp_cache_dir)


@pytest.fixture
def storage(config: UIConfig) -> StorageService:
    """Create StorageService instance with temp directory."""
    return StorageService(config)


@pytest.fixture
def mock_audio_data() -> np.ndarray:
    """Create mock audio data (1 second of silence at 48kHz)."""
    sample_rate = 48000
    duration = 1.0
    return np.zeros(int(sample_rate * duration), dtype=np.float32)
