"""Tests for word mapping functionality."""

import pytest
from ruvox.tts_pipeline.word_mapping import (
    WordSpan,
    WordMapping,
    tokenize_words,
    build_word_mapping,
    _is_possible_transliteration,
)
from ruvox.tts_pipeline import TTSPipeline


class TestTokenizeWords:
    """Tests for tokenize_words function."""

    def test_simple_text(self):
        """Test tokenizing simple text."""
        words = tokenize_words("Hello world")
        assert len(words) == 2
        assert words[0].text == "Hello"
        assert words[0].start == 0
        assert words[0].end == 5
        assert words[1].text == "world"
        assert words[1].start == 6
        assert words[1].end == 11

    def test_russian_text(self):
        """Test tokenizing Russian text."""
        words = tokenize_words("Привет мир")
        assert len(words) == 2
        assert words[0].text == "Привет"
        assert words[1].text == "мир"

    def test_mixed_text(self):
        """Test tokenizing mixed Russian/English text."""
        words = tokenize_words("Вызови getUserData через API")
        assert len(words) == 4
        assert words[0].text == "Вызови"
        assert words[1].text == "getUserData"
        assert words[2].text == "через"
        assert words[3].text == "API"

    def test_empty_text(self):
        """Test tokenizing empty text."""
        words = tokenize_words("")
        assert len(words) == 0

    def test_punctuation(self):
        """Test that punctuation is excluded."""
        words = tokenize_words("Hello, world!")
        assert len(words) == 2
        assert words[0].text == "Hello"
        assert words[1].text == "world"

    def test_numbers(self):
        """Test that numbers are included as words."""
        words = tokenize_words("There are 42 items")
        assert len(words) == 4
        assert words[2].text == "42"


class TestWordMapping:
    """Tests for WordMapping class."""

    def test_get_original_range_simple(self):
        """Test getting original range for simple case."""
        mapping = WordMapping(
            original_text="Hello world",
            transformed_text="привет мир",
            original_words=[
                WordSpan("Hello", 0, 5),
                WordSpan("world", 6, 11),
            ],
            transformed_words=[
                WordSpan("привет", 0, 6),
                WordSpan("мир", 7, 10),
            ],
            word_map={0: (0, 1), 1: (1, 2)},
        )

        # First word
        result = mapping.get_original_range_for_word(0)
        assert result == (0, 5)

        # Second word
        result = mapping.get_original_range_for_word(1)
        assert result == (6, 11)

    def test_get_original_range_for_position(self):
        """Test getting original range for character position."""
        mapping = WordMapping(
            original_text="Hello world",
            transformed_text="привет мир",
            original_words=[
                WordSpan("Hello", 0, 5),
                WordSpan("world", 6, 11),
            ],
            transformed_words=[
                WordSpan("привет", 0, 6),
                WordSpan("мир", 7, 10),
            ],
            word_map={0: (0, 1), 1: (1, 2)},
        )

        # Position in first word
        result = mapping.get_original_range_for_position(2, 4)
        assert result == (0, 5)

        # Position in second word
        result = mapping.get_original_range_for_position(7, 9)
        assert result == (6, 11)

    def test_out_of_range(self):
        """Test handling out of range indices."""
        mapping = WordMapping(
            original_text="Hello",
            transformed_text="привет",
            original_words=[WordSpan("Hello", 0, 5)],
            transformed_words=[WordSpan("привет", 0, 6)],
            word_map={0: (0, 1)},
        )

        result = mapping.get_original_range_for_word(10)
        assert result is None


class TestBuildWordMapping:
    """Tests for build_word_mapping function."""

    def test_identity_mapping(self):
        """Test mapping for text without transformation."""
        original = "Привет мир"
        transformed = "Привет мир"
        mapping = build_word_mapping(original, transformed)

        assert mapping.original_text == original
        assert mapping.transformed_text == transformed
        assert len(mapping.original_words) == 2
        assert len(mapping.transformed_words) == 2

    def test_simple_transformation(self):
        """Test mapping for simple transformation."""
        original = "Hello world"
        transformed = "хелло ворлд"
        mapping = build_word_mapping(original, transformed)

        # Both should have 2 words
        assert len(mapping.original_words) == 2
        assert len(mapping.transformed_words) == 2

        # First word should map to first word
        orig_range = mapping.get_original_range_for_word(0)
        assert orig_range is not None
        assert original[orig_range[0]:orig_range[1]] == "Hello"

    def test_camel_case_expansion(self):
        """Test mapping for camelCase expansion."""
        original = "getUserData"
        transformed = "гет юзер дата"
        mapping = build_word_mapping(original, transformed)

        # Original has 1 word, transformed has 3
        assert len(mapping.original_words) == 1
        assert len(mapping.transformed_words) == 3

        # All transformed words should map to the single original word
        for i in range(3):
            orig_range = mapping.get_original_range_for_word(i)
            assert orig_range is not None
            # Should be within the original word
            assert orig_range[0] >= 0
            assert orig_range[1] <= len(original)

    def test_mixed_text(self):
        """Test mapping for mixed Russian/English text."""
        original = "Вызови getUserData через API"
        transformed = "Вызови гет юзер дата через эй пи ай"
        mapping = build_word_mapping(original, transformed)

        # Check that Russian words map correctly
        # "Вызови" should map to "Вызови"
        first_range = mapping.get_original_range_for_word(0)
        assert first_range is not None
        assert original[first_range[0]:first_range[1]] == "Вызови"


class TestIsPossibleTransliteration:
    """Tests for transliteration detection."""

    def test_obvious_transliterations(self):
        """Test detection of obvious transliterations."""
        assert _is_possible_transliteration("hello", "хелло")
        assert _is_possible_transliteration("get", "гет")
        assert _is_possible_transliteration("feature", "фича")

    def test_non_transliterations(self):
        """Test rejection of non-transliterations."""
        assert not _is_possible_transliteration("hello", "мир")
        assert not _is_possible_transliteration("abc", "")
        assert not _is_possible_transliteration("", "abc")


class TestPipelineWithMapping:
    """Tests for TTSPipeline.process_with_mapping method."""

    @pytest.fixture
    def pipeline(self):
        return TTSPipeline()

    def test_process_with_mapping_returns_tuple(self, pipeline):
        """Test that process_with_mapping returns tuple."""
        result = pipeline.process_with_mapping("Hello world")
        assert isinstance(result, tuple)
        assert len(result) == 2
        assert isinstance(result[0], str)
        assert isinstance(result[1], WordMapping)

    def test_mapping_for_simple_text(self, pipeline):
        """Test mapping for simple Russian text."""
        original = "Привет мир"
        transformed, mapping = pipeline.process_with_mapping(original)

        # Text should be mostly unchanged
        assert "Привет" in transformed or "привет" in transformed.lower()

        # Mapping should have words
        assert len(mapping.original_words) >= 2
        assert len(mapping.transformed_words) >= 2

    def test_mapping_for_english_text(self, pipeline):
        """Test mapping for English text."""
        original = "Hello feature"
        transformed, mapping = pipeline.process_with_mapping(original)

        # Should be transliterated
        assert transformed != original

        # Mapping should work
        orig_range = mapping.get_original_range_for_word(0)
        assert orig_range is not None

    def test_mapping_for_code_identifier(self, pipeline):
        """Test mapping for code identifier."""
        original = "Вызови getUserData"
        transformed, mapping = pipeline.process_with_mapping(original)

        # getUserData should be expanded
        assert "гет" in transformed.lower() or "юзер" in transformed.lower()

        # First word should map correctly
        first_range = mapping.get_original_range_for_word(0)
        assert first_range is not None
        assert original[first_range[0]:first_range[1]] == "Вызови"

    def test_mapping_for_numbers(self, pipeline):
        """Test mapping for numbers."""
        original = "Осталось 42 дня"
        transformed, mapping = pipeline.process_with_mapping(original)

        # Number should be expanded to words
        assert "42" not in transformed
        assert "сорок" in transformed.lower() or "два" in transformed.lower()

        # Mapping should still work
        assert len(mapping.word_map) > 0
