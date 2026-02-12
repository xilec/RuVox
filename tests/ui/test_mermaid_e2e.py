"""E2E test for Mermaid rendering pipeline.

Tests the full cycle:
  Markdown with ```mermaid block → MermaidRenderer renders SVG via QWebEngineView
  → SVG cached → TextViewerWidget shows image via loadResource() → preview dialog works.
"""

import pytest
from unittest.mock import patch, MagicMock
from pathlib import Path

from PyQt6.QtCore import QUrl, QTimer, QEventLoop
from PyQt6.QtWidgets import QApplication

try:
    from PyQt6.QtWebEngineWidgets import QWebEngineView  # noqa: F401
    HAS_WEBENGINE = True
except ImportError:
    HAS_WEBENGINE = False

pytestmark = pytest.mark.skipif(
    not HAS_WEBENGINE,
    reason="PyQt6-WebEngine not available",
)


MERMAID_CODE = "graph TD\n  A[Start] --> B[End]"

MARKDOWN_WITH_MERMAID = f"""# Test Document

Some text before diagram.

```mermaid
{MERMAID_CODE}
```

Some text after diagram.
"""


@pytest.fixture(scope="module")
def qapp():
    """Create QApplication instance for tests.

    QWebEngineView (Chromium) requires a non-empty argv with program name.
    """
    app = QApplication.instance()
    if app is None:
        app = QApplication(["test"])
    yield app


def _wait_for_signal(signal, timeout_ms=10000):
    """Block until signal emits or timeout.  Returns True if signal fired."""
    loop = QEventLoop()
    fired = [False]

    def _on_signal(*args):
        fired[0] = True
        loop.quit()

    signal.connect(_on_signal)
    QTimer.singleShot(timeout_ms, loop.quit)
    loop.exec()
    return fired[0]


class TestMermaidRendererE2E:
    """E2E: MermaidRenderer downloads mermaid.js and renders SVG."""

    def test_render_produces_svg(self, qapp):
        """Full render cycle: code → SVG string in cache."""
        from fast_tts_rus.ui.services.mermaid_renderer import MermaidRenderer

        renderer = MermaidRenderer()
        try:
            renderer.render(MERMAID_CODE)

            # Wait for svg_ready signal (mermaid.js download + render)
            ok = _wait_for_signal(renderer.svg_ready, timeout_ms=30000)
            assert ok, "svg_ready signal not emitted within 30s"

            svg = renderer.get_cached_svg(MERMAID_CODE)
            assert svg is not None, "SVG not cached after rendering"
            assert "<svg" in svg, f"Result doesn't look like SVG: {svg[:100]}"
            assert "</svg>" in svg

            # Pixmap conversion should work
            pixmap = renderer.get_cached_pixmap(MERMAID_CODE, 600)
            assert pixmap is not None, "Failed to convert SVG to pixmap"
            assert pixmap.width() == 600
            assert pixmap.height() > 0
        finally:
            renderer.cleanup()
            renderer.deleteLater()

    def test_mermaid_js_downloaded(self, qapp):
        """mermaid.min.js should be downloaded and cached on disk."""
        from fast_tts_rus.ui.services.mermaid_renderer import MermaidRenderer, _mermaid_cache_dir

        renderer = MermaidRenderer()
        try:
            renderer.render(MERMAID_CODE)
            _wait_for_signal(renderer.svg_ready, timeout_ms=30000)

            js_path = renderer.mermaid_js_path()
            assert js_path is not None
            assert js_path.exists()
            assert js_path.stat().st_size > 10000  # mermaid.min.js is large
        finally:
            renderer.cleanup()
            renderer.deleteLater()


class TestTextViewerMermaidE2E:
    """E2E: TextViewerWidget shows mermaid diagram as image after rendering."""

    def test_image_appears_after_render(self, qapp):
        """Markdown with mermaid block should show image, not placeholder."""
        from fast_tts_rus.ui.widgets.text_viewer import TextViewerWidget, TextFormat
        from fast_tts_rus.ui.models.entry import TextEntry

        viewer = TextViewerWidget()
        try:
            viewer.set_format(TextFormat.MARKDOWN)

            entry = TextEntry(original_text=MARKDOWN_WITH_MERMAID)
            viewer.set_entry(entry)

            renderer = viewer._mermaid_renderer
            assert renderer is not None, "Renderer should be initialized"

            # Wait for SVG render to complete
            ok = _wait_for_signal(renderer.svg_ready, timeout_ms=30000)
            assert ok, "svg_ready signal not emitted"

            # After svg_ready, _on_mermaid_ready re-renders text
            # Process events to let re-render happen
            qapp.processEvents()

            # Now check the HTML contains the image tag (not placeholder)
            html = viewer.toHtml()
            assert "mermaid-diagram" in html or 'src="mermaid-img:0"' in html, (
                f"Image tag not found in HTML after render. "
                f"HTML excerpt: {html[:500]}"
            )

            # loadResource should return a pixmap now
            from PyQt6.QtGui import QTextDocument
            result = viewer.loadResource(
                QTextDocument.ResourceType.ImageResource.value,
                QUrl("mermaid-img:0"),
            )
            assert result is not None, (
                "loadResource returned None — SVG not cached or pixmap conversion failed"
            )
        finally:
            if viewer._mermaid_renderer:
                viewer._mermaid_renderer.cleanup()
            viewer.deleteLater()

    def test_highlighting_works_with_mermaid(self, qapp):
        """Word highlighting should work for text around mermaid blocks."""
        from fast_tts_rus.ui.widgets.text_viewer import TextViewerWidget, TextFormat
        from fast_tts_rus.ui.models.entry import TextEntry

        viewer = TextViewerWidget()
        try:
            text = MARKDOWN_WITH_MERMAID

            # Find positions of words in original text
            before_start = text.find("Some text before")
            after_start = text.find("Some text after")

            timestamps = [
                {
                    "word": "Some", "start": 0.0, "end": 0.5,
                    "original_pos": [before_start, before_start + 4],
                },
                {
                    "word": "after", "start": 1.0, "end": 1.5,
                    "original_pos": [after_start + 10, after_start + 15],
                },
            ]

            viewer.set_format(TextFormat.MARKDOWN)

            # Mock _start_mermaid_rendering to avoid async wait
            with patch.object(viewer, "_start_mermaid_rendering"):
                viewer.set_entry(TextEntry(original_text=text), timestamps)

            # Highlight first word
            viewer.highlight_at_position(0.2)
            assert viewer.extraSelections(), "First 'Some' not highlighted"

            # Highlight word after mermaid block
            viewer.highlight_at_position(1.2)
            assert viewer.extraSelections(), "'after' not highlighted"
        finally:
            if viewer._mermaid_renderer:
                viewer._mermaid_renderer.cleanup()
            viewer.deleteLater()


class TestMermaidPreviewE2E:
    """E2E: MermaidPreviewDialog renders diagram interactively."""

    def test_preview_loads_diagram(self, qapp):
        """Preview dialog should load mermaid diagram without errors."""
        from fast_tts_rus.ui.dialogs.mermaid_preview import MermaidPreviewDialog
        from fast_tts_rus.ui.services.mermaid_renderer import _mermaid_cache_dir

        js_path = _mermaid_cache_dir() / "mermaid.min.js"
        if not js_path.exists():
            pytest.skip("mermaid.min.js not cached (run renderer test first)")

        dialog = MermaidPreviewDialog(js_path)
        try:
            # Capture what setHtml receives
            calls = []
            orig_setHtml = dialog._web_view.setHtml

            def capture(html, base_url=None):
                calls.append((html, base_url))
                orig_setHtml(html, base_url)

            dialog._web_view.setHtml = capture

            with patch.object(dialog, "exec", return_value=0):
                dialog.show_diagram(MERMAID_CODE, title="E2E Test")

            assert len(calls) == 1
            html, base_url = calls[0]

            # HTML should contain the mermaid code
            assert "Start" in html
            assert "End" in html

            # Base URL must be set
            assert base_url is not None, "Base URL not passed to setHtml"
            assert base_url.toLocalFile().endswith("/")

            # Script should use relative path
            assert 'src="mermaid.min.js"' in html
        finally:
            dialog.deleteLater()

    def test_escape_key_closes(self, qapp):
        """Escape key should close the preview dialog."""
        from fast_tts_rus.ui.dialogs.mermaid_preview import MermaidPreviewDialog
        from fast_tts_rus.ui.services.mermaid_renderer import _mermaid_cache_dir
        from PyQt6.QtGui import QKeyEvent
        from PyQt6.QtCore import QEvent, Qt

        js_path = _mermaid_cache_dir() / "mermaid.min.js"
        if not js_path.exists():
            pytest.skip("mermaid.min.js not cached")

        dialog = MermaidPreviewDialog(js_path)
        try:
            with patch.object(dialog, "accept") as mock_accept:
                event = QKeyEvent(
                    QEvent.Type.KeyPress,
                    Qt.Key.Key_Escape,
                    Qt.KeyboardModifier.NoModifier,
                )
                dialog.keyPressEvent(event)
                mock_accept.assert_called_once()
        finally:
            dialog.deleteLater()
