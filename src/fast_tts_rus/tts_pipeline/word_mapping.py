"""Word-level mapping between original and transformed text.

Simpler approach than character-level mapping: tracks which words
in the transformed text correspond to which words in the original text.
"""

import re
from dataclasses import dataclass, field


@dataclass
class WordSpan:
    """A word with its position in text."""

    text: str
    start: int
    end: int

    def __repr__(self):
        return f"WordSpan({self.text!r}, {self.start}:{self.end})"


@dataclass
class WordMapping:
    """Mapping between words in original and transformed text.

    Each entry maps a range of words in transformed text to a range
    of words in the original text.
    """

    original_text: str
    transformed_text: str
    original_words: list[WordSpan] = field(default_factory=list)
    transformed_words: list[WordSpan] = field(default_factory=list)
    # Mapping: transformed word index -> (orig_start_idx, orig_end_idx)
    word_map: dict[int, tuple[int, int]] = field(default_factory=dict)

    def get_original_range_for_word(self, trans_word_idx: int) -> tuple[int, int] | None:
        """Get character range in original text for a transformed word.

        Args:
            trans_word_idx: Index of word in transformed text

        Returns:
            Tuple of (start, end) character positions in original text,
            or None if word index is out of range
        """
        if trans_word_idx < 0 or trans_word_idx >= len(self.transformed_words):
            return None

        # Check if we have explicit mapping
        if trans_word_idx in self.word_map:
            orig_start_idx, orig_end_idx = self.word_map[trans_word_idx]
            if orig_start_idx < len(self.original_words) and orig_end_idx <= len(
                self.original_words
            ):
                start = self.original_words[orig_start_idx].start
                end = self.original_words[orig_end_idx - 1].end
                return (start, end)

        # Fallback: use proportional mapping
        ratio = trans_word_idx / max(len(self.transformed_words), 1)
        orig_idx = int(ratio * len(self.original_words))
        orig_idx = min(orig_idx, len(self.original_words) - 1)

        if orig_idx >= 0 and orig_idx < len(self.original_words):
            word = self.original_words[orig_idx]
            return (word.start, word.end)

        return None

    def get_original_range_for_position(
        self, trans_start: int, trans_end: int
    ) -> tuple[int, int] | None:
        """Get character range in original text for a position range in transformed text.

        Args:
            trans_start: Start position in transformed text
            trans_end: End position in transformed text

        Returns:
            Tuple of (start, end) character positions in original text
        """
        # Find which word(s) the position falls into
        start_word_idx = None
        end_word_idx = None

        for i, word in enumerate(self.transformed_words):
            if word.start <= trans_start < word.end:
                start_word_idx = i
            if word.start < trans_end <= word.end:
                end_word_idx = i
            if start_word_idx is not None and end_word_idx is not None:
                break

        if start_word_idx is None:
            # Position is before first word or between words
            # Find nearest word
            for i, word in enumerate(self.transformed_words):
                if word.start >= trans_start:
                    start_word_idx = i
                    break
            if start_word_idx is None:
                start_word_idx = len(self.transformed_words) - 1

        if end_word_idx is None:
            end_word_idx = start_word_idx

        # Get original range for these words
        start_range = self.get_original_range_for_word(start_word_idx)
        end_range = self.get_original_range_for_word(end_word_idx)

        if start_range and end_range:
            return (start_range[0], end_range[1])
        elif start_range:
            return start_range
        elif end_range:
            return end_range

        return None


def tokenize_words(text: str) -> list[WordSpan]:
    """Tokenize text into words with positions.

    Args:
        text: Text to tokenize

    Returns:
        List of WordSpan objects
    """
    words = []
    # Match word characters (letters, digits, underscores) and apostrophes
    for match in re.finditer(r"[\w']+", text, re.UNICODE):
        words.append(WordSpan(text=match.group(), start=match.start(), end=match.end()))
    return words


def build_word_mapping(
    original: str, transformed: str, pipeline=None
) -> WordMapping:
    """Build word mapping between original and transformed text.

    This uses heuristics to match words:
    1. Exact matches
    2. Prefix/suffix matches
    3. Proportional position fallback

    Args:
        original: Original text
        transformed: Transformed text
        pipeline: Optional TTSPipeline for normalization hints

    Returns:
        WordMapping object
    """
    orig_words = tokenize_words(original)
    trans_words = tokenize_words(transformed)

    mapping = WordMapping(
        original_text=original,
        transformed_text=transformed,
        original_words=orig_words,
        transformed_words=trans_words,
    )

    if not orig_words or not trans_words:
        return mapping

    # Build mapping using alignment heuristics
    word_map = {}

    # Track which original words have been mapped
    orig_used = [False] * len(orig_words)
    orig_idx = 0

    for trans_idx, trans_word in enumerate(trans_words):
        trans_lower = trans_word.text.lower()

        # Try to find matching original word starting from current position
        best_match = None
        best_score = 0

        # Look ahead in original words for a match
        for i in range(orig_idx, min(orig_idx + 10, len(orig_words))):
            if orig_used[i]:
                continue

            orig_word = orig_words[i]
            orig_lower = orig_word.text.lower()

            score = 0

            # Exact match
            if trans_lower == orig_lower:
                score = 100
            # Original word starts with transformed (getUserData -> гет, юзер, дата)
            elif orig_lower.startswith(trans_lower[:3]) if len(trans_lower) >= 3 else False:
                score = 50
            # Transformed is transliteration of original (check first letters)
            elif _is_possible_transliteration(orig_lower, trans_lower):
                score = 40
            # Position-based fallback
            elif i == orig_idx:
                score = 10

            if score > best_score:
                best_score = score
                best_match = i

        if best_match is not None:
            word_map[trans_idx] = (best_match, best_match + 1)
            orig_used[best_match] = True
            # Move orig_idx forward
            while orig_idx < len(orig_words) and orig_used[orig_idx]:
                orig_idx += 1
        else:
            # No match found, use proportional position
            ratio = trans_idx / len(trans_words)
            fallback_idx = int(ratio * len(orig_words))
            fallback_idx = min(fallback_idx, len(orig_words) - 1)
            word_map[trans_idx] = (fallback_idx, fallback_idx + 1)

    mapping.word_map = word_map
    return mapping


def _is_possible_transliteration(english: str, russian: str) -> bool:
    """Check if russian could be transliteration of english.

    Simple heuristic based on common transliteration patterns.
    """
    if not english or not russian:
        return False

    # Common first-letter transliterations
    first_letter_map = {
        'a': 'а', 'b': 'б', 'c': 'с', 'd': 'д', 'e': 'е',
        'f': 'ф', 'g': 'г', 'h': 'х', 'i': 'и', 'j': 'дж',
        'k': 'к', 'l': 'л', 'm': 'м', 'n': 'н', 'o': 'о',
        'p': 'п', 'q': 'к', 'r': 'р', 's': 'с', 't': 'т',
        'u': 'у', 'v': 'в', 'w': 'в', 'x': 'кс', 'y': 'й',
        'z': 'з',
    }

    eng_first = english[0].lower()
    rus_first = russian[0].lower()

    expected_rus = first_letter_map.get(eng_first, '')
    return rus_first == expected_rus or (len(expected_rus) > 1 and russian.lower().startswith(expected_rus))
