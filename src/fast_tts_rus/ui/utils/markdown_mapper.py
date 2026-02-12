"""Markdown position mapper for text highlighting.

Maps character positions between original Markdown text and rendered plain text,
enabling accurate word highlighting when displaying Markdown with formatting.
"""

import logging
import re
from xml.etree import ElementTree as ET

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

        # Extract text chunks from ElementTree
        # Note: parser.root may not exist for empty documents
        if not hasattr(md_instance.parser, "root"):
            logger.debug("Parser has no root element (empty document)")
            return html

        root = md_instance.parser.root
        text_chunks = self._extract_text_chunks(root)

        # Build position mapping
        self._build_position_map(text_chunks)

        return html

    def get_rendered_range(
        self, original_start: int, original_end: int
    ) -> tuple[int, int] | None:
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
        """Convert HTML to plain text by removing tags.

        Simulates QTextDocument.toPlainText() behavior.

        Args:
            html: HTML string

        Returns:
            Plain text with HTML tags removed
        """
        # Remove all HTML tags
        plain = re.sub(r"<[^>]+>", "", html)
        return plain.strip()

    def _extract_text_chunks(self, root: ET.Element) -> list[str]:
        """Extract text chunks from ElementTree in document order.

        Only extracts text that will be visible in the rendered output,
        ignoring Markdown syntax elements.

        Args:
            root: Root element of the parsed Markdown tree

        Returns:
            List of text chunks in document order
        """
        chunks: list[str] = []

        def walk(elem: ET.Element) -> None:
            # Text before first child
            if elem.text:
                text = elem.text.strip()
                if text:
                    chunks.append(text)

            # Process children recursively
            for child in elem:
                walk(child)
                # Text after child (tail)
                if child.tail:
                    text = child.tail.strip()
                    if text:
                        chunks.append(text)

        walk(root)
        return chunks

    def _build_position_map(self, text_chunks: list[str]) -> None:
        """Build character-level position mapping.

        Maps each character position in original text to corresponding position
        in rendered plain text. Handles multiple occurrences of the same text chunk.

        If text_chunks from ElementTree are insufficient (e.g., code blocks use
        placeholders), falls back to word-level matching.

        Args:
            text_chunks: List of text chunks extracted from rendered document
        """
        rendered_pos = 0
        used_original_ranges: list[tuple[int, int]] = []

        for chunk in text_chunks:
            # Find chunk in rendered plain text
            rendered_idx = self.rendered_plain.find(chunk, rendered_pos)
            if rendered_idx == -1:
                logger.debug("Chunk %r not found in rendered text", chunk)
                continue

            # Find first unused occurrence in original text
            original_idx = self._find_unused_occurrence(chunk, used_original_ranges)
            if original_idx == -1:
                logger.debug("Chunk %r not found in original text", chunk)
                continue

            # Map each character in the chunk
            for i in range(len(chunk)):
                self.position_map[original_idx + i] = rendered_idx + i

            # Mark this range as used
            used_original_ranges.append((original_idx, original_idx + len(chunk)))
            rendered_pos = rendered_idx + len(chunk)

        # Fallback: if mapping is sparse, add word-level mapping
        # This handles cases like code blocks where ElementTree uses placeholders
        if len(self.position_map) < len(self.rendered_plain) * 0.5:
            logger.debug("Sparse mapping detected, adding word-level fallback")
            self._add_word_level_mapping(used_original_ranges)

        logger.debug(
            "Built position map with %d mappings for %d chunks",
            len(self.position_map),
            len(text_chunks),
        )

    def _add_word_level_mapping(self, used_ranges: list[tuple[int, int]]) -> None:
        """Add word-level mapping as fallback for unmapped regions.

        Extracts words from both original and rendered text and maps them.
        Useful for code blocks and other special content.

        Args:
            used_ranges: Already mapped ranges to avoid duplicates
        """
        # Extract words from rendered plain
        rendered_words = list(re.finditer(r'\b\w+\b', self.rendered_plain))

        # Track rendered position
        for rendered_match in rendered_words:
            rendered_word = rendered_match.group()
            rendered_start = rendered_match.start()

            # Check if this position is already mapped
            if any(rendered_start in self.position_map.values() for _ in range(len(rendered_word))):
                continue

            # Find this word in original (first unused occurrence)
            original_idx = self._find_unused_occurrence(rendered_word, used_ranges)
            if original_idx == -1:
                continue

            # Map each character
            for i in range(len(rendered_word)):
                if original_idx + i not in self.position_map:
                    self.position_map[original_idx + i] = rendered_start + i

            used_ranges.append((original_idx, original_idx + len(rendered_word)))

    def _find_unused_occurrence(
        self, chunk: str, used_ranges: list[tuple[int, int]]
    ) -> int:
        """Find first occurrence of chunk not in used ranges.

        Args:
            chunk: Text to find
            used_ranges: List of (start, end) ranges already used

        Returns:
            Start position of chunk, or -1 if not found
        """
        search_pos = 0
        while True:
            idx = self.original_text.find(chunk, search_pos)
            if idx == -1:
                return -1

            # Check if this position overlaps with used ranges
            chunk_end = idx + len(chunk)
            is_used = any(
                # Check for overlap: chunk overlaps with range if:
                # chunk_start < range_end AND chunk_end > range_start
                idx < range_end and chunk_end > range_start
                for range_start, range_end in used_ranges
            )

            if not is_used:
                return idx

            search_pos = idx + 1

        return -1
