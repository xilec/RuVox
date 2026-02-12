"""Tests for format selector UI in MainWindow.

These tests use mocks to avoid creating PlayerWidget (which requires mpv).
They verify UI structure and component interactions (contracts).
"""

import pytest
from unittest.mock import Mock, patch, MagicMock
from PyQt6.QtWidgets import QApplication

from fast_tts_rus.ui.main_window import MainWindow
from fast_tts_rus.ui.widgets.text_viewer import TextFormat
from fast_tts_rus.ui.models.config import UIConfig


@pytest.fixture(scope="module")
def qapp():
    """Create QApplication instance for tests."""
    app = QApplication.instance()
    if app is None:
        app = QApplication([])
    yield app


@pytest.fixture
def mock_app(tmp_path):
    """Create mock TTSApplication with real config."""
    app = Mock()
    app.config = UIConfig(cache_dir=tmp_path)
    app.config.text_format = "plain"
    # Mock the save method
    app.config.save = Mock()
    app.storage = Mock()
    app.storage.get_all_entries = Mock(return_value=[])
    app.tts_worker = Mock()
    return app


@pytest.fixture
def main_window(qapp, mock_app):
    """Create MainWindow with mocked PlayerWidget.

    PlayerWidget is mocked because it requires mpv which causes segfault in tests.
    We verify UI structure and method calls, not player functionality.
    """
    from PyQt6.QtWidgets import QWidget

    # Create a stub PlayerWidget that is a real QWidget
    class StubPlayerWidget(QWidget):
        """Stub PlayerWidget for testing - avoids mpv initialization."""

        # Mock signals as class attributes
        next_requested = Mock()
        prev_requested = Mock()
        playback_started = Mock()
        playback_stopped = Mock()
        position_changed = Mock()

        def __init__(self, *args, **kwargs):
            super().__init__()
            self.current_entry = None
            self.setMinimumHeight(80)

            # Make signals work like real Qt signals (callable mocks)
            self.next_requested = Mock()
            self.prev_requested = Mock()
            self.playback_started = Mock()
            self.playback_stopped = Mock()
            self.position_changed = Mock()

            # Mock methods
            self.load_entry = Mock(return_value=True)
            self.play = Mock()
            self.stop = Mock()
            self.toggle_play_pause = Mock()
            self.seek_relative = Mock()
            self.speed_up = Mock()
            self.speed_down = Mock()

    with patch('fast_tts_rus.ui.main_window.PlayerWidget', StubPlayerWidget):
        window = MainWindow(mock_app)
        yield window
        window.deleteLater()


class TestFormatSelectorUI:
    """Test format selector UI elements and structure."""

    def test_format_selector_created(self, main_window):
        """FORMAT_SELECTOR widget should be created in MainWindow."""
        assert hasattr(main_window, 'format_selector')
        assert main_window.format_selector is not None

    def test_format_selector_has_two_options(self, main_window):
        """FORMAT_SELECTOR should have exactly 2 options: Plain and Markdown."""
        assert main_window.format_selector.count() == 2

        # Verify labels
        plain_text = main_window.format_selector.itemText(0)
        markdown_text = main_window.format_selector.itemText(1)

        assert "Plain" in plain_text
        assert "Markdown" in markdown_text

    def test_format_selector_data_values(self, main_window):
        """FORMAT_SELECTOR items should have correct data values."""
        plain_data = main_window.format_selector.itemData(0)
        markdown_data = main_window.format_selector.itemData(1)

        assert plain_data == TextFormat.PLAIN.value
        assert markdown_data == TextFormat.MARKDOWN.value


class TestFormatChangeContract:
    """Test contract: changing format_selector should call text_viewer.set_format()."""

    def test_change_to_markdown_calls_set_format(self, main_window):
        """CONTRACT: Changing selector to Markdown should call text_viewer.set_format(MARKDOWN)."""
        # Spy on text_viewer.set_format
        original_set_format = main_window.text_viewer.set_format
        calls = []

        def spy_set_format(fmt):
            calls.append(fmt)
            original_set_format(fmt)

        main_window.text_viewer.set_format = spy_set_format

        # Change to Markdown
        main_window.format_selector.setCurrentIndex(1)

        # Verify set_format was called with correct argument
        assert len(calls) == 1
        assert calls[0] == TextFormat.MARKDOWN

    def test_change_saves_to_config(self, main_window, mock_app):
        """CONTRACT: Changing format should save to config.text_format and call config.save()."""
        # Change to Markdown
        main_window.format_selector.setCurrentIndex(1)

        # Verify config updated
        assert mock_app.config.text_format == TextFormat.MARKDOWN.value

        # Verify save called
        assert mock_app.config.save.called


class TestFormatRestoration:
    """Test restoring saved format on startup."""

    def test_restore_plain_format(self, main_window, mock_app):
        """Should restore plain format from config on load_entries()."""
        mock_app.config.text_format = "plain"

        main_window.load_entries()

        # Verify text_viewer format
        assert main_window.text_viewer.text_format == TextFormat.PLAIN

        # Verify selector position (without triggering signal)
        assert main_window.format_selector.currentData() == TextFormat.PLAIN.value

    def test_restore_markdown_format(self, qapp, mock_app):
        """Should restore markdown format from config on load_entries()."""
        from PyQt6.QtWidgets import QWidget

        class StubPlayerWidget(QWidget):
            """Stub PlayerWidget for testing - avoids mpv initialization."""
            next_requested = Mock()
            prev_requested = Mock()
            playback_started = Mock()
            playback_stopped = Mock()
            position_changed = Mock()

            def __init__(self, *args, **kwargs):
                super().__init__()
                self.current_entry = None
                self.setMinimumHeight(80)
                self.next_requested = Mock()
                self.prev_requested = Mock()
                self.playback_started = Mock()
                self.playback_stopped = Mock()
                self.position_changed = Mock()
                self.load_entry = Mock(return_value=True)
                self.play = Mock()
                self.stop = Mock()
                self.toggle_play_pause = Mock()
                self.seek_relative = Mock()
                self.speed_up = Mock()
                self.speed_down = Mock()

        with patch('fast_tts_rus.ui.main_window.PlayerWidget', StubPlayerWidget):
            mock_app.config.text_format = "markdown"
            mock_app.config.save = Mock()

            window = MainWindow(mock_app)
            window.load_entries()

            # Verify restored
            assert window.text_viewer.text_format == TextFormat.MARKDOWN
            assert window.format_selector.currentData() == TextFormat.MARKDOWN.value

            window.deleteLater()

    def test_invalid_format_fallback(self, qapp, mock_app):
        """Invalid format in config should fallback to plain."""
        from PyQt6.QtWidgets import QWidget

        class StubPlayerWidget(QWidget):
            """Stub PlayerWidget for testing - avoids mpv initialization."""
            next_requested = Mock()
            prev_requested = Mock()
            playback_started = Mock()
            playback_stopped = Mock()
            position_changed = Mock()

            def __init__(self, *args, **kwargs):
                super().__init__()
                self.current_entry = None
                self.setMinimumHeight(80)
                self.next_requested = Mock()
                self.prev_requested = Mock()
                self.playback_started = Mock()
                self.playback_stopped = Mock()
                self.position_changed = Mock()
                self.load_entry = Mock(return_value=True)
                self.play = Mock()
                self.stop = Mock()
                self.toggle_play_pause = Mock()
                self.seek_relative = Mock()
                self.speed_up = Mock()
                self.speed_down = Mock()

        with patch('fast_tts_rus.ui.main_window.PlayerWidget', StubPlayerWidget):
            mock_app.config.text_format = "invalid_format"
            mock_app.config.save = Mock()

            window = MainWindow(mock_app)
            window.load_entries()

            # Should fallback to plain
            assert window.text_viewer.text_format == TextFormat.PLAIN

            window.deleteLater()


class TestFormatPersistence:
    """Test format persistence across sessions (integration)."""

    def test_format_persists_across_sessions(self, qapp, tmp_path):
        """Format should persist when saved and loaded again."""
        from PyQt6.QtWidgets import QWidget

        class StubPlayerWidget(QWidget):
            """Stub PlayerWidget for testing - avoids mpv initialization."""
            next_requested = Mock()
            prev_requested = Mock()
            playback_started = Mock()
            playback_stopped = Mock()
            position_changed = Mock()

            def __init__(self, *args, **kwargs):
                super().__init__()
                self.current_entry = None
                self.setMinimumHeight(80)
                self.next_requested = Mock()
                self.prev_requested = Mock()
                self.playback_started = Mock()
                self.playback_stopped = Mock()
                self.position_changed = Mock()
                self.load_entry = Mock(return_value=True)
                self.play = Mock()
                self.stop = Mock()
                self.toggle_play_pause = Mock()
                self.seek_relative = Mock()
                self.speed_up = Mock()
                self.speed_down = Mock()

        with patch('fast_tts_rus.ui.main_window.PlayerWidget', StubPlayerWidget):
            # Session 1: Create window, change format, save
            config = UIConfig(cache_dir=tmp_path)
            config.text_format = "plain"

            app1 = Mock()
            app1.config = config
            app1.storage = Mock()
            app1.storage.get_all_entries = Mock(return_value=[])
            app1.tts_worker = Mock()

            window1 = MainWindow(app1)
            window1.load_entries()
            window1.format_selector.setCurrentIndex(1)  # Switch to Markdown

            # Verify saved
            assert config.text_format == "markdown"

            window1.deleteLater()

            # Session 2: Load config again
            config2 = UIConfig.load(tmp_path / "config.json")
            assert config2.text_format == "markdown"

            app2 = Mock()
            app2.config = config2
            app2.storage = Mock()
            app2.storage.get_all_entries = Mock(return_value=[])
            app2.tts_worker = Mock()

            window2 = MainWindow(app2)
            window2.load_entries()

            # Should restore Markdown
            assert window2.text_viewer.text_format == TextFormat.MARKDOWN
            assert window2.format_selector.currentData() == TextFormat.MARKDOWN.value

            window2.deleteLater()


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
