"""Text mapping between original and transformed text.

This module provides tracking of text transformations to map positions
in the transformed (TTS-ready) text back to the original text.
"""

from dataclasses import dataclass, field


@dataclass
class TextSpan:
    """A span mapping original text position to transformed text position.

    Attributes:
        orig_start: Start position in original text
        orig_end: End position in original text (exclusive)
        trans_start: Start position in transformed text
        trans_end: End position in transformed text (exclusive)
    """

    orig_start: int
    orig_end: int
    trans_start: int
    trans_end: int

    @property
    def orig_length(self) -> int:
        return self.orig_end - self.orig_start

    @property
    def trans_length(self) -> int:
        return self.trans_end - self.trans_start


@dataclass
class TextMapping:
    """Mapping between original and transformed text.

    Tracks all transformations applied to text and provides methods
    to map positions between original and transformed text.
    """

    original: str
    transformed: str = ""
    spans: list[TextSpan] = field(default_factory=list)

    def get_original_range(self, trans_start: int, trans_end: int) -> tuple[int, int]:
        """Map a range in transformed text back to original text.

        Args:
            trans_start: Start position in transformed text
            trans_end: End position in transformed text

        Returns:
            Tuple of (orig_start, orig_end) in original text
        """
        if not self.spans:
            # No mapping, return same positions (clamped to original length)
            return (
                min(trans_start, len(self.original)),
                min(trans_end, len(self.original)),
            )

        # Find spans that overlap with the given range
        orig_start = None
        orig_end = None

        for span in self.spans:
            # Check if this span overlaps with the query range
            if span.trans_end <= trans_start:
                # Span is before our range
                continue
            if span.trans_start >= trans_end:
                # Span is after our range, we're done
                break

            # Span overlaps with our range
            if orig_start is None:
                orig_start = span.orig_start
            orig_end = span.orig_end

        if orig_start is None:
            # No overlapping spans found, try to interpolate
            return self._interpolate_position(trans_start, trans_end)

        return (orig_start, orig_end)

    def _interpolate_position(self, trans_start: int, trans_end: int) -> tuple[int, int]:
        """Interpolate position when no exact span match found."""
        if not self.spans:
            return (trans_start, trans_end)

        # Find the closest span before trans_start
        prev_span = None
        next_span = None

        for span in self.spans:
            if span.trans_end <= trans_start:
                prev_span = span
            elif span.trans_start >= trans_end and next_span is None:
                next_span = span
                break

        # Interpolate based on surrounding spans
        if prev_span is not None and next_span is not None:
            # Linear interpolation between spans
            trans_gap = next_span.trans_start - prev_span.trans_end
            orig_gap = next_span.orig_start - prev_span.orig_end

            if trans_gap > 0:
                ratio = (trans_start - prev_span.trans_end) / trans_gap
                start = prev_span.orig_end + int(ratio * orig_gap)
                ratio_end = (trans_end - prev_span.trans_end) / trans_gap
                end = prev_span.orig_end + int(ratio_end * orig_gap)
                return (start, min(end, len(self.original)))

        if prev_span is not None:
            # After last known span
            offset = trans_start - prev_span.trans_end
            start = min(prev_span.orig_end + offset, len(self.original))
            end = min(start + (trans_end - trans_start), len(self.original))
            return (start, end)

        if next_span is not None:
            # Before first known span
            offset = next_span.trans_start - trans_end
            end = max(next_span.orig_start - offset, 0)
            start = max(end - (trans_end - trans_start), 0)
            return (start, end)

        # Fallback: return clamped positions
        return (
            min(trans_start, len(self.original)),
            min(trans_end, len(self.original)),
        )

    def get_original_word_at(self, trans_pos: int) -> tuple[int, int]:
        """Get the original word boundaries for a position in transformed text.

        This finds the word in the original text that corresponds to
        the given position in the transformed text.

        Args:
            trans_pos: Position in transformed text

        Returns:
            Tuple of (word_start, word_end) in original text
        """
        # First map the position
        orig_start, orig_end = self.get_original_range(trans_pos, trans_pos + 1)

        # Expand to word boundaries in original text
        word_start = orig_start
        word_end = orig_end

        # Find word start (go back until whitespace or start)
        while word_start > 0 and not self.original[word_start - 1].isspace():
            word_start -= 1

        # Find word end (go forward until whitespace or end)
        while word_end < len(self.original) and not self.original[word_end].isspace():
            word_end += 1

        return (word_start, word_end)


class MappingBuilder:
    """Builder for constructing TextMapping during text transformation.

    Tracks replacements as they occur and builds the final mapping.
    """

    def __init__(self, original: str):
        self.original = original
        self._current_text = original
        self._offset = 0  # Cumulative offset from replacements
        self._replacements: list[tuple[int, int, int, int]] = []
        # Each tuple: (orig_start, orig_end, trans_start, trans_end)

    def replace(self, start: int, end: int, replacement: str) -> str:
        """Record a replacement and update tracking.

        Args:
            start: Start position in current text
            end: End position in current text
            replacement: Replacement string

        Returns:
            The new text after replacement
        """
        # Calculate original positions
        orig_start = self._to_original_pos(start)
        orig_end = self._to_original_pos(end)

        # Calculate transformed positions
        trans_start = start
        trans_end = start + len(replacement)

        # Record the replacement
        self._replacements.append((orig_start, orig_end, trans_start, trans_end))

        # Update current text
        self._current_text = (
            self._current_text[:start] + replacement + self._current_text[end:]
        )

        # Update offset for subsequent replacements
        length_diff = len(replacement) - (end - start)
        self._offset += length_diff

        return self._current_text

    def _to_original_pos(self, current_pos: int) -> int:
        """Convert a position in current text to position in original text."""
        # Account for all previous replacements
        pos = current_pos
        for orig_start, orig_end, trans_start, trans_end in self._replacements:
            if trans_end <= current_pos:
                # This replacement is before our position
                # Adjust by the difference in lengths
                pos += (orig_end - orig_start) - (trans_end - trans_start)
        return pos

    def get_current_text(self) -> str:
        """Get the current transformed text."""
        return self._current_text

    def build(self) -> TextMapping:
        """Build the final TextMapping.

        Returns:
            TextMapping with all recorded transformations
        """
        spans = []
        for orig_start, orig_end, trans_start, trans_end in self._replacements:
            spans.append(
                TextSpan(
                    orig_start=orig_start,
                    orig_end=orig_end,
                    trans_start=trans_start,
                    trans_end=trans_end,
                )
            )

        # Sort spans by transformed position
        spans.sort(key=lambda s: s.trans_start)

        return TextMapping(
            original=self.original,
            transformed=self._current_text,
            spans=spans,
        )


def create_identity_mapping(text: str) -> TextMapping:
    """Create an identity mapping (no transformation).

    Args:
        text: The text (same for original and transformed)

    Returns:
        TextMapping where positions map 1:1
    """
    return TextMapping(
        original=text,
        transformed=text,
        spans=[TextSpan(0, len(text), 0, len(text))],
    )
