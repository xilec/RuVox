"""Markdown position mapper for text highlighting.

Maps character positions between original Markdown text and rendered plain text,
enabling accurate word highlighting when displaying Markdown with formatting.

Uses difflib.SequenceMatcher for precise character-level alignment between
original Markdown and rendered plain text, eliminating the need for search-based
position mapping.
"""

import logging

import markdown

logger = logging.getLogger(__name__)


class MarkdownPositionMapper:
    """Maps positions between original Markdown text and rendered plain text.

    This class builds a mapping that allows converting character positions
    from the original Markdown text to positions in the rendered plain text
    (as extracted from HTML via toPlainText()).

    The mapping handles Markdown syntax that gets removed during rendering
    (e.g., `#`, `**`, `` ` ``, `[text](url)` → `text`).

    Example:
        >>> original = "Some **bold** text"
        >>> mapper = MarkdownPositionMapper(original)
        >>> html = mapper.build_mapping()
        >>> # Word "bold" at original[7:11]
        >>> result = mapper.get_rendered_range(7, 11)
        >>> result  # (5, 9) - position in "Some bold text"
        (5, 9)

    Attributes:
        original_text: Original Markdown text
        rendered_plain: Rendered plain text (HTML with tags removed)
        position_map: Dictionary mapping original_pos -> rendered_pos
    """

    def __init__(self, original_text: str):
        """Initialize mapper with original Markdown text.

        Args:
            original_text: Original Markdown text to map
        """
        self.original_text = original_text
        self.rendered_plain = ""
        self.position_map: dict[int, int] = {}

    def build_mapping(self, md_instance: markdown.Markdown | None = None) -> str:
        """Build position mapping and return rendered HTML.

        This method:
        1. Converts Markdown to HTML
        2. Extracts plain text from HTML (simulating QTextDocument.toPlainText())
        3. Builds character-level mapping between original and rendered positions

        Args:
            md_instance: Optional Markdown instance to use for conversion.
                If None, creates a new instance with fenced_code and tables extensions.

        Returns:
            Rendered HTML string
        """
        if md_instance is None:
            md_instance = markdown.Markdown(extensions=["fenced_code", "tables"])

        # Convert Markdown to HTML
        html = md_instance.convert(self.original_text)

        # Extract plain text (simulate QTextDocument.toPlainText())
        self.rendered_plain = self._html_to_plain(html)

        # Handle empty text case
        if not html or not self.rendered_plain:
            return html

        # Build exact character-level position mapping using sequence alignment
        self._build_position_map()

        return html

    def get_rendered_range(self, original_start: int, original_end: int) -> tuple[int, int] | None:
        """Get position range in rendered text from original text range.

        Handles cases where boundaries fall on Markdown syntax that gets removed
        during rendering (e.g., `**`, `` ` ``).

        Args:
            original_start: Start position in original text (inclusive)
            original_end: End position in original text (exclusive)

        Returns:
            Tuple of (rendered_start, rendered_end) or None if mapping not found.
            Positions are in the rendered plain text.

        Example:
            >>> # original: "Some **bold** text"
            >>> # rendered: "Some bold text"
            >>> mapper.get_rendered_range(7, 11)  # "bold" in original
            (5, 9)  # "bold" in rendered
        """
        if not self.position_map:
            logger.warning("Position map not built. Call build_mapping() first.")
            return None

        # Find first mapped character from start
        rendered_start = None
        for pos in range(original_start, min(original_end, len(self.original_text))):
            if pos in self.position_map:
                rendered_start = self.position_map[pos]
                break

        # Find last mapped character before end
        rendered_end = None
        for pos in range(original_end - 1, max(original_start - 1, -1), -1):
            if pos in self.position_map:
                rendered_end = self.position_map[pos] + 1
                break

        if rendered_start is None or rendered_end is None:
            logger.debug(
                "No mapping found for range [%d, %d) in original text",
                original_start,
                original_end,
            )
            return None

        return (rendered_start, rendered_end)

    def _html_to_plain(self, html: str) -> str:
        """Convert HTML to plain text using Qt.

        Uses QTextDocument.toPlainText() to match exactly how Qt renders HTML.
        This properly handles tables, entities, and other HTML structures.

        Args:
            html: HTML string

        Returns:
            Plain text as Qt would render it
        """
        from PyQt6.QtGui import QTextDocument

        doc = QTextDocument()
        doc.setHtml(html)
        return doc.toPlainText()

    def _build_position_map(self) -> None:
        """Build exact character-level position mapping using sequence alignment.

        Uses difflib.SequenceMatcher to align original Markdown text with rendered
        plain text. This provides precise position mapping without search or tracking
        used ranges.

        Markdown syntax (**, #, `, etc.) appears as 'delete' operations in alignment,
        while matching text creates direct character-level mappings.
        """
        from difflib import SequenceMatcher

        # autojunk=False is critical: with True (default), characters appearing
        # 1+len(b)//100 times are treated as junk, causing misalignment after
        # large deleted sections (e.g. URLs in Markdown links)
        matcher = SequenceMatcher(None, self.original_text, self.rendered_plain, autojunk=False)

        for tag, i1, i2, j1, j2 in matcher.get_opcodes():
            if tag == "equal":
                # Characters match exactly - create direct mapping
                for k in range(i2 - i1):
                    self.position_map[i1 + k] = j1 + k
            elif tag == "delete":
                # Markdown syntax removed during rendering - no mapping needed
                pass
            elif tag == "replace":
                # Characters changed (e.g., HTML entities decoded by Qt)
                # Map them proportionally for rough alignment
                orig_len = i2 - i1
                rend_len = j2 - j1
                if orig_len > 0 and rend_len > 0:
                    for k in range(orig_len):
                        # Proportional mapping for replaced sections
                        rend_offset = int(k * rend_len / orig_len)
                        self.position_map[i1 + k] = j1 + rend_offset
            elif tag == "insert":
                # Text inserted in rendered (shouldn't happen for Markdown)
                # This might occur if Qt adds something during rendering
                pass

        logger.debug(
            "Built position map with %d mappings using sequence alignment",
            len(self.position_map),
        )
