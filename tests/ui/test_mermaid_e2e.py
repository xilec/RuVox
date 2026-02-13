"""E2E test for Mermaid rendering pipeline.

Tests the full cycle:
  Markdown with ```mermaid block → MermaidRenderer renders SVG via QWebEngineView
  → SVG cached → pixmap captured → TextViewerWidget shows image → preview dialog works.
"""

import pytest
from unittest.mock import patch, MagicMock
from pathlib import Path

from PyQt6.QtCore import QUrl, QTimer, QEventLoop, Qt
from PyQt6.QtWidgets import QApplication

SCREENSHOT_DIR = Path(__file__).resolve().parent.parent.parent / "tmp"

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

COMPLEX_MERMAID = """\
flowchart TB
    subgraph Input["Входная обработка"]
        A[Входная последовательность] --> B[Token Embedding]
        B --> C[Positional Encoding]
    end

    subgraph Encoder["Encoder блоки (×N)"]
        C --> D[Multi-Head Self-Attention]
        D --> E[Add & Norm]
        E --> F[Feed Forward Network]
        F --> G[Add & Norm]
        G --> H{Больше блоков?}
        H -->|Да| D
        H -->|Нет| I[Выход Encoder]
    end

    subgraph Decoder["Decoder блоки (×N)"]
        J[Вход Decoder] --> K[Masked Multi-Head Self-Attention]
        K --> L[Add & Norm]
        I --> M[Multi-Head Cross-Attention]
        L --> M
        M --> N[Add & Norm]
        N --> O[Feed Forward Network]
        O --> P[Add & Norm]
        P --> Q{Больше блоков?}
        Q -->|Да| K
        Q -->|Нет| R[Выход Decoder]
    end

    subgraph Output["Выходная генерация"]
        R --> S[Linear Projection]
        S --> T[Softmax]
        T --> U[Распределение вероятностей]
    end

    style Input fill:#e1f5fe
    style Encoder fill:#fff3e0
    style Decoder fill:#f3e5f5
    style Output fill:#e8f5e9"""

MARKDOWN_WITH_MERMAID = f"""# Test Document

Some text before diagram.

```mermaid
{MERMAID_CODE}
```

Some text after diagram.
"""

MARKDOWN_WITH_COMPLEX = f"""# Transformer Architecture

```mermaid
{COMPLEX_MERMAID}
```

End of document.
"""



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


def _process_events_for(ms: int) -> None:
    """Process Qt events for given milliseconds."""
    loop = QEventLoop()
    QTimer.singleShot(ms, loop.quit)
    loop.exec()


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


class TestMermaidRenderQuality:
    """Test that mermaid diagrams render with proper visuals."""

    def test_simple_diagram_light_background(self, qapp):
        """Simple diagram pixmap should have a white background.

        Screenshot: tmp/mermaid_quality_simple.png
        """
        from fast_tts_rus.ui.services.mermaid_renderer import MermaidRenderer

        SCREENSHOT_DIR.mkdir(parents=True, exist_ok=True)
        renderer = MermaidRenderer()
        try:
            renderer.render(MERMAID_CODE)
            ok = _wait_for_signal(renderer.svg_ready, timeout_ms=30000)
            assert ok, "svg_ready not emitted"

            pixmap = renderer.get_cached_pixmap(MERMAID_CODE, 400)
            assert pixmap is not None
            pixmap.save(str(SCREENSHOT_DIR / "mermaid_quality_simple.png"))

            # Check corners are white (background)
            image = pixmap.toImage()
            corner = image.pixelColor(0, 0)
            assert corner.red() > 240, f"Background not white: R={corner.red()}"
            assert corner.green() > 240, f"Background not white: G={corner.green()}"
            assert corner.blue() > 240, f"Background not white: B={corner.blue()}"
        finally:
            renderer.cleanup()
            renderer.deleteLater()

    def test_simple_diagram_has_visible_content(self, qapp):
        """Pixmap should contain non-white content (the actual diagram).

        Screenshot: tmp/mermaid_quality_content.png
        """
        from fast_tts_rus.ui.services.mermaid_renderer import MermaidRenderer

        SCREENSHOT_DIR.mkdir(parents=True, exist_ok=True)
        renderer = MermaidRenderer()
        try:
            renderer.render(MERMAID_CODE)
            ok = _wait_for_signal(renderer.svg_ready, timeout_ms=30000)
            assert ok, "svg_ready not emitted"

            pixmap = renderer.get_cached_pixmap(MERMAID_CODE, 600)
            assert pixmap is not None
            pixmap.save(str(SCREENSHOT_DIR / "mermaid_quality_content.png"))

            # Scan for non-white pixels (diagram shapes/text)
            image = pixmap.toImage()
            has_color = False
            for x in range(0, image.width(), 5):
                for y in range(0, image.height(), 5):
                    c = image.pixelColor(x, y)
                    if c.red() < 240 or c.green() < 240 or c.blue() < 240:
                        has_color = True
                        break
                if has_color:
                    break
            assert has_color, "Pixmap is all white — diagram not rendered"
        finally:
            renderer.cleanup()
            renderer.deleteLater()

    def test_simple_diagram_no_dark_blocks(self, qapp):
        """Diagram nodes should NOT have black/dark backgrounds.

        Screenshot: tmp/mermaid_quality_no_dark.png
        """
        from fast_tts_rus.ui.services.mermaid_renderer import MermaidRenderer

        SCREENSHOT_DIR.mkdir(parents=True, exist_ok=True)
        renderer = MermaidRenderer()
        try:
            renderer.render(MERMAID_CODE)
            ok = _wait_for_signal(renderer.svg_ready, timeout_ms=30000)
            assert ok

            pixmap = renderer.get_cached_pixmap(MERMAID_CODE, 600)
            assert pixmap is not None
            pixmap.save(str(SCREENSHOT_DIR / "mermaid_quality_no_dark.png"))

            # Count very dark pixels — should be minimal (only text/arrows)
            image = pixmap.toImage()
            dark_count = 0
            total = 0
            for x in range(0, image.width(), 3):
                for y in range(0, image.height(), 3):
                    total += 1
                    c = image.pixelColor(x, y)
                    if c.red() < 30 and c.green() < 30 and c.blue() < 30:
                        dark_count += 1
            dark_ratio = dark_count / total if total > 0 else 0
            assert dark_ratio < 0.15, (
                f"Too many dark pixels ({dark_ratio:.1%}) — "
                f"nodes have black backgrounds?"
            )
        finally:
            renderer.cleanup()
            renderer.deleteLater()

    def test_complex_diagram_renders(self, qapp):
        """Complex transformer diagram should render completely.

        Screenshot: tmp/mermaid_quality_complex.png
        """
        from fast_tts_rus.ui.services.mermaid_renderer import MermaidRenderer

        SCREENSHOT_DIR.mkdir(parents=True, exist_ok=True)
        renderer = MermaidRenderer()
        try:
            renderer.render(COMPLEX_MERMAID)
            ok = _wait_for_signal(renderer.svg_ready, timeout_ms=30000)
            assert ok, "svg_ready not emitted for complex diagram"

            svg = renderer.get_cached_svg(COMPLEX_MERMAID)
            assert svg is not None
            assert "<svg" in svg

            pixmap = renderer.get_cached_pixmap(COMPLEX_MERMAID, 800)
            assert pixmap is not None
            assert pixmap.height() > 200, "Complex diagram too short"
            pixmap.save(str(SCREENSHOT_DIR / "mermaid_quality_complex.png"))

            # SVG should contain text from the diagram
            assert "Encoder" in svg or "encoder" in svg.lower(), (
                "SVG missing 'Encoder' text"
            )

            # Check that pixmap has visible content
            image = pixmap.toImage()
            has_color = False
            for x in range(0, image.width(), 10):
                for y in range(0, image.height(), 10):
                    c = image.pixelColor(x, y)
                    if c.red() < 240 or c.green() < 240 or c.blue() < 240:
                        has_color = True
                        break
                if has_color:
                    break
            assert has_color, "Complex diagram pixmap is all white"

            # Check no excessive dark areas
            dark_count = 0
            total = 0
            for x in range(0, image.width(), 5):
                for y in range(0, image.height(), 5):
                    total += 1
                    c = image.pixelColor(x, y)
                    if c.red() < 30 and c.green() < 30 and c.blue() < 30:
                        dark_count += 1
            dark_ratio = dark_count / total if total > 0 else 0
            assert dark_ratio < 0.15, (
                f"Complex diagram has {dark_ratio:.1%} dark pixels — "
                f"dark theme or missing content?"
            )
        finally:
            renderer.cleanup()
            renderer.deleteLater()

    def test_complex_diagram_has_subgraph_colors(self, qapp):
        """Complex diagram with styled subgraphs should have colored areas.

        Screenshot: tmp/mermaid_quality_colors.png
        """
        from fast_tts_rus.ui.services.mermaid_renderer import MermaidRenderer

        SCREENSHOT_DIR.mkdir(parents=True, exist_ok=True)
        renderer = MermaidRenderer()
        try:
            renderer.render(COMPLEX_MERMAID)
            ok = _wait_for_signal(renderer.svg_ready, timeout_ms=30000)
            assert ok

            pixmap = renderer.get_cached_pixmap(COMPLEX_MERMAID, 800)
            assert pixmap is not None
            pixmap.save(str(SCREENSHOT_DIR / "mermaid_quality_colors.png"))

            # Check for colored pixels (subgraph backgrounds are pastel colors)
            image = pixmap.toImage()
            has_blue_tint = False  # #e1f5fe (Input)
            has_orange_tint = False  # #fff3e0 (Encoder)
            has_purple_tint = False  # #f3e5f5 (Decoder)
            has_green_tint = False  # #e8f5e9 (Output)

            for x in range(0, image.width(), 5):
                for y in range(0, image.height(), 5):
                    c = image.pixelColor(x, y)
                    r, g, b = c.red(), c.green(), c.blue()
                    # Light blue: R<240, G>240, B>250
                    if r < 235 and g > 240 and b > 250:
                        has_blue_tint = True
                    # Light orange: R>250, G>240, B<235
                    if r > 250 and g > 240 and b < 235:
                        has_orange_tint = True
                    # Light purple: R>240, G<240, B>240
                    if r > 240 and g < 235 and b > 240:
                        has_purple_tint = True
                    # Light green: R<240, G>245, B<240
                    if r < 240 and g > 245 and b < 240:
                        has_green_tint = True

            colored_count = sum([has_blue_tint, has_orange_tint, has_purple_tint, has_green_tint])
            assert colored_count >= 2, (
                f"Expected subgraph colors but only found {colored_count}/4: "
                f"blue={has_blue_tint}, orange={has_orange_tint}, "
                f"purple={has_purple_tint}, green={has_green_tint}"
            )
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


class TestMermaidVisualE2E:
    """Visual E2E tests with screenshots saved to tmp/."""

    def test_viewer_renders_mermaid_image(self, qapp):
        """TextViewer should render mermaid block as an image, not placeholder.

        Screenshots:
          - tmp/mermaid_01_initial.png — right after set_entry
          - tmp/mermaid_02_rendered.png — after SVG ready, image visible
        """
        from fast_tts_rus.ui.widgets.text_viewer import TextViewerWidget, TextFormat
        from fast_tts_rus.ui.models.entry import TextEntry
        from fast_tts_rus.ui.services.mermaid_renderer import _code_hash

        SCREENSHOT_DIR.mkdir(parents=True, exist_ok=True)

        viewer = TextViewerWidget()
        viewer.resize(800, 600)
        viewer.show()
        try:
            viewer.set_format(TextFormat.MARKDOWN)
            entry = TextEntry(original_text=MARKDOWN_WITH_MERMAID)
            viewer.set_entry(entry)

            # Screenshot 1: initial state
            _process_events_for(200)
            pixmap1 = viewer.grab()
            pixmap1.save(str(SCREENSHOT_DIR / "mermaid_01_initial.png"))

            renderer = viewer._mermaid_renderer
            assert renderer is not None, "Renderer not initialized"

            # Wait for full rendering pipeline: SVG render + pixmap capture.
            # _pixmap_cache is set at the same time as svg_ready is emitted,
            # so check it to avoid waiting for already-emitted signal.
            h = _code_hash(MERMAID_CODE)
            if h not in renderer._pixmap_cache:
                ok = _wait_for_signal(renderer.svg_ready, timeout_ms=30000)
                assert ok, "svg_ready not emitted"
            # Wait for _on_mermaid_ready to re-render text
            _process_events_for(500)

            # Screenshot 2: with rendered image
            _process_events_for(200)
            pixmap2 = viewer.grab()
            pixmap2.save(str(SCREENSHOT_DIR / "mermaid_02_rendered.png"))

            # Verify image is in the document
            html = viewer.toHtml()
            assert 'src="mermaid-img:0"' in html, (
                "Image tag not in HTML after render — still showing placeholder"
            )
            assert "загружается" not in html, (
                "Placeholder text still present after render"
            )

            # loadResource should return a pixmap
            from PyQt6.QtGui import QTextDocument
            resource = viewer.loadResource(
                QTextDocument.ResourceType.ImageResource.value,
                QUrl("mermaid-img:0"),
            )
            assert resource is not None, "loadResource returned None"
        finally:
            if viewer._mermaid_renderer:
                viewer._mermaid_renderer.cleanup()
            viewer.deleteLater()
            _process_events_for(100)  # Allow cleanup before next test

    def test_viewer_renders_complex_diagram(self, qapp):
        """TextViewer should render complex transformer diagram as image.

        Screenshots:
          - tmp/mermaid_04_complex_viewer.png — complex diagram in TextViewer
        """
        from fast_tts_rus.ui.widgets.text_viewer import TextViewerWidget, TextFormat
        from fast_tts_rus.ui.models.entry import TextEntry
        from fast_tts_rus.ui.services.mermaid_renderer import _code_hash

        SCREENSHOT_DIR.mkdir(parents=True, exist_ok=True)

        viewer = TextViewerWidget()
        viewer.resize(900, 700)
        viewer.show()
        try:
            viewer.set_format(TextFormat.MARKDOWN)
            entry = TextEntry(original_text=MARKDOWN_WITH_COMPLEX)
            viewer.set_entry(entry)

            renderer = viewer._mermaid_renderer
            assert renderer is not None, "Renderer not initialized"

            h = _code_hash(COMPLEX_MERMAID)
            if h not in renderer._pixmap_cache:
                ok = _wait_for_signal(renderer.svg_ready, timeout_ms=30000)
                assert ok, "svg_ready not emitted for complex diagram"
            _process_events_for(500)

            _process_events_for(200)
            pixmap = viewer.grab()
            pixmap.save(str(SCREENSHOT_DIR / "mermaid_04_complex_viewer.png"))

            html = viewer.toHtml()
            assert 'src="mermaid-img:0"' in html, "Complex diagram image not in HTML"
        finally:
            if viewer._mermaid_renderer:
                viewer._mermaid_renderer.cleanup()
            viewer.deleteLater()
            _process_events_for(100)

    def test_preview_dialog_renders_diagram(self, qapp):
        """Preview dialog should render interactive mermaid diagram.

        Screenshots:
          - tmp/mermaid_03_preview.png — preview dialog with rendered diagram
        """
        from fast_tts_rus.ui.dialogs.mermaid_preview import MermaidPreviewDialog
        from fast_tts_rus.ui.services.mermaid_renderer import _mermaid_cache_dir

        SCREENSHOT_DIR.mkdir(parents=True, exist_ok=True)

        js_path = _mermaid_cache_dir() / "mermaid.min.js"
        if not js_path.exists():
            pytest.skip("mermaid.min.js not cached")

        dialog = MermaidPreviewDialog(js_path)
        dialog.resize(800, 600)
        try:
            # Prepare HTML but don't call exec() (would block)
            dialog._current_code = MERMAID_CODE
            html = dialog._build_html(MERMAID_CODE)
            base_url = QUrl.fromLocalFile(str(js_path.parent) + "/")
            dialog._web_view.setHtml(html, base_url)
            dialog.setWindowTitle("Mermaid — E2E Test")
            dialog.show()

            # Wait for WebEngine to render mermaid diagram
            _process_events_for(3000)

            # Screenshot 3: preview dialog
            pixmap = dialog.grab()
            pixmap.save(str(SCREENSHOT_DIR / "mermaid_03_preview.png"))

            # Verify dialog state
            assert dialog.windowTitle() == "Mermaid — E2E Test"
            assert dialog._zoom_level == 1.0
        finally:
            dialog.close()
            dialog.deleteLater()

    def test_preview_dialog_complex_diagram(self, qapp):
        """Preview dialog should render complex transformer diagram.

        Screenshots:
          - tmp/mermaid_05_complex_preview.png — complex diagram in preview
        """
        from fast_tts_rus.ui.dialogs.mermaid_preview import MermaidPreviewDialog
        from fast_tts_rus.ui.services.mermaid_renderer import _mermaid_cache_dir

        SCREENSHOT_DIR.mkdir(parents=True, exist_ok=True)

        js_path = _mermaid_cache_dir() / "mermaid.min.js"
        if not js_path.exists():
            pytest.skip("mermaid.min.js not cached")

        dialog = MermaidPreviewDialog(js_path)
        dialog.resize(1000, 800)
        try:
            dialog._current_code = COMPLEX_MERMAID
            html = dialog._build_html(COMPLEX_MERMAID)
            base_url = QUrl.fromLocalFile(str(js_path.parent) + "/")
            dialog._web_view.setHtml(html, base_url)
            dialog.setWindowTitle("Mermaid — Transformer Architecture")
            dialog.show()

            _process_events_for(5000)

            pixmap = dialog.grab()
            pixmap.save(str(SCREENSHOT_DIR / "mermaid_05_complex_preview.png"))

            assert dialog.windowTitle() == "Mermaid — Transformer Architecture"
        finally:
            dialog.close()
            dialog.deleteLater()


class TestTextViewerLinkNavigation:
    """Test that clicking mermaid links doesn't break TextViewer state."""

    def test_mermaid_click_preserves_content(self, qapp):
        """Clicking mermaid link should not clear TextViewer content."""
        from fast_tts_rus.ui.widgets.text_viewer import TextViewerWidget, TextFormat
        from fast_tts_rus.ui.models.entry import TextEntry

        viewer = TextViewerWidget()
        try:
            viewer.set_format(TextFormat.MARKDOWN)

            with patch.object(viewer, "_start_mermaid_rendering"):
                viewer.set_entry(TextEntry(original_text=MARKDOWN_WITH_MERMAID))

            html_before = viewer.toHtml()
            assert len(html_before) > 100, "No content set"

            # Simulate clicking mermaid link (with preview mocked)
            with patch.object(viewer, "_show_mermaid_preview"):
                viewer._on_anchor_clicked(QUrl("mermaid:0"))

            # Content should be preserved
            html_after = viewer.toHtml()
            assert len(html_after) > 100, (
                "Content lost after clicking mermaid link"
            )
            assert "Test Document" in viewer.toPlainText(), (
                "Title text missing after clicking mermaid link"
            )
        finally:
            if viewer._mermaid_renderer:
                viewer._mermaid_renderer.cleanup()
            viewer.deleteLater()

    def test_switch_to_plain_after_click(self, qapp):
        """Switching to plain text after mermaid click should show raw text."""
        from fast_tts_rus.ui.widgets.text_viewer import TextViewerWidget, TextFormat
        from fast_tts_rus.ui.models.entry import TextEntry

        viewer = TextViewerWidget()
        try:
            viewer.set_format(TextFormat.MARKDOWN)

            with patch.object(viewer, "_start_mermaid_rendering"):
                viewer.set_entry(TextEntry(original_text=MARKDOWN_WITH_MERMAID))

            # Click mermaid link
            with patch.object(viewer, "_show_mermaid_preview"):
                viewer._on_anchor_clicked(QUrl("mermaid:0"))

            # Switch to plain text
            viewer.set_format(TextFormat.PLAIN)
            plain = viewer.toPlainText()
            assert "```mermaid" in plain, (
                f"Plain text mode doesn't show raw mermaid block: {plain[:200]}"
            )
            assert "Some text before" in plain
            assert "Some text after" in plain
        finally:
            if viewer._mermaid_renderer:
                viewer._mermaid_renderer.cleanup()
            viewer.deleteLater()

    def test_plain_text_has_no_link_formatting(self, qapp):
        """Plain text after markdown should not have link character format."""
        from fast_tts_rus.ui.widgets.text_viewer import TextViewerWidget, TextFormat
        from fast_tts_rus.ui.models.entry import TextEntry
        from PyQt6.QtGui import QTextCursor

        viewer = TextViewerWidget()
        try:
            # Set markdown with mermaid (which injects <a> tags)
            viewer.set_format(TextFormat.MARKDOWN)
            with patch.object(viewer, "_start_mermaid_rendering"):
                viewer.set_entry(TextEntry(original_text=MARKDOWN_WITH_MERMAID))

            # Switch to plain text
            viewer.set_format(TextFormat.PLAIN)

            # Check that no text has anchor formatting
            cursor = QTextCursor(viewer.document())
            cursor.movePosition(QTextCursor.MoveOperation.Start)
            cursor.movePosition(
                QTextCursor.MoveOperation.End,
                QTextCursor.MoveMode.KeepAnchor,
            )
            char_format = cursor.charFormat()
            assert not char_format.isAnchor(), (
                "Plain text has anchor formatting — link styles leaked from HTML"
            )

            # Default stylesheet should be cleared
            assert viewer.document().defaultStyleSheet() == ""
        finally:
            if viewer._mermaid_renderer:
                viewer._mermaid_renderer.cleanup()
            viewer.deleteLater()

    def test_switch_entry_after_mermaid_clears_formatting(self, qapp):
        """Switching to a new entry should not carry over link formatting."""
        from fast_tts_rus.ui.widgets.text_viewer import TextViewerWidget, TextFormat
        from fast_tts_rus.ui.models.entry import TextEntry
        from PyQt6.QtGui import QTextCursor

        viewer = TextViewerWidget()
        try:
            # Set markdown with mermaid
            viewer.set_format(TextFormat.MARKDOWN)
            with patch.object(viewer, "_start_mermaid_rendering"):
                viewer.set_entry(TextEntry(original_text=MARKDOWN_WITH_MERMAID))

            # Switch to plain text mode and set a simple entry
            viewer.set_format(TextFormat.PLAIN)
            viewer.set_entry(TextEntry(original_text="Simple plain text"))

            plain = viewer.toPlainText()
            assert plain == "Simple plain text"

            # Verify no anchor format
            cursor = QTextCursor(viewer.document())
            cursor.movePosition(QTextCursor.MoveOperation.Start)
            cursor.movePosition(
                QTextCursor.MoveOperation.End,
                QTextCursor.MoveMode.KeepAnchor,
            )
            assert not cursor.charFormat().isAnchor()
        finally:
            if viewer._mermaid_renderer:
                viewer._mermaid_renderer.cleanup()
            viewer.deleteLater()
