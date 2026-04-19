"""Custom player icons drawn via QPainterPath.

Replaces QStyle.StandardPixmap to ensure icons respect theme colors
(Fusion style hardcodes icon color regardless of palette).
"""

from PyQt6.QtCore import QRectF, Qt
from PyQt6.QtGui import QColor, QIcon, QPainter, QPainterPath, QPixmap


def _render_pixmap(size: int, draw_fn, color: str) -> QPixmap:
    """Render a pixmap by calling draw_fn(painter, rect) with the given color."""
    pixmap = QPixmap(size, size)
    pixmap.fill(QColor(0, 0, 0, 0))
    painter = QPainter(pixmap)
    painter.setRenderHint(QPainter.RenderHint.Antialiasing)
    painter.setBrush(QColor(color))
    painter.setPen(Qt.PenStyle.NoPen)
    margin = size * 0.15
    rect = QRectF(margin, margin, size - 2 * margin, size - 2 * margin)
    draw_fn(painter, rect)
    painter.end()
    return pixmap


def _render_icon(size: int, draw_fn, color: str, disabled_color: str) -> QIcon:
    """Render a QIcon with Normal and Disabled pixmaps."""
    icon = QIcon()
    icon.addPixmap(_render_pixmap(size, draw_fn, color), QIcon.Mode.Normal)
    icon.addPixmap(_render_pixmap(size, draw_fn, disabled_color), QIcon.Mode.Disabled)
    return icon


def _draw_play(painter: QPainter, r: QRectF) -> None:
    """Right-pointing triangle (play)."""
    path = QPainterPath()
    path.moveTo(r.left(), r.top())
    path.lineTo(r.right(), r.center().y())
    path.lineTo(r.left(), r.bottom())
    path.closeSubpath()
    painter.drawPath(path)


def _draw_pause(painter: QPainter, r: QRectF) -> None:
    """Two vertical bars (pause)."""
    bar_w = r.width() * 0.3
    gap = r.width() * 0.2
    x1 = r.left() + (r.width() - 2 * bar_w - gap) / 2
    x2 = x1 + bar_w + gap
    painter.drawRect(QRectF(x1, r.top(), bar_w, r.height()))
    painter.drawRect(QRectF(x2, r.top(), bar_w, r.height()))


def _draw_seek_backward(painter: QPainter, r: QRectF) -> None:
    """Two left-pointing triangles (seek backward)."""
    mid = r.center().x()
    cy = r.center().y()
    # Left triangle
    path = QPainterPath()
    path.moveTo(mid, r.top())
    path.lineTo(r.left(), cy)
    path.lineTo(mid, r.bottom())
    path.closeSubpath()
    # Right triangle
    path.moveTo(r.right(), r.top())
    path.lineTo(mid, cy)
    path.lineTo(r.right(), r.bottom())
    path.closeSubpath()
    painter.drawPath(path)


def _draw_seek_forward(painter: QPainter, r: QRectF) -> None:
    """Two right-pointing triangles (seek forward)."""
    mid = r.center().x()
    cy = r.center().y()
    # Left triangle
    path = QPainterPath()
    path.moveTo(r.left(), r.top())
    path.lineTo(mid, cy)
    path.lineTo(r.left(), r.bottom())
    path.closeSubpath()
    # Right triangle
    path.moveTo(mid, r.top())
    path.lineTo(r.right(), cy)
    path.lineTo(mid, r.bottom())
    path.closeSubpath()
    painter.drawPath(path)


def _draw_skip_backward(painter: QPainter, r: QRectF) -> None:
    """Bar + left-pointing triangle (skip backward)."""
    bar_w = r.width() * 0.15
    tri_left = r.left() + bar_w + r.width() * 0.05
    cy = r.center().y()
    # Bar
    painter.drawRect(QRectF(r.left(), r.top(), bar_w, r.height()))
    # Triangle
    path = QPainterPath()
    path.moveTo(r.right(), r.top())
    path.lineTo(tri_left, cy)
    path.lineTo(r.right(), r.bottom())
    path.closeSubpath()
    painter.drawPath(path)


def _draw_skip_forward(painter: QPainter, r: QRectF) -> None:
    """Right-pointing triangle + bar (skip forward)."""
    bar_w = r.width() * 0.15
    tri_right = r.right() - bar_w - r.width() * 0.05
    cy = r.center().y()
    # Triangle
    path = QPainterPath()
    path.moveTo(r.left(), r.top())
    path.lineTo(tri_right, cy)
    path.lineTo(r.left(), r.bottom())
    path.closeSubpath()
    painter.drawPath(path)
    # Bar
    painter.drawRect(QRectF(r.right() - bar_w, r.top(), bar_w, r.height()))


def _draw_arrow_up(painter: QPainter, r: QRectF) -> None:
    """Filled upward triangle."""
    path = QPainterPath()
    path.moveTo(r.center().x(), r.top())
    path.lineTo(r.right(), r.bottom())
    path.lineTo(r.left(), r.bottom())
    path.closeSubpath()
    painter.drawPath(path)


def _draw_arrow_down(painter: QPainter, r: QRectF) -> None:
    """Filled downward triangle."""
    path = QPainterPath()
    path.moveTo(r.left(), r.top())
    path.lineTo(r.right(), r.top())
    path.lineTo(r.center().x(), r.bottom())
    path.closeSubpath()
    painter.drawPath(path)


# --- Public API ---


def icon_play(color: str, disabled_color: str, size: int = 16) -> QIcon:
    return _render_icon(size, _draw_play, color, disabled_color)


def icon_pause(color: str, disabled_color: str, size: int = 16) -> QIcon:
    return _render_icon(size, _draw_pause, color, disabled_color)


def icon_seek_backward(color: str, disabled_color: str, size: int = 16) -> QIcon:
    return _render_icon(size, _draw_seek_backward, color, disabled_color)


def icon_seek_forward(color: str, disabled_color: str, size: int = 16) -> QIcon:
    return _render_icon(size, _draw_seek_forward, color, disabled_color)


def icon_skip_backward(color: str, disabled_color: str, size: int = 16) -> QIcon:
    return _render_icon(size, _draw_skip_backward, color, disabled_color)


def icon_skip_forward(color: str, disabled_color: str, size: int = 16) -> QIcon:
    return _render_icon(size, _draw_skip_forward, color, disabled_color)


def icon_arrow_up(color: str, disabled_color: str, size: int = 10) -> QIcon:
    return _render_icon(size, _draw_arrow_up, color, disabled_color)


def icon_arrow_down(color: str, disabled_color: str, size: int = 10) -> QIcon:
    return _render_icon(size, _draw_arrow_down, color, disabled_color)
