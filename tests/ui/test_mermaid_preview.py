"""Tests for MermaidPreviewDialog."""

import pytest
from unittest.mock import patch, MagicMock
from pathlib import Path

from PyQt6.QtWidgets import QApplication

try:
    from PyQt6.QtWebEngineWidgets import QWebEngineView  # noqa: F401
    HAS_WEBENGINE = True
except ImportError:
    HAS_WEBENGINE = False

pytestmark = pytest.mark.skipif(
    not HAS_WEBENGINE,
    reason="PyQt6-WebEngine not available (missing system libraries)",
)


@pytest.fixture(scope="module")
def qapp():
    """Create QApplication instance for tests."""
    app = QApplication.instance()
    if app is None:
        app = QApplication([])
    yield app


class TestMermaidPreviewDialog:
    """Test MermaidPreviewDialog creation and content."""

    def test_dialog_creates_without_error(self, qapp, tmp_path):
        """Dialog should instantiate without crashing."""
        from fast_tts_rus.ui.dialogs.mermaid_preview import MermaidPreviewDialog

        js_path = tmp_path / "mermaid.min.js"
        js_path.write_text("// mock")

        dialog = MermaidPreviewDialog(js_path)
        assert dialog.windowTitle() == "Mermaid Diagram"
        assert dialog.isModal()
        dialog.deleteLater()

    def test_dialog_has_web_view(self, qapp, tmp_path):
        """Dialog should contain a QWebEngineView."""
        from fast_tts_rus.ui.dialogs.mermaid_preview import MermaidPreviewDialog

        js_path = tmp_path / "mermaid.min.js"
        js_path.write_text("// mock")

        dialog = MermaidPreviewDialog(js_path)
        assert dialog._web_view is not None
        assert isinstance(dialog._web_view, QWebEngineView)
        dialog.deleteLater()

    def test_show_diagram_sets_title(self, qapp, tmp_path):
        """show_diagram should set custom window title."""
        from fast_tts_rus.ui.dialogs.mermaid_preview import MermaidPreviewDialog

        js_path = tmp_path / "mermaid.min.js"
        js_path.write_text("// mock")

        dialog = MermaidPreviewDialog(js_path)

        # Patch exec() to prevent blocking
        with patch.object(dialog, "exec", return_value=0):
            dialog.show_diagram("graph TD\n  A --> B", title="Test")

        assert "Test" in dialog.windowTitle()
        dialog.deleteLater()

    def test_html_contains_mermaid_code(self, qapp, tmp_path):
        """HTML loaded into web view should contain the mermaid code."""
        from fast_tts_rus.ui.dialogs.mermaid_preview import MermaidPreviewDialog

        js_path = tmp_path / "mermaid.min.js"
        js_path.write_text("// mock")

        dialog = MermaidPreviewDialog(js_path)

        loaded_html = []
        original_setHtml = dialog._web_view.setHtml

        def capture_html(html, *args, **kwargs):
            loaded_html.append(html)

        dialog._web_view.setHtml = capture_html

        with patch.object(dialog, "exec", return_value=0):
            dialog.show_diagram("graph TD\n  A --> B")

        assert len(loaded_html) == 1
        assert "graph TD" in loaded_html[0]
        dialog.deleteLater()

    def test_zoom_level_changes(self, qapp, tmp_path):
        """Zoom methods should change zoom level."""
        from fast_tts_rus.ui.dialogs.mermaid_preview import MermaidPreviewDialog

        js_path = tmp_path / "mermaid.min.js"
        js_path.write_text("// mock")

        dialog = MermaidPreviewDialog(js_path)
        assert dialog._zoom_level == 1.0

        # Patch runJavaScript to avoid errors
        dialog._web_view.page().runJavaScript = MagicMock()

        dialog._zoom_in()
        assert dialog._zoom_level == 1.25

        dialog._zoom_out()
        assert dialog._zoom_level == 1.0

        dialog._zoom_reset()
        assert dialog._zoom_level == 1.0

        dialog.deleteLater()

    def test_theme_toggle(self, qapp, tmp_path):
        """Theme toggle should switch between light and dark."""
        from fast_tts_rus.ui.dialogs.mermaid_preview import MermaidPreviewDialog

        js_path = tmp_path / "mermaid.min.js"
        js_path.write_text("// mock")

        dialog = MermaidPreviewDialog(js_path)
        assert dialog._dark_theme is False

        dialog._toggle_theme()
        assert dialog._dark_theme is True
        assert "Светлая" in dialog._btn_theme.text()

        dialog._toggle_theme()
        assert dialog._dark_theme is False
        assert "Тёмная" in dialog._btn_theme.text()

        dialog.deleteLater()
