"""Tracked text for precise position mapping during transformations.

This module provides a TrackedText class that wraps text and tracks
all modifications, allowing precise mapping between original and
transformed positions.
"""

import re
from dataclasses import dataclass, field
from typing import Callable, Match


@dataclass
class Replacement:
    """A single replacement operation."""

    orig_start: int  # Start position in original text
    orig_end: int  # End position in original text
    orig_text: str  # Original text that was replaced
    new_text: str  # New text after replacement


@dataclass
class CharMapping:
    """Character-level mapping from transformed to original positions.

    For each character in transformed text, stores the corresponding
    range in original text.
    """

    original: str
    transformed: str
    # For each position in transformed, (orig_start, orig_end)
    # If a transformed char maps to multiple original chars, orig_end > orig_start + 1
    # If multiple transformed chars map to one original char, they share the same range
    char_map: list[tuple[int, int]] = field(default_factory=list)

    def get_original_range(self, trans_start: int, trans_end: int) -> tuple[int, int]:
        """Map a range in transformed text to original text.

        Args:
            trans_start: Start position in transformed text
            trans_end: End position in transformed text

        Returns:
            Tuple of (orig_start, orig_end)
        """
        if not self.char_map:
            return (trans_start, trans_end)

        # Clamp to valid range
        trans_start = max(0, min(trans_start, len(self.char_map) - 1))
        trans_end = max(0, min(trans_end, len(self.char_map)))

        if trans_start >= len(self.char_map):
            # Position is past end of text
            if self.char_map:
                return (self.char_map[-1][1], self.char_map[-1][1])
            return (len(self.original), len(self.original))

        # Get the range covered by all characters in the transformed range
        orig_start = self.char_map[trans_start][0]
        orig_end = self.char_map[trans_start][1]

        for i in range(trans_start + 1, min(trans_end, len(self.char_map))):
            orig_start = min(orig_start, self.char_map[i][0])
            orig_end = max(orig_end, self.char_map[i][1])

        return (orig_start, orig_end)

    def get_original_word_range(self, trans_pos: int) -> tuple[int, int]:
        """Get word boundaries in original text for a position in transformed.

        Args:
            trans_pos: Position in transformed text

        Returns:
            Tuple of (word_start, word_end) in original text
        """
        orig_start, orig_end = self.get_original_range(trans_pos, trans_pos + 1)

        # Expand to word boundaries
        word_start = orig_start
        word_end = orig_end

        # Find word start
        while word_start > 0 and not self.original[word_start - 1].isspace():
            word_start -= 1

        # Find word end
        while word_end < len(self.original) and not self.original[word_end].isspace():
            word_end += 1

        return (word_start, word_end)


@dataclass
class OffsetEntry:
    """Tracks a single replacement for offset calculation."""
    current_pos: int  # Position in current text at time of replacement
    orig_start: int   # Start position in original text
    orig_end: int     # End position in original text
    new_len: int      # Length of replacement text


class TrackedText:
    """Text wrapper that tracks all modifications for position mapping.

    Usage:
        tracked = TrackedText("Hello getUserData")
        tracked.sub(r'getUserData', 'гет юзер дата')
        mapping = tracked.build_mapping()
        # mapping.get_original_range(6, 9) -> position of "getUserData"
    """

    def __init__(self, text: str):
        self.original = text
        self._current = text
        self._replacements: list[Replacement] = []
        # Offset tracking: ordered list of replacements with positions
        self._offset_entries: list[OffsetEntry] = []
        self._sorted_entries_cache: list[OffsetEntry] | None = None

    def _get_sorted_entries(self) -> list[OffsetEntry]:
        if self._sorted_entries_cache is None:
            self._sorted_entries_cache = sorted(self._offset_entries, key=lambda e: e.orig_start)
        return self._sorted_entries_cache

    @property
    def text(self) -> str:
        """Get current text."""
        return self._current

    def sub(
        self,
        pattern: str | re.Pattern,
        repl: str | Callable[[Match], str],
        count: int = 0,
        flags: int = 0,
    ) -> "TrackedText":
        """Perform regex substitution with tracking.

        Args:
            pattern: Regex pattern
            repl: Replacement string or function
            count: Maximum replacements (0 = unlimited)
            flags: Regex flags

        Returns:
            self for chaining
        """
        if isinstance(pattern, str):
            pattern = re.compile(pattern, flags)

        # Find all matches first
        matches = list(pattern.finditer(self._current))
        if count > 0:
            matches = matches[:count]

        # Process matches in reverse order to maintain positions in current text
        for match in reversed(matches):
            start, end = match.start(), match.end()
            old_text = match.group()

            # Get replacement text
            if callable(repl):
                new_text = repl(match)
            else:
                new_text = match.expand(repl)

            # First check: does any character in the match fall inside a replacement?
            # This catches cross-boundary matches (e.g., space from original + space from replacement)
            match_touches_replacement = False
            for pos in range(start, end):
                if self._is_current_pos_inside_replacement(pos):
                    match_touches_replacement = True
                    break

            if match_touches_replacement:
                # This match includes characters from inside a replacement
                # Skip to avoid creating cross-boundary replacements
                continue

            # Calculate original positions using current offset entries
            orig_start = self._current_to_original(start)
            orig_end = self._current_to_original(end)

            # Check if this original range overlaps with an existing replacement
            # We check in ORIGINAL coordinates because current positions shift
            # as we make more replacements
            containing_entry = self._find_containing_replacement(orig_start, orig_end)

            if containing_entry is not None:
                # This change is inside an existing replacement
                # Skip entirely - the original replacement already covers this region
                # Note: this means words inside replaced text won't be further normalized,
                # but it keeps the mapping consistent
                continue

            # No overlap - record as new replacement
            self._replacements.append(
                Replacement(
                    orig_start=orig_start,
                    orig_end=orig_end,
                    orig_text=old_text,
                    new_text=new_text,
                )
            )

            # Record offset entry (insert at beginning since we process in reverse)
            self._offset_entries.insert(0, OffsetEntry(
                current_pos=start,
                orig_start=orig_start,
                orig_end=orig_end,
                new_len=len(new_text),
            ))

            self._sorted_entries_cache = None

            # Update text
            self._current = self._current[:start] + new_text + self._current[end:]

        return self

    def _is_current_pos_inside_replacement(self, current_pos: int) -> bool:
        """Check if a position in current text is inside a replacement's new text.

        This is checked in CURRENT text coordinates, computing where each
        replacement is located after all previous replacements.

        Returns:
            True if the position falls inside any replacement's new text.
        """
        sorted_entries = self._get_sorted_entries()
        cumulative_delta = 0

        for entry in sorted_entries:
            old_len = entry.orig_end - entry.orig_start
            new_len = entry.new_len
            delta = new_len - old_len

            # Where this replacement is in current text
            current_start = entry.orig_start + cumulative_delta
            current_end = current_start + new_len

            if current_pos < current_start:
                # Position is before this replacement - not inside any
                return False
            elif current_pos < current_end:
                # Position is inside this replacement
                return True
            else:
                # Position is after this replacement
                cumulative_delta += delta

        return False

    def _find_containing_replacement(
        self, orig_start: int, orig_end: int
    ) -> OffsetEntry | None:
        """Find an existing replacement that contains or overlaps the given range.

        This is checked in ORIGINAL text coordinates, not current text coordinates,
        because current positions shift as more replacements are made.

        Returns:
            The OffsetEntry that contains this range, or None if no overlap.
        """
        for entry in self._offset_entries:
            if orig_start == orig_end:
                # Point range - check if it's inside an existing replacement
                # A point at position P is inside [A, B) if A <= P < B
                if entry.orig_start <= orig_start < entry.orig_end:
                    return entry
            else:
                # Normal range - check if ranges overlap
                # Two ranges [a, b) and [c, d) overlap if a < d and c < b
                if orig_start < entry.orig_end and entry.orig_start < orig_end:
                    return entry
        return None

    def replace(self, old: str, new: str, count: int = -1) -> "TrackedText":
        """Perform string replacement with tracking.

        Args:
            old: String to replace
            new: Replacement string
            count: Maximum replacements (-1 = unlimited)

        Returns:
            self for chaining
        """
        # Escape special regex characters
        pattern = re.escape(old)
        max_count = 0 if count < 0 else count
        return self.sub(pattern, new, count=max_count)

    def _current_to_original(self, current_pos: int) -> int:
        """Convert position in current text to position in original text.

        Uses offset entries sorted by original position to correctly map
        current positions back to original positions.
        """
        # Sort entries by original start position (stable reference)
        sorted_entries = self._get_sorted_entries()

        # Compute where each replacement is in current text
        # by accumulating deltas from all previous replacements
        cumulative_delta = 0
        for entry in sorted_entries:
            old_len = entry.orig_end - entry.orig_start
            new_len = entry.new_len
            delta = new_len - old_len

            # Where this replacement is in current text
            current_start = entry.orig_start + cumulative_delta
            current_end = current_start + new_len

            if current_pos < current_start:
                # Position is before this replacement
                # Subtract cumulative delta to get original position
                return current_pos - cumulative_delta
            elif current_pos < current_end:
                # Position is INSIDE this replacement
                # Map to the start of the original range
                return entry.orig_start
            else:
                # Position is after this replacement
                cumulative_delta += delta

        # Position is after all replacements
        return current_pos - cumulative_delta

    def build_mapping(self) -> CharMapping:
        """Build character-level mapping from tracked replacements.

        Returns:
            CharMapping with position mappings
        """
        if not self._replacements:
            # No changes - identity mapping
            char_map = [(i, i + 1) for i in range(len(self._current))]
            return CharMapping(
                original=self.original,
                transformed=self._current,
                char_map=char_map,
            )

        # Sort replacements by original position
        sorted_repls = sorted(self._replacements, key=lambda r: r.orig_start)

        # Build character map
        new_char_map: list[tuple[int, int]] = []
        orig_idx = 0

        for repl in sorted_repls:
            # Copy unchanged characters before this replacement
            while orig_idx < repl.orig_start:
                new_char_map.append((orig_idx, orig_idx + 1))
                orig_idx += 1

            # Add mapping for replacement
            # All characters in new_text map to the original range
            for _ in range(len(repl.new_text)):
                new_char_map.append((repl.orig_start, repl.orig_end))

            orig_idx = repl.orig_end

        # Copy remaining unchanged characters
        while orig_idx < len(self.original):
            new_char_map.append((orig_idx, orig_idx + 1))
            orig_idx += 1

        return CharMapping(
            original=self.original,
            transformed=self._current,
            char_map=new_char_map,
        )


def create_tracked_text(text: str) -> TrackedText:
    """Create a new TrackedText instance.

    Args:
        text: Original text

    Returns:
        TrackedText instance
    """
    return TrackedText(text)
