"""Theme definitions and runtime switching for RuVox UI."""

from dataclasses import dataclass

from PyQt6.QtGui import QColor, QPalette
from PyQt6.QtWidgets import QApplication


@dataclass(frozen=True)
class ThemeDefinition:
    """Complete theme definition including palette, QSS, and semantic colors."""

    id: str
    name: str
    palette_fn: object  # callable returning QPalette
    qss: str
    highlight_bg: str  # Word highlight during playback
    secondary_text: str  # Info labels color
    accent: str  # Accent color (playing bar, highlights)
    markdown_css: str  # CSS for markdown inside QTextBrowser


# ---------------------------------------------------------------------------
# Palettes
# ---------------------------------------------------------------------------


def _dark_pro_palette() -> QPalette:
    p = QPalette()
    p.setColor(QPalette.ColorRole.Window, QColor("#1e1e2e"))
    p.setColor(QPalette.ColorRole.WindowText, QColor("#e0e0e0"))
    p.setColor(QPalette.ColorRole.Base, QColor("#252536"))
    p.setColor(QPalette.ColorRole.AlternateBase, QColor("#2a2a3d"))
    p.setColor(QPalette.ColorRole.Text, QColor("#e0e0e0"))
    p.setColor(QPalette.ColorRole.Button, QColor("#383852"))
    p.setColor(QPalette.ColorRole.ButtonText, QColor("#e0e0e0"))
    p.setColor(QPalette.ColorRole.Highlight, QColor("#7c3aed"))
    p.setColor(QPalette.ColorRole.HighlightedText, QColor("#ffffff"))
    p.setColor(QPalette.ColorRole.ToolTipBase, QColor("#333355"))
    p.setColor(QPalette.ColorRole.ToolTipText, QColor("#e0e0e0"))
    p.setColor(QPalette.ColorRole.PlaceholderText, QColor("#888888"))
    p.setColor(QPalette.ColorRole.Mid, QColor("#3a3a55"))
    p.setColor(QPalette.ColorRole.Dark, QColor("#16162a"))
    p.setColor(QPalette.ColorRole.Shadow, QColor("#000000"))
    return p


def _minimal_light_palette() -> QPalette:
    p = QPalette()
    p.setColor(QPalette.ColorRole.Window, QColor("#ffffff"))
    p.setColor(QPalette.ColorRole.WindowText, QColor("#111111"))
    p.setColor(QPalette.ColorRole.Base, QColor("#fafafa"))
    p.setColor(QPalette.ColorRole.AlternateBase, QColor("#f5f5f5"))
    p.setColor(QPalette.ColorRole.Text, QColor("#111111"))
    p.setColor(QPalette.ColorRole.Button, QColor("#f0f0f0"))
    p.setColor(QPalette.ColorRole.ButtonText, QColor("#111111"))
    p.setColor(QPalette.ColorRole.Highlight, QColor("#2563eb"))
    p.setColor(QPalette.ColorRole.HighlightedText, QColor("#ffffff"))
    p.setColor(QPalette.ColorRole.ToolTipBase, QColor("#ffffff"))
    p.setColor(QPalette.ColorRole.ToolTipText, QColor("#111111"))
    p.setColor(QPalette.ColorRole.PlaceholderText, QColor("#999999"))
    p.setColor(QPalette.ColorRole.Mid, QColor("#dddddd"))
    p.setColor(QPalette.ColorRole.Dark, QColor("#cccccc"))
    p.setColor(QPalette.ColorRole.Shadow, QColor("#aaaaaa"))
    return p


def _dark_navy_palette() -> QPalette:
    p = QPalette()
    p.setColor(QPalette.ColorRole.Window, QColor("#1a1d21"))
    p.setColor(QPalette.ColorRole.WindowText, QColor("#d1d2d3"))
    p.setColor(QPalette.ColorRole.Base, QColor("#222529"))
    p.setColor(QPalette.ColorRole.AlternateBase, QColor("#282c30"))
    p.setColor(QPalette.ColorRole.Text, QColor("#d1d2d3"))
    p.setColor(QPalette.ColorRole.Button, QColor("#363c42"))
    p.setColor(QPalette.ColorRole.ButtonText, QColor("#d1d2d3"))
    p.setColor(QPalette.ColorRole.Highlight, QColor("#8fa8c8"))
    p.setColor(QPalette.ColorRole.HighlightedText, QColor("#ffffff"))
    p.setColor(QPalette.ColorRole.ToolTipBase, QColor("#2c3036"))
    p.setColor(QPalette.ColorRole.ToolTipText, QColor("#d1d2d3"))
    p.setColor(QPalette.ColorRole.PlaceholderText, QColor("#9a9b9d"))
    p.setColor(QPalette.ColorRole.Mid, QColor("#3a3e44"))
    p.setColor(QPalette.ColorRole.Dark, QColor("#15181c"))
    p.setColor(QPalette.ColorRole.Shadow, QColor("#000000"))
    return p


# ---------------------------------------------------------------------------
# QSS stylesheets
# ---------------------------------------------------------------------------

_DARK_PRO_QSS = """
QMainWindow { background-color: #1e1e2e; }
QWidget { color: #e0e0e0; }
QListWidget { background-color: #252536; border: 1px solid #3a3a55; border-radius: 4px; }
QListWidget::item { padding: 4px; border-bottom: 1px solid #2a2a3d; }
QListWidget::item:selected { background-color: #7c3aed; }
QListWidget::item:selected:alternate { background-color: #7c3aed; }
QListWidget::item:alternate { background-color: #2a2a3d; }
QTextBrowser { background-color: #252536; border: 1px solid #3a3a55; border-radius: 4px; color: #e0e0e0; }
QPushButton {
    background: #383852; border: 1px solid #3a3a55;
    border-radius: 6px; padding: 4px 8px; color: #e0e0e0;
}
QPushButton:hover { background-color: #4a4a6a; border-color: #7c3aed; }
QPushButton:disabled { color: #555; }
QSlider::groove:horizontal { background: #3a3a55; height: 6px; border-radius: 3px; }
QSlider::handle:horizontal { background: #7c3aed; width: 14px; margin: -4px 0; border-radius: 7px; }
QSlider::sub-page:horizontal { background: #7c3aed; border-radius: 3px; }
QComboBox {
    background: #383852; border: 1px solid #3a3a55;
    border-radius: 4px; padding: 4px 8px; color: #e0e0e0;
}
QStatusBar { background-color: #16162a; color: #888888; }
QLabel { color: #e0e0e0; }
QueueItemWidget QLabel[objectName="info_label"] { color: #888888; }
"""

_MINIMAL_LIGHT_QSS = """
QMainWindow { background-color: #ffffff; }
QWidget { color: #111111; }
QListWidget { background-color: #fafafa; border: 1px solid #e0e0e0; border-radius: 2px; }
QListWidget::item { padding: 4px; border-bottom: 1px solid #f0f0f0; }
QListWidget::item:selected { background-color: #dbeafe; }
QListWidget::item:selected:alternate { background-color: #dbeafe; }
QListWidget::item:alternate { background-color: #f5f5f5; }
QTextBrowser { background-color: #fafafa; border: 1px solid #e0e0e0; border-radius: 2px; color: #111111; }
QPushButton {
    background: #f0f0f0; border: 1px solid #e0e0e0;
    border-radius: 4px; padding: 4px 8px; color: #111111;
}
QPushButton:hover { background-color: #e8e8e8; border-color: #2563eb; }
QPushButton:disabled { color: #bbb; }
QSlider::groove:horizontal { background: #e0e0e0; height: 6px; border-radius: 3px; }
QSlider::handle:horizontal { background: #2563eb; width: 14px; margin: -4px 0; border-radius: 7px; }
QSlider::sub-page:horizontal { background: #2563eb; border-radius: 3px; }
QComboBox {
    background: #ffffff; border: 1px solid #e0e0e0;
    border-radius: 4px; padding: 4px 8px; color: #111111;
}
QStatusBar { background-color: #fafafa; color: #999999; }
QLabel { color: #111111; }
QueueItemWidget QLabel[objectName="info_label"] { color: #999999; }
"""

_DARK_NAVY_QSS = """
QMainWindow { background-color: #1a1d21; }
QWidget { color: #d1d2d3; }
QListWidget { background-color: #222529; border: 1px solid #3a3e44; border-radius: 4px; }
QListWidget::item { padding: 4px; border-bottom: 1px solid #2c3036; }
QListWidget::item:selected { background-color: #253548; }
QListWidget::item:selected:alternate { background-color: #253548; }
QListWidget::item:alternate { background-color: #282c30; }
QTextBrowser { background-color: #222529; border: 1px solid #3a3e44; border-radius: 4px; color: #d1d2d3; }
QPushButton {
    background: #363c42; border: 1px solid #3a3e44;
    border-radius: 6px; padding: 4px 8px; color: #d1d2d3;
}
QPushButton:hover { background-color: #454b52; border-color: #8fa8c8; }
QPushButton:disabled { color: #555; }
QSlider::groove:horizontal { background: #3a3e44; height: 6px; border-radius: 3px; }
QSlider::handle:horizontal { background: #8fa8c8; width: 14px; margin: -4px 0; border-radius: 7px; }
QSlider::sub-page:horizontal { background: #8fa8c8; border-radius: 3px; }
QComboBox {
    background: #363c42; border: 1px solid #3a3e44;
    border-radius: 4px; padding: 4px 8px; color: #d1d2d3;
}
QStatusBar { background-color: #15181c; color: #9a9b9d; }
QLabel { color: #d1d2d3; }
QueueItemWidget QLabel[objectName="info_label"] { color: #9a9b9d; }
"""

# ---------------------------------------------------------------------------
# Markdown CSS per theme
# ---------------------------------------------------------------------------

_DARK_PRO_MARKDOWN_CSS = """
body { font-family: sans-serif; line-height: 1.5; color: #e0e0e0; }
code { background-color: #2a2a3d; padding: 2px 4px; border-radius: 3px; }
pre { background-color: #2a2a3d; padding: 8px; border-radius: 4px; overflow-x: auto; }
pre code { background-color: transparent; padding: 0; }
blockquote { border-left: 3px solid #3a3a55; margin-left: 0; padding-left: 12px; color: #888888; }
h1, h2, h3, h4, h5, h6 { margin-top: 0.5em; margin-bottom: 0.3em; }
ul, ol { margin-top: 0.3em; margin-bottom: 0.3em; }
img.mermaid-diagram {
    cursor: pointer; max-width: 100%;
    border: 1px solid #3a3a55; border-radius: 4px; padding: 8px;
}
.mermaid-placeholder {
    background: #2a2a3d; border: 1px dashed #3a3a55;
    border-radius: 4px; padding: 12px; text-align: center; color: #888888;
}
"""

_MINIMAL_LIGHT_MARKDOWN_CSS = """
body { font-family: sans-serif; line-height: 1.5; color: #111111; }
code { background-color: #f0f0f0; padding: 2px 4px; border-radius: 3px; }
pre { background-color: #f0f0f0; padding: 8px; border-radius: 4px; overflow-x: auto; }
pre code { background-color: transparent; padding: 0; }
blockquote { border-left: 3px solid #e0e0e0; margin-left: 0; padding-left: 12px; color: #666666; }
h1, h2, h3, h4, h5, h6 { margin-top: 0.5em; margin-bottom: 0.3em; }
ul, ol { margin-top: 0.3em; margin-bottom: 0.3em; }
img.mermaid-diagram {
    cursor: pointer; max-width: 100%;
    border: 1px solid #e0e0e0; border-radius: 4px; padding: 8px;
}
.mermaid-placeholder {
    background: #f5f5f5; border: 1px dashed #cccccc;
    border-radius: 4px; padding: 12px; text-align: center; color: #999999;
}
"""

_DARK_NAVY_MARKDOWN_CSS = """
body { font-family: sans-serif; line-height: 1.5; color: #d1d2d3; }
code { background-color: #282c30; padding: 2px 4px; border-radius: 3px; }
pre { background-color: #282c30; padding: 8px; border-radius: 4px; overflow-x: auto; }
pre code { background-color: transparent; padding: 0; }
blockquote { border-left: 3px solid #3a3e44; margin-left: 0; padding-left: 12px; color: #9a9b9d; }
h1, h2, h3, h4, h5, h6 { margin-top: 0.5em; margin-bottom: 0.3em; }
ul, ol { margin-top: 0.3em; margin-bottom: 0.3em; }
img.mermaid-diagram {
    cursor: pointer; max-width: 100%;
    border: 1px solid #3a3e44; border-radius: 4px; padding: 8px;
}
.mermaid-placeholder {
    background: #282c30; border: 1px dashed #3a3e44;
    border-radius: 4px; padding: 12px; text-align: center; color: #9a9b9d;
}
"""

# ---------------------------------------------------------------------------
# Theme registry
# ---------------------------------------------------------------------------

THEMES: dict[str, ThemeDefinition] = {
    "dark_pro": ThemeDefinition(
        id="dark_pro",
        name="Dark Professional",
        palette_fn=_dark_pro_palette,
        qss=_DARK_PRO_QSS,
        highlight_bg="#6b4d9e",
        secondary_text="#888888",
        accent="#7c3aed",
        markdown_css=_DARK_PRO_MARKDOWN_CSS,
    ),
    "minimal_light": ThemeDefinition(
        id="minimal_light",
        name="Minimal Light",
        palette_fn=_minimal_light_palette,
        qss=_MINIMAL_LIGHT_QSS,
        highlight_bg="#FFF176",
        secondary_text="#999999",
        accent="#2563eb",
        markdown_css=_MINIMAL_LIGHT_MARKDOWN_CSS,
    ),
    "dark_navy": ThemeDefinition(
        id="dark_navy",
        name="Dark Navy",
        palette_fn=_dark_navy_palette,
        qss=_DARK_NAVY_QSS,
        highlight_bg="#3a6478",
        secondary_text="#9a9b9d",
        accent="#8fa8c8",
        markdown_css=_DARK_NAVY_MARKDOWN_CSS,
    ),
}

DEFAULT_THEME = "dark_pro"

_current_theme_id: str = DEFAULT_THEME


def get_theme(theme_id: str) -> ThemeDefinition:
    """Get theme by ID. Falls back to default if not found."""
    return THEMES.get(theme_id, THEMES[DEFAULT_THEME])


def get_current_theme() -> ThemeDefinition:
    """Get the currently active theme."""
    return get_theme(_current_theme_id)


def get_available_themes() -> list[tuple[str, str]]:
    """Return list of (id, display_name) for all available themes."""
    return [(t.id, t.name) for t in THEMES.values()]


def apply_theme(theme_id: str) -> None:
    """Apply theme globally via QApplication."""
    global _current_theme_id

    theme = get_theme(theme_id)
    _current_theme_id = theme.id

    app = QApplication.instance()
    if app is None:
        return

    app.setPalette(theme.palette_fn())
    app.setStyleSheet(theme.qss)
