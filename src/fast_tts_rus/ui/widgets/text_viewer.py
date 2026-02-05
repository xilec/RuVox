"""Text viewer widget with Markdown support and word highlighting."""

from enum import Enum
from typing import Any

import markdown
from PyQt6.QtCore import Qt
from PyQt6.QtGui import QTextCursor, QTextCharFormat, QColor
from PyQt6.QtWidgets import QTextBrowser, QScrollBar

from fast_tts_rus.ui.models.entry import TextEntry


class TextFormat(Enum):
    """Text display format."""
    MARKDOWN = "markdown"
    PLAIN = "plain"


class TextViewerWidget(QTextBrowser):
    """Text viewer with Markdown rendering and word highlighting.

    Features:
    - Markdown rendering (headers, lists, code blocks, links)
    - Plain text mode
    - Current word highlighting during playback
    - Auto-scroll to current position
    """

    def __init__(self, parent=None):
        super().__init__(parent)

        self.current_entry: TextEntry | None = None
        self.timestamps: list[dict[str, Any]] | None = None
        self.text_format: TextFormat = TextFormat.MARKDOWN
        self._last_highlighted_pos: tuple[int, int] | None = None

        # Position mapping: original text position -> rendered document position
        # Built after each render to enable precise cursor positioning
        self._position_map: list[int] | None = None

        # Setup highlighting format
        self._highlight_format = QTextCharFormat()
        self._highlight_format.setBackground(QColor("#FFFF99"))  # Yellow background
        self._highlight_format.setFontUnderline(True)

        # Normal format (to restore)
        self._normal_format = QTextCharFormat()

        # Configure widget
        self.setReadOnly(True)
        self.setOpenExternalLinks(True)

        # Markdown converter
        self._md = markdown.Markdown(extensions=['fenced_code', 'tables'])

    def set_format(self, fmt: TextFormat) -> None:
        """Switch display format between Markdown and plain text."""
        self.text_format = fmt
        if self.current_entry:
            self._render_text()

    def set_entry(self, entry: TextEntry, timestamps: list[dict[str, Any]] | None = None) -> None:
        """Set the entry to display.

        Args:
            entry: TextEntry to display
            timestamps: Optional word timestamps for highlighting
        """
        self.current_entry = entry
        self.timestamps = timestamps
        self._last_highlighted_pos = None
        self._render_text()

    def clear_entry(self) -> None:
        """Clear the current entry."""
        self.current_entry = None
        self.timestamps = None
        self._last_highlighted_pos = None
        self.clear()

    def _render_text(self) -> None:
        """Render text in current format."""
        if not self.current_entry:
            self.clear()
            self._position_map = None
            return

        text = self.current_entry.original_text

        if self.text_format == TextFormat.MARKDOWN:
            # Convert Markdown to HTML
            self._md.reset()
            html = self._md.convert(text)
            # Add basic styling
            styled_html = f"""
            <style>
                body {{ font-family: sans-serif; line-height: 1.5; }}
                code {{ background-color: #f4f4f4; padding: 2px 4px; }}
                pre {{ background-color: #f4f4f4; padding: 8px; overflow-x: auto; }}
                blockquote {{ border-left: 3px solid #ccc; margin-left: 0; padding-left: 12px; color: #666; }}
            </style>
            {html}
            """
            self.setHtml(styled_html)

            # Build position map: original -> rendered
            self._position_map = self._build_position_map(
                text, self.document().toPlainText()
            )
        else:
            self.setPlainText(text)
            # In plain text mode, positions are 1:1
            self._position_map = None

    def highlight_at_position(self, position_sec: float) -> None:
        """Highlight word at the given audio position.

        Args:
            position_sec: Current playback position in seconds
        """
        if not self.timestamps:
            return

        # Find word at current position
        word_info = self._find_word_at(position_sec)
        if not word_info:
            self._clear_highlight()
            return

        original_pos = word_info.get("original_pos")
        if not original_pos or len(original_pos) != 2:
            return

        start, end = original_pos

        # Skip if same position
        if self._last_highlighted_pos == (start, end):
            return

        # Clear previous highlight
        self._clear_highlight()

        # Apply new highlight
        self._highlight_range(start, end)
        self._last_highlighted_pos = (start, end)

        # Scroll to visible
        self._ensure_visible(start)

    def _find_word_at(self, position_sec: float) -> dict[str, Any] | None:
        """Find word timestamp at given position."""
        if not self.timestamps:
            return None

        for word_info in self.timestamps:
            start = word_info.get("start", 0)
            end = word_info.get("end", 0)
            if start <= position_sec < end:
                return word_info

        return None

    def _highlight_range(self, start: int, end: int) -> None:
        """Highlight text range using position mapping.

        Uses pre-built position map to convert original text positions
        to rendered document positions, avoiding unreliable word search.
        """
        if self.text_format == TextFormat.PLAIN or self._position_map is None:
            # Plain text mode: positions are 1:1
            cursor = self.textCursor()
            cursor.setPosition(start)
            cursor.setPosition(end, QTextCursor.MoveMode.KeepAnchor)
            cursor.mergeCharFormat(self._highlight_format)
        else:
            # Markdown mode: use position map
            doc_len = len(self.document().toPlainText())

            # Map original positions to rendered document positions
            if start < len(self._position_map):
                doc_start = self._position_map[start]
            else:
                doc_start = doc_len

            # For end position, find the mapped position
            if end <= len(self._position_map):
                # Map end-1 and add 1 to get the end of the last character
                doc_end = self._position_map[min(end - 1, len(self._position_map) - 1)] + 1
            else:
                doc_end = doc_len

            # Clamp to document bounds
            doc_start = max(0, min(doc_start, doc_len))
            doc_end = max(doc_start, min(doc_end, doc_len))

            cursor = self.textCursor()
            cursor.setPosition(doc_start)
            cursor.setPosition(doc_end, QTextCursor.MoveMode.KeepAnchor)
            cursor.mergeCharFormat(self._highlight_format)

    def _build_position_map(self, original: str, rendered: str) -> list[int]:
        """Build position map from original text to rendered document.

        Uses sequence alignment to map each character in original text
        to its position in the rendered document.

        Returns:
            List where position_map[orig_idx] = rendered_idx
        """
        # Simple alignment using longest common subsequence approach
        # For each character in original, find its position in rendered

        position_map = []
        rendered_idx = 0

        for orig_idx, orig_char in enumerate(original):
            # Skip whitespace differences (Markdown may normalize whitespace)
            while (rendered_idx < len(rendered) and
                   rendered[rendered_idx] != orig_char and
                   rendered[rendered_idx].isspace() and
                   orig_char.isspace()):
                rendered_idx += 1

            # Find matching character in rendered
            if rendered_idx < len(rendered) and rendered[rendered_idx] == orig_char:
                position_map.append(rendered_idx)
                rendered_idx += 1
            elif orig_char.isspace():
                # Whitespace might be normalized, map to current position
                position_map.append(max(0, rendered_idx - 1))
            else:
                # Character might be removed by Markdown (e.g., ** for bold)
                # Map to the last valid position
                position_map.append(max(0, rendered_idx - 1) if rendered_idx > 0 else 0)

        return position_map

    def _clear_highlight(self) -> None:
        """Clear any existing highlight."""
        if self._last_highlighted_pos is None:
            return

        # Reset all formatting by re-rendering
        # This is simpler than tracking and clearing specific ranges
        cursor = self.textCursor()
        cursor.select(QTextCursor.SelectionType.Document)
        cursor.setCharFormat(self._normal_format)

        # Re-render to restore proper formatting
        self._render_text()
        self._last_highlighted_pos = None

    def _ensure_visible(self, char_pos: int) -> None:
        """Scroll to make character position visible without setting cursor."""
        # Map original position to document position
        if self._position_map is not None and char_pos < len(self._position_map):
            doc_pos = self._position_map[char_pos]
        else:
            doc_pos = char_pos

        # Create temporary cursor to find position
        cursor = QTextCursor(self.document())
        doc_len = len(self.document().toPlainText())
        cursor.setPosition(min(doc_pos, doc_len))

        # Get the rectangle for this position and scroll to it
        rect = self.cursorRect(cursor)
        self.verticalScrollBar().setValue(
            self.verticalScrollBar().value() + rect.top() - self.height() // 3
        )
