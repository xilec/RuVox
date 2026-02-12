"""Text viewer widget with Markdown support and word highlighting."""

import logging
from enum import Enum
from typing import Any

import markdown
from PyQt6.QtCore import Qt
from PyQt6.QtGui import QTextCursor, QTextCharFormat, QColor
from PyQt6.QtWidgets import QTextBrowser, QTextEdit, QScrollBar, QWidget

from fast_tts_rus.ui.models.entry import TextEntry
from fast_tts_rus.ui.utils.markdown_mapper import MarkdownPositionMapper

logger = logging.getLogger(__name__)


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

    def __init__(self, parent: QWidget | None = None):
        super().__init__(parent)

        self.current_entry: TextEntry | None = None
        self.timestamps: list[dict[str, Any]] | None = None
        self.text_format: TextFormat = TextFormat.PLAIN
        self._last_highlighted_pos: tuple[int, int] | None = None

        # Setup highlighting format (using ExtraSelections to preserve document formatting)
        self._highlight_format = QTextCharFormat()
        self._highlight_format.setBackground(QColor("#FFFF99"))  # Yellow background
        self._highlight_format.setFontUnderline(True)

        # Configure widget
        self.setReadOnly(True)
        self.setOpenExternalLinks(True)

        # Markdown converter
        self._md = markdown.Markdown(extensions=['fenced_code', 'tables'])

        # Markdown position mapper for accurate highlighting
        self._markdown_mapper: MarkdownPositionMapper | None = None

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
        self._markdown_mapper = None
        self.setExtraSelections([])  # Clear any highlight
        self.clear()

    def _render_text(self) -> None:
        """Render text in current format."""
        if not self.current_entry:
            self.clear()
            self._markdown_mapper = None
            return

        text = self.current_entry.original_text

        if self.text_format == TextFormat.MARKDOWN:
            # Convert Markdown to HTML first
            self._md.reset()
            html = self._md.convert(text)

            # Add basic styling
            styled_html = f"""
            <style>
                body {{ font-family: sans-serif; line-height: 1.5; }}
                code {{ background-color: #f4f4f4; padding: 2px 4px; border-radius: 3px; }}
                pre {{ background-color: #f4f4f4; padding: 8px; border-radius: 4px; overflow-x: auto; }}
                pre code {{ background-color: transparent; padding: 0; }}
                blockquote {{ border-left: 3px solid #ccc; margin-left: 0; padding-left: 12px; color: #666; }}
                h1, h2, h3, h4, h5, h6 {{ margin-top: 0.5em; margin-bottom: 0.3em; }}
                ul, ol {{ margin-top: 0.3em; margin-bottom: 0.3em; }}
            </style>
            {html}
            """

            # Set HTML in document FIRST
            self.setHtml(styled_html)

            # Build position mapping from the actual document
            # This ensures mapper.rendered_plain matches toPlainText()
            self._markdown_mapper = MarkdownPositionMapper(text)
            self._markdown_mapper.rendered_plain = self.toPlainText()
            self._markdown_mapper._build_position_map()

            logger.debug("Rendered Markdown with position mapping")
        else:
            # Plain text mode - no mapping needed
            self._markdown_mapper = None
            self.setPlainText(text)
            logger.debug("Rendered plain text")

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

        # Apply new highlight and get document position
        doc_pos, doc_end = self._highlight_range(start, end)
        self._last_highlighted_pos = (start, end)

        # Scroll to visible
        if doc_pos is not None:
            self._ensure_visible_at_doc_pos(doc_pos)

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

    def _highlight_range(self, start: int, end: int) -> tuple[int | None, int | None]:
        """Highlight text range in the document using ExtraSelections.

        For plain text: positions are 1:1.
        For Markdown: uses position mapper for accurate highlighting.

        Args:
            start: Start position in original text (inclusive)
            end: End position in original text (exclusive)

        Returns:
            Tuple of (doc_start, doc_end) positions, or (None, None) if not found.
        """
        if self.text_format == TextFormat.PLAIN:
            # Plain text mode: positions are 1:1
            doc_start, doc_end = start, end
        else:
            # Markdown mode: use position mapper
            if not self._markdown_mapper:
                logger.warning("Markdown mapper not available for highlighting")
                return None, None

            # Get rendered position from mapper
            result = self._markdown_mapper.get_rendered_range(start, end)
            if not result:
                logger.debug("No mapping found for range [%d, %d)", start, end)
                return None, None

            doc_start, doc_end = result
            logger.debug(
                "Highlighted original[%d:%d] -> rendered[%d:%d]",
                start, end, doc_start, doc_end
            )

        # Apply highlight using ExtraSelections (preserves document formatting)
        cursor = QTextCursor(self.document())
        cursor.setPosition(doc_start)
        cursor.setPosition(doc_end, QTextCursor.MoveMode.KeepAnchor)

        selection = QTextEdit.ExtraSelection()
        selection.cursor = cursor
        selection.format = self._highlight_format

        self.setExtraSelections([selection])

        return doc_start, doc_end

    def _clear_highlight(self) -> None:
        """Clear any existing highlight."""
        # Clear ExtraSelections (removes highlight without affecting document formatting)
        self.setExtraSelections([])
        self._last_highlighted_pos = None

    def _ensure_visible_at_doc_pos(self, doc_pos: int) -> None:
        """Scroll to make document position visible without setting cursor."""
        # Create temporary cursor to find position
        cursor = QTextCursor(self.document())
        doc_len = len(self.document().toPlainText())
        cursor.setPosition(min(doc_pos, doc_len))

        # Get the rectangle for this position and scroll to it
        rect = self.cursorRect(cursor)
        self.verticalScrollBar().setValue(
            self.verticalScrollBar().value() + rect.top() - self.height() // 3
        )
