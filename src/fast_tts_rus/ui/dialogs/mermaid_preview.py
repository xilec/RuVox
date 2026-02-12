"""Modal dialog for interactive Mermaid diagram preview."""

import logging
from pathlib import Path

from PyQt6.QtCore import QUrl, Qt
from PyQt6.QtWidgets import (
    QDialog,
    QVBoxLayout,
    QHBoxLayout,
    QPushButton,
    QWidget,
)

logger = logging.getLogger(__name__)


class MermaidPreviewDialog(QDialog):
    """Full-screen modal dialog with interactive Mermaid diagram.

    Uses QWebEngineView with mermaid.js loaded from local cache
    for pan/zoom interaction with the rendered diagram.
    """

    def __init__(self, mermaid_js_path: Path, parent: QWidget | None = None):
        super().__init__(parent)
        self._mermaid_js_path = mermaid_js_path
        self._zoom_level = 1.0

        self.setWindowTitle("Mermaid Diagram")
        self.setModal(True)

        # Size: 80% of screen
        if parent and parent.screen():
            screen_geom = parent.screen().availableGeometry()
        else:
            from PyQt6.QtWidgets import QApplication
            screen = QApplication.primaryScreen()
            screen_geom = screen.availableGeometry() if screen else None

        if screen_geom:
            self.resize(
                int(screen_geom.width() * 0.8),
                int(screen_geom.height() * 0.8),
            )
        else:
            self.resize(1024, 768)

        self._setup_ui()

    def _setup_ui(self) -> None:
        """Build dialog layout with toolbar and web view."""
        layout = QVBoxLayout(self)
        layout.setContentsMargins(0, 0, 0, 0)

        # Toolbar
        toolbar = QHBoxLayout()
        toolbar.setContentsMargins(8, 4, 8, 4)

        btn_zoom_in = QPushButton("+")
        btn_zoom_in.setFixedWidth(40)
        btn_zoom_in.setToolTip("Увеличить")
        btn_zoom_in.clicked.connect(self._zoom_in)

        btn_zoom_out = QPushButton("−")
        btn_zoom_out.setFixedWidth(40)
        btn_zoom_out.setToolTip("Уменьшить")
        btn_zoom_out.clicked.connect(self._zoom_out)

        btn_reset = QPushButton("100%")
        btn_reset.setFixedWidth(60)
        btn_reset.setToolTip("Сбросить масштаб")
        btn_reset.clicked.connect(self._zoom_reset)

        self._btn_theme = QPushButton("Тёмная тема")
        self._btn_theme.setFixedWidth(120)
        self._btn_theme.clicked.connect(self._toggle_theme)
        self._dark_theme = False

        btn_close = QPushButton("Закрыть")
        btn_close.clicked.connect(self.accept)

        toolbar.addWidget(btn_zoom_in)
        toolbar.addWidget(btn_zoom_out)
        toolbar.addWidget(btn_reset)
        toolbar.addStretch()
        toolbar.addWidget(self._btn_theme)
        toolbar.addWidget(btn_close)

        layout.addLayout(toolbar)

        # WebEngineView
        from PyQt6.QtWebEngineWidgets import QWebEngineView

        self._web_view = QWebEngineView()
        layout.addWidget(self._web_view)

    def show_diagram(self, code: str, title: str = "") -> None:
        """Load and display a Mermaid diagram.

        Args:
            code: Mermaid diagram source code.
            title: Optional window title suffix.
        """
        if title:
            self.setWindowTitle(f"Mermaid — {title}")

        js_url = QUrl.fromLocalFile(str(self._mermaid_js_path)).toString()

        escaped = (
            code.replace("\\", "\\\\")
            .replace("`", "\\`")
            .replace("$", "\\$")
        )

        theme = "dark" if self._dark_theme else "default"
        bg_color = "#1e1e1e" if self._dark_theme else "#ffffff"
        self._current_code = code

        html = f"""<!DOCTYPE html>
<html><head><meta charset="utf-8">
<style>
  body {{
    margin: 0; padding: 20px;
    display: flex; justify-content: center; align-items: flex-start;
    background: {bg_color};
    overflow: auto;
  }}
  .mermaid {{ zoom: {self._zoom_level}; }}
</style>
</head><body>
<pre class="mermaid">{self._escape_html(code)}</pre>
<script src="{js_url}"></script>
<script>
  mermaid.initialize({{startOnLoad: true, theme: '{theme}'}});
</script>
</body></html>"""

        self._web_view.setHtml(html)
        self.exec()

    @staticmethod
    def _escape_html(text: str) -> str:
        """Escape HTML entities in text."""
        return (
            text.replace("&", "&amp;")
            .replace("<", "&lt;")
            .replace(">", "&gt;")
            .replace('"', "&quot;")
        )

    # -- Zoom --

    def _zoom_in(self) -> None:
        self._zoom_level = min(self._zoom_level + 0.25, 4.0)
        self._apply_zoom()

    def _zoom_out(self) -> None:
        self._zoom_level = max(self._zoom_level - 0.25, 0.25)
        self._apply_zoom()

    def _zoom_reset(self) -> None:
        self._zoom_level = 1.0
        self._apply_zoom()

    def _apply_zoom(self) -> None:
        js = f"document.querySelector('.mermaid').style.zoom = '{self._zoom_level}';"
        self._web_view.page().runJavaScript(js)

    # -- Theme --

    def _toggle_theme(self) -> None:
        self._dark_theme = not self._dark_theme
        self._btn_theme.setText(
            "Светлая тема" if self._dark_theme else "Тёмная тема"
        )
        # Re-render with new theme
        if hasattr(self, "_current_code"):
            js_url = QUrl.fromLocalFile(str(self._mermaid_js_path)).toString()
            theme = "dark" if self._dark_theme else "default"
            bg_color = "#1e1e1e" if self._dark_theme else "#ffffff"

            html = f"""<!DOCTYPE html>
<html><head><meta charset="utf-8">
<style>
  body {{
    margin: 0; padding: 20px;
    display: flex; justify-content: center; align-items: flex-start;
    background: {bg_color};
    overflow: auto;
  }}
  .mermaid {{ zoom: {self._zoom_level}; }}
</style>
</head><body>
<pre class="mermaid">{self._escape_html(self._current_code)}</pre>
<script src="{js_url}"></script>
<script>
  mermaid.initialize({{startOnLoad: true, theme: '{theme}'}});
</script>
</body></html>"""

            self._web_view.setHtml(html)
