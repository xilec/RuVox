"""Text viewer widget with Markdown support and word highlighting."""

import logging
import re
from enum import Enum
from typing import Any

import markdown
from PyQt6.QtCore import Qt, QUrl
from PyQt6.QtGui import QDesktopServices, QTextCursor, QTextCharFormat, QColor
from PyQt6.QtWidgets import QTextBrowser, QTextEdit, QScrollBar, QWidget

from fast_tts_rus.ui.models.entry import TextEntry
from fast_tts_rus.ui.utils.markdown_mapper import MarkdownPositionMapper

logger = logging.getLogger(__name__)

# Regex to extract ```mermaid ... ``` blocks from raw Markdown
_MERMAID_BLOCK_RE = re.compile(
    r"```mermaid\s*\n(.*?)```",
    re.DOTALL,
)


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
        # Handle all anchor clicks ourselves (mermaid:// and external)
        self.setOpenExternalLinks(False)
        self.anchorClicked.connect(self._on_anchor_clicked)

        # Markdown converter
        self._md = markdown.Markdown(extensions=['fenced_code', 'tables'])

        # Markdown position mapper for accurate highlighting
        self._markdown_mapper: MarkdownPositionMapper | None = None

        # Mermaid rendering state
        self._mermaid_blocks: list[str] = []  # diagram code for current text
        self._mermaid_renderer = None  # lazy init: MermaidRenderer

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
            self._mermaid_blocks = []
            return

        text = self.current_entry.original_text

        if self.text_format == TextFormat.MARKDOWN:
            # 1. Extract mermaid blocks BEFORE markdown conversion
            text_without_mermaid, mermaid_blocks = self._extract_mermaid_blocks(text)
            self._mermaid_blocks = mermaid_blocks

            # 2. Convert remaining Markdown to HTML
            self._md.reset()
            html = self._md.convert(text_without_mermaid)

            # 3. Replace placeholders with images or text links
            if mermaid_blocks:
                html = self._inject_mermaid_images(html, mermaid_blocks)

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
                img.mermaid-diagram {{ cursor: pointer; max-width: 100%; border: 1px solid #ddd; border-radius: 4px; padding: 8px; }}
                .mermaid-placeholder {{ background: #f0f0f0; border: 1px dashed #aaa; border-radius: 4px; padding: 12px; text-align: center; color: #666; }}
            </style>
            {html}
            """

            # 4. Set HTML in document (loadResource() provides mermaid images on demand)
            self.setHtml(styled_html)

            # Build position mapping from the actual document
            # This ensures mapper.rendered_plain matches toPlainText()
            self._markdown_mapper = MarkdownPositionMapper(text)
            self._markdown_mapper.rendered_plain = self.toPlainText()
            self._markdown_mapper._build_position_map()

            # 5. Start async rendering for uncached blocks
            if mermaid_blocks:
                self._start_mermaid_rendering(mermaid_blocks)

            logger.debug("Rendered Markdown with position mapping")
        else:
            # Plain text mode - no mapping needed
            self._markdown_mapper = None
            self._mermaid_blocks = []
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

    # -- Mermaid support --

    def _extract_mermaid_blocks(self, text: str) -> tuple[str, list[str]]:
        """Extract ```mermaid...``` blocks, replace with placeholder markers.

        Returns:
            Tuple of (text_with_placeholders, list_of_mermaid_code_strings).
        """
        blocks: list[str] = []

        def _replace(match: re.Match) -> str:
            blocks.append(match.group(1).strip())
            return f"<!--MERMAID:{len(blocks) - 1}-->"

        text_out = _MERMAID_BLOCK_RE.sub(_replace, text)
        return text_out, blocks

    def _inject_mermaid_images(self, html: str, blocks: list[str]) -> str:
        """Replace <!--MERMAID:N--> placeholders in HTML.

        If SVG is cached → <a href="mermaid://N"><img ...></a>
        If not cached → text placeholder with link.
        """
        renderer = self._mermaid_renderer

        for i, code in enumerate(blocks):
            marker = f"<!--MERMAID:{i}-->"
            # Markdown wraps it in <p> tags
            marker_in_p = f"<p>{marker}</p>"

            has_cached = renderer and renderer.get_cached_svg(code)

            if has_cached:
                replacement = (
                    f'<p><a href="mermaid:{i}">'
                    f'<img class="mermaid-diagram" src="mermaid-img:{i}" '
                    f'alt="Mermaid diagram" width="600" /></a></p>'
                )
            else:
                replacement = (
                    f'<p class="mermaid-placeholder">'
                    f'<a href="mermaid:{i}">'
                    f'[Mermaid-диаграмма — загружается...]'
                    f'</a></p>'
                )

            if marker_in_p in html:
                html = html.replace(marker_in_p, replacement)
            else:
                html = html.replace(marker, replacement)

        return html

    def loadResource(self, type_: int, url: QUrl):
        """Provide mermaid diagram pixmaps on demand.

        Qt calls this when it encounters a resource URL (e.g. ``mermaid-img:0``)
        during HTML rendering — the image is available exactly when needed.
        """
        if url.scheme() == "mermaid-img" and self._mermaid_renderer:
            try:
                idx = int(url.path())
                if 0 <= idx < len(self._mermaid_blocks):
                    pixmap = self._mermaid_renderer.get_cached_pixmap(
                        self._mermaid_blocks[idx], 600
                    )
                    if pixmap:
                        return pixmap
            except (ValueError, TypeError):
                pass
        return super().loadResource(type_, url)

    def _start_mermaid_rendering(self, blocks: list[str]) -> None:
        """Start async rendering for uncached mermaid blocks."""
        uncached = [
            code for code in blocks
            if not (self._mermaid_renderer and self._mermaid_renderer.get_cached_svg(code))
        ]

        if not uncached:
            return

        renderer = self._init_mermaid_renderer()
        for code in uncached:
            renderer.render(code)

    def _on_mermaid_ready(self, code_hash: str, svg: str) -> None:
        """Callback when SVG rendering is complete. Re-render if relevant."""
        from fast_tts_rus.ui.services.mermaid_renderer import _code_hash

        # Check if any current mermaid block matches this hash
        if not self._mermaid_blocks:
            return

        for code in self._mermaid_blocks:
            if _code_hash(code) == code_hash:
                logger.debug("Mermaid SVG ready, re-rendering text viewer")
                self._render_text()
                return

    def _on_anchor_clicked(self, url: QUrl) -> None:
        """Handle anchor clicks: mermaid:N opens preview, others open in browser."""
        scheme = url.scheme()

        if scheme == "mermaid":
            # Parse index from path: mermaid:0 → path "0"
            try:
                idx = int(url.path())
            except (ValueError, TypeError):
                return

            if 0 <= idx < len(self._mermaid_blocks):
                self._show_mermaid_preview(idx)
        else:
            # Open external URL in default browser
            QDesktopServices.openUrl(url)

    def _show_mermaid_preview(self, index: int) -> None:
        """Open MermaidPreviewDialog for the given block index."""
        renderer = self._mermaid_renderer
        if not renderer or not renderer.mermaid_js_path():
            logger.warning("Mermaid JS not available for preview")
            return

        code = self._mermaid_blocks[index]

        from fast_tts_rus.ui.dialogs.mermaid_preview import MermaidPreviewDialog

        dialog = MermaidPreviewDialog(renderer.mermaid_js_path(), parent=self)
        dialog.show_diagram(code, title=f"Диаграмма {index + 1}")

    def _init_mermaid_renderer(self):
        """Lazy-init MermaidRenderer."""
        if self._mermaid_renderer is None:
            from fast_tts_rus.ui.services.mermaid_renderer import MermaidRenderer

            self._mermaid_renderer = MermaidRenderer(parent=self)
            self._mermaid_renderer.svg_ready.connect(self._on_mermaid_ready)

        return self._mermaid_renderer
