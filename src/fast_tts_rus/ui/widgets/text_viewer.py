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
        else:
            self.setPlainText(text)

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
        """Highlight text range.

        Note: In Markdown mode, positions may not match exactly due to HTML
        conversion. This is a best-effort implementation.
        """
        if self.text_format == TextFormat.PLAIN:
            cursor = self.textCursor()
            cursor.setPosition(start)
            cursor.setPosition(end, QTextCursor.MoveMode.KeepAnchor)
            cursor.mergeCharFormat(self._highlight_format)
        else:
            # In Markdown mode, we need to search for the text
            # This is approximate since HTML conversion changes positions
            if not self.current_entry:
                return

            # Get the original word
            original_text = self.current_entry.original_text
            if start < len(original_text) and end <= len(original_text):
                word = original_text[start:end]
                # Find and highlight in the rendered document
                cursor = self.document().find(word)
                if not cursor.isNull():
                    cursor.mergeCharFormat(self._highlight_format)

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
        if self.text_format == TextFormat.PLAIN:
            # Create temporary cursor to find position, don't set it as widget cursor
            cursor = QTextCursor(self.document())
            cursor.setPosition(char_pos)
            # Get the rectangle for this position and scroll to it
            rect = self.cursorRect(cursor)
            self.verticalScrollBar().setValue(
                self.verticalScrollBar().value() + rect.top() - self.height() // 3
            )
        else:
            # In Markdown mode, find approximate position
            if self.current_entry:
                word = self.current_entry.original_text[char_pos:char_pos + 10]
                first_word = word.split()[0] if word.split() else ""
                if first_word:
                    cursor = self.document().find(first_word)
                    if not cursor.isNull():
                        rect = self.cursorRect(cursor)
                        self.verticalScrollBar().setValue(
                            self.verticalScrollBar().value() + rect.top() - self.height() // 3
                        )
