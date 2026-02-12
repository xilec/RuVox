"""Mermaid diagram renderer using hidden QWebEngineView.

Renders Mermaid code to SVG via mermaid.js in a hidden web view.
Caches results by code hash. Downloads mermaid.min.js on first use.
"""

import hashlib
import logging
import urllib.request
from pathlib import Path

from PyQt6.QtCore import pyqtSignal, QObject, QUrl
from PyQt6.QtGui import QImage, QPixmap
from PyQt6.QtWidgets import QWidget

logger = logging.getLogger(__name__)

MERMAID_CDN_URL = "https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.min.js"


def _mermaid_cache_dir() -> Path:
    """Return cache directory for mermaid assets."""
    path = Path.home() / ".cache" / "fast_tts_rus" / "mermaid"
    path.mkdir(parents=True, exist_ok=True)
    return path


def _code_hash(code: str) -> str:
    """Return short hash of mermaid code for caching."""
    return hashlib.sha256(code.strip().encode()).hexdigest()[:16]


class MermaidRenderer(QObject):
    """Renders Mermaid diagrams to SVG via hidden QWebEngineView.

    Signals:
        svg_ready(code_hash, svg_string): Emitted when SVG rendering is complete.
    """

    svg_ready = pyqtSignal(str, str)  # (code_hash, svg_string)

    def __init__(self, parent: QWidget | None = None):
        super().__init__(parent)
        self._web_view = None  # lazy init
        self._svg_cache: dict[str, str] = {}  # hash → SVG
        self._pixmap_cache: dict[str, QPixmap] = {}  # hash → captured pixmap
        self._mermaid_js_path: Path | None = None
        self._queue: list[tuple[str, str]] = []  # (hash, code)
        self._rendering = False
        self._js_ready = False
        self._js_downloading = False
        self._pending_callbacks: list[callable] = []

    # -- Public API --

    def get_cached_svg(self, code: str) -> str | None:
        """Return cached SVG for code, or None."""
        h = _code_hash(code)
        return self._svg_cache.get(h)

    def get_cached_pixmap(self, code: str, width: int) -> QPixmap | None:
        """Return cached pixmap scaled to width.

        Prefers captured pixmap from webview (accurate browser rendering).
        Falls back to QSvgRenderer conversion for simple SVGs.
        """
        from PyQt6.QtCore import Qt

        h = _code_hash(code)

        # Prefer captured pixmap (handles CSS, foreignObject, etc.)
        pixmap = self._pixmap_cache.get(h)
        if pixmap and width > 0:
            return pixmap.scaledToWidth(width, Qt.TransformationMode.SmoothTransformation)

        # Fallback to SVG conversion (basic diagrams)
        svg = self._svg_cache.get(h)
        if not svg:
            return None
        return self._svg_to_pixmap(svg, width)

    def render(self, code: str) -> None:
        """Queue code for rendering. Result via svg_ready signal."""
        h = _code_hash(code)
        if h in self._svg_cache:
            self.svg_ready.emit(h, self._svg_cache[h])
            return

        # Avoid duplicates in queue
        if not any(qh == h for qh, _ in self._queue):
            self._queue.append((h, code.strip()))

        self._ensure_mermaid_js(self._process_queue)

    def mermaid_js_path(self) -> Path | None:
        """Return path to mermaid.min.js if downloaded."""
        return self._mermaid_js_path

    # -- mermaid.js download --

    def _ensure_mermaid_js(self, callback) -> None:
        """Download mermaid.min.js if not cached, then call callback."""
        cache_dir = _mermaid_cache_dir()
        js_path = cache_dir / "mermaid.min.js"

        if js_path.exists() and js_path.stat().st_size > 0:
            self._mermaid_js_path = js_path
            self._js_ready = True
            callback()
            return

        if self._js_downloading:
            self._pending_callbacks.append(callback)
            return

        self._js_downloading = True
        self._pending_callbacks.append(callback)

        try:
            logger.info("Downloading mermaid.min.js from CDN...")
            urllib.request.urlretrieve(MERMAID_CDN_URL, str(js_path))
            self._mermaid_js_path = js_path
            self._js_ready = True
            logger.info("mermaid.min.js downloaded to %s", js_path)
        except Exception:
            logger.exception("Failed to download mermaid.min.js")
            self._js_downloading = False
            self._pending_callbacks.clear()
            return

        self._js_downloading = False
        callbacks = list(self._pending_callbacks)
        self._pending_callbacks.clear()
        for cb in callbacks:
            cb()

    # -- Rendering pipeline --

    def _process_queue(self) -> None:
        """Process next item in the render queue."""
        if self._rendering or not self._queue or not self._js_ready:
            return

        code_hash, code = self._queue.pop(0)

        # Check cache again (may have been rendered while waiting)
        if code_hash in self._svg_cache:
            self.svg_ready.emit(code_hash, self._svg_cache[code_hash])
            self._process_queue()
            return

        self._rendering = True
        self._render_in_webview(code_hash, code)

    def _ensure_webview(self):
        """Lazy-init hidden QWebEngineView."""
        if self._web_view is not None:
            return

        from PyQt6.QtWebEngineWidgets import QWebEngineView
        from PyQt6.QtCore import Qt

        self._web_view = QWebEngineView()
        self._web_view.setAttribute(Qt.WidgetAttribute.WA_DontShowOnScreen, True)
        self._web_view.resize(1600, 1200)
        self._web_view.show()  # needed for rendering even though not visible

    @staticmethod
    def _escape_html(text: str) -> str:
        """Escape HTML entities for embedding in <pre> tag."""
        return (
            text.replace("&", "&amp;")
            .replace("<", "&lt;")
            .replace(">", "&gt;")
            .replace('"', "&quot;")
        )

    def _render_in_webview(self, code_hash: str, code: str) -> None:
        """Render mermaid diagram visually in webview for capture.

        Uses mermaid.run() to render <pre class="mermaid"> content into SVG,
        then captures the rendered page as a QPixmap for accurate rendering
        (supports CSS, foreignObject, etc. that QSvgRenderer cannot handle).
        """
        self._ensure_webview()

        escaped = self._escape_html(code)

        html = f"""<!DOCTYPE html>
<html><head><meta charset="utf-8">
<style>
  * {{ margin: 0; padding: 0; box-sizing: border-box; }}
  body {{ background: white; padding: 8px; }}
</style>
</head><body>
<pre class="mermaid">{escaped}</pre>
<script src="mermaid.min.js"></script>
<script>
  window.renderedSvg = null;
  window.renderError = null;
  window.svgWidth = 0;
  window.svgHeight = 0;
  mermaid.initialize({{startOnLoad: false, theme: 'default'}});
  mermaid.run().then(function() {{
    var svg = document.querySelector('svg');
    if (svg) {{
      var rect = svg.getBoundingClientRect();
      window.svgWidth = Math.ceil(rect.width);
      window.svgHeight = Math.ceil(rect.height);
      window.renderedSvg = svg.outerHTML;
    }} else {{
      window.renderError = 'Error: no SVG element after mermaid.run()';
    }}
  }}).catch(function(err) {{
    window.renderError = 'Error: ' + err.toString();
  }});
</script>
</body></html>"""

        self._current_hash = code_hash

        # Base URL allows relative <script src="mermaid.min.js"> to resolve
        base_url = QUrl.fromLocalFile(str(self._mermaid_js_path.parent) + "/")
        self._web_view.loadFinished.connect(self._on_load_finished)
        self._web_view.setHtml(html, base_url)

    def _on_load_finished(self, ok: bool) -> None:
        """Called when HTML is loaded. Poll for SVG result."""
        self._web_view.loadFinished.disconnect(self._on_load_finished)

        if not ok:
            logger.error("Mermaid WebView failed to load HTML")
            self._rendering = False
            self._process_queue()
            return

        self._poll_count = 0
        self._poll_for_svg()

    def _poll_for_svg(self) -> None:
        """Poll JavaScript for rendered SVG."""
        from PyQt6.QtCore import QTimer

        self._web_view.page().runJavaScript(
            "window.renderedSvg || window.renderError",
            self._on_js_result,
        )

    def _on_js_result(self, result) -> None:
        """Handle JavaScript result from polling."""
        from PyQt6.QtCore import QTimer

        self._poll_count += 1

        if result is None and self._poll_count < 50:
            # Not ready yet, poll again after 100ms
            QTimer.singleShot(100, self._poll_for_svg)
            return

        code_hash = self._current_hash

        if result and not str(result).startswith("Error"):
            # Success - cache SVG and capture webview as pixmap
            self._svg_cache[code_hash] = result
            logger.debug("Mermaid SVG rendered for hash %s", code_hash)
            # Get dimensions for webview resize before capture
            self._web_view.page().runJavaScript(
                "[window.svgWidth || 0, window.svgHeight || 0]",
                lambda dims: self._capture_webview_pixmap(
                    code_hash,
                    dims[0] if dims else 800,
                    dims[1] if dims else 600,
                ),
            )
        else:
            logger.error("Mermaid render failed: %s", result)
            self._rendering = False
            self._process_queue()

    def _capture_webview_pixmap(self, code_hash: str, svg_w: int, svg_h: int) -> None:
        """Capture rendered diagram from webview as QPixmap."""
        from PyQt6.QtCore import QTimer

        # Resize webview to fit SVG content (8px CSS padding on each side)
        padding = 16
        new_w = min(max(int(svg_w) + padding, 100), 4000)
        new_h = min(max(int(svg_h) + padding, 100), 4000)
        self._web_view.resize(new_w, new_h)

        def _grab():
            pixmap = self._web_view.grab()
            if not pixmap.isNull():
                self._pixmap_cache[code_hash] = pixmap
                logger.debug(
                    "Captured mermaid pixmap %dx%d for %s",
                    pixmap.width(), pixmap.height(), code_hash,
                )
            else:
                logger.warning("Failed to capture mermaid pixmap for %s", code_hash)

            self._rendering = False
            self.svg_ready.emit(code_hash, self._svg_cache.get(code_hash, ""))
            self._process_queue()

        # Wait for webview to repaint after resize
        QTimer.singleShot(300, _grab)

    # -- SVG to QPixmap (fallback) --

    @staticmethod
    def _svg_to_pixmap(svg: str, width: int) -> QPixmap | None:
        """Convert SVG string to QPixmap scaled to width (fallback).

        Note: QSvgRenderer has limited CSS support — may not render
        foreignObject, CSS variables, or complex styles correctly.
        Prefer _capture_webview_pixmap for accurate rendering.
        """
        try:
            from PyQt6.QtSvg import QSvgRenderer
            from PyQt6.QtCore import QByteArray, Qt

            renderer = QSvgRenderer(QByteArray(svg.encode()))
            if not renderer.isValid():
                return None

            # Scale preserving aspect ratio
            default_size = renderer.defaultSize()
            if default_size.width() <= 0:
                return None

            scale = width / default_size.width()
            h = int(default_size.height() * scale)

            image = QImage(width, h, QImage.Format.Format_ARGB32)
            image.fill(Qt.GlobalColor.transparent)

            from PyQt6.QtGui import QPainter
            painter = QPainter(image)
            renderer.render(painter)
            painter.end()

            return QPixmap.fromImage(image)
        except Exception:
            logger.exception("Failed to convert SVG to QPixmap")
            return None

    def cleanup(self) -> None:
        """Clean up web view resources."""
        if self._web_view is not None:
            self._web_view.close()
            self._web_view.deleteLater()
            self._web_view = None
        self._pixmap_cache.clear()
