"""Tests for TrackedText character-level mapping."""

import pytest
from fast_tts_rus.tts_pipeline.tracked_text import (
    TrackedText,
    CharMapping,
    Replacement,
    create_tracked_text,
)


class TestTrackedTextBasic:
    """Basic tests for TrackedText."""

    def test_no_changes(self):
        """Text without changes should have identity mapping."""
        tracked = TrackedText("Hello world")
        mapping = tracked.build_mapping()

        assert mapping.original == "Hello world"
        assert mapping.transformed == "Hello world"
        assert len(mapping.char_map) == 11
        # Each char maps to itself
        for i in range(11):
            assert mapping.char_map[i] == (i, i + 1)

    def test_simple_replace(self):
        """Test simple string replacement."""
        tracked = TrackedText("Hello world")
        tracked.replace("world", "мир")

        assert tracked.text == "Hello мир"
        mapping = tracked.build_mapping()

        assert mapping.original == "Hello world"
        assert mapping.transformed == "Hello мир"

    def test_simple_sub(self):
        """Test regex substitution."""
        tracked = TrackedText("Hello world")
        tracked.sub(r"world", "мир")

        assert tracked.text == "Hello мир"

    def test_chaining(self):
        """Test method chaining."""
        tracked = TrackedText("Hello world")
        result = tracked.replace("Hello", "Привет").replace("world", "мир")

        assert result is tracked
        assert tracked.text == "Привет мир"


class TestCharMapping:
    """Tests for CharMapping position resolution."""

    def test_simple_replacement_mapping(self):
        """Test mapping for simple word replacement."""
        tracked = TrackedText("Hello world")
        tracked.replace("world", "мир")
        mapping = tracked.build_mapping()

        # "Hello " stays same (0-6)
        for i in range(6):
            assert mapping.char_map[i] == (i, i + 1)

        # "мир" maps to original "world" (6-11)
        for i in range(6, 9):  # 3 chars in "мир"
            assert mapping.char_map[i] == (6, 11)

    def test_get_original_range_unchanged(self):
        """Test getting original range for unchanged text."""
        tracked = TrackedText("Hello world")
        mapping = tracked.build_mapping()

        result = mapping.get_original_range(0, 5)
        assert result == (0, 5)

    def test_get_original_range_replaced(self):
        """Test getting original range for replaced text."""
        tracked = TrackedText("Hello world")
        tracked.replace("world", "мир")
        mapping = tracked.build_mapping()

        # Position in "мир" should map to "world"
        result = mapping.get_original_range(6, 9)
        assert result == (6, 11)

    def test_get_original_range_spanning(self):
        """Test getting range spanning replacement boundary."""
        tracked = TrackedText("Hello world")
        tracked.replace("world", "мир")
        mapping = tracked.build_mapping()

        # Range spanning "o " and "м" should cover both original ranges
        result = mapping.get_original_range(4, 7)
        assert result[0] == 4  # starts at 'o'
        assert result[1] == 11  # ends at "world"


class TestMultipleReplacements:
    """Tests for multiple replacements."""

    def test_two_replacements(self):
        """Test two separate replacements."""
        tracked = TrackedText("Hello world")
        tracked.replace("Hello", "Привет")
        tracked.replace("world", "мир")

        assert tracked.text == "Привет мир"
        mapping = tracked.build_mapping()

        # "Привет" (6 chars) maps to "Hello" (5 chars, pos 0-5)
        for i in range(6):
            assert mapping.char_map[i] == (0, 5)

        # Space maps to space
        assert mapping.char_map[6] == (5, 6)

        # "мир" maps to "world" (pos 6-11)
        for i in range(7, 10):
            assert mapping.char_map[i] == (6, 11)

    def test_expanding_replacement(self):
        """Test replacement that expands text."""
        tracked = TrackedText("12345")
        tracked.replace("12345", "двенадцать тысяч триста сорок пять")

        mapping = tracked.build_mapping()

        # All chars in long text map to short original
        for i in range(len(mapping.transformed)):
            assert mapping.char_map[i] == (0, 5)

    def test_contracting_replacement(self):
        """Test replacement that contracts text."""
        tracked = TrackedText("getUserData")
        tracked.replace("getUserData", "гет")

        mapping = tracked.build_mapping()

        # "гет" maps to full "getUserData"
        for i in range(3):
            assert mapping.char_map[i] == (0, 11)


class TestRegexSubstitution:
    """Tests for regex substitution with tracking."""

    def test_regex_with_groups(self):
        """Test regex substitution with capture groups."""
        tracked = TrackedText("test_value_here")
        tracked.sub(r"_", " ")

        assert tracked.text == "test value here"

    def test_regex_callback(self):
        """Test regex substitution with callback function."""
        tracked = TrackedText("hello WORLD")
        tracked.sub(r"[A-Z]+", lambda m: m.group().lower())

        assert tracked.text == "hello world"

    def test_limited_count(self):
        """Test replacement with count limit."""
        tracked = TrackedText("a_b_c_d")
        tracked.sub(r"_", " ", count=2)

        assert tracked.text == "a b c_d"


class TestWordBoundaryMapping:
    """Tests for word boundary resolution."""

    def test_get_original_word_range(self):
        """Test getting word boundaries from position."""
        tracked = TrackedText("Hello world test")
        mapping = tracked.build_mapping()

        # Position in "world" should return "world" boundaries
        word_start, word_end = mapping.get_original_word_range(6)
        assert word_start == 6
        assert word_end == 11

    def test_get_word_range_after_replacement(self):
        """Test word boundaries after replacement."""
        tracked = TrackedText("Hello world")
        tracked.replace("world", "мир")
        mapping = tracked.build_mapping()

        # Position in "мир" should return "world" boundaries
        word_start, word_end = mapping.get_original_word_range(7)
        assert word_start == 6
        assert word_end == 11


class TestEdgeCases:
    """Edge case tests."""

    def test_empty_text(self):
        """Test with empty text."""
        tracked = TrackedText("")
        mapping = tracked.build_mapping()

        assert mapping.original == ""
        assert mapping.transformed == ""
        assert len(mapping.char_map) == 0

    def test_replacement_at_start(self):
        """Test replacement at text start."""
        tracked = TrackedText("Hello world")
        tracked.replace("Hello", "Привет")
        mapping = tracked.build_mapping()

        assert mapping.transformed == "Привет world"
        assert mapping.char_map[0] == (0, 5)

    def test_replacement_at_end(self):
        """Test replacement at text end."""
        tracked = TrackedText("Hello world")
        tracked.replace("world", "мир")
        mapping = tracked.build_mapping()

        assert mapping.transformed == "Hello мир"
        assert mapping.char_map[-1] == (6, 11)

    def test_adjacent_replacements(self):
        """Test adjacent replacements."""
        tracked = TrackedText("AB")
        tracked.replace("A", "1")
        tracked.replace("B", "2")
        mapping = tracked.build_mapping()

        assert mapping.transformed == "12"
        assert mapping.char_map[0] == (0, 1)
        assert mapping.char_map[1] == (1, 2)

    def test_position_past_end(self):
        """Test position past end of text."""
        tracked = TrackedText("Hello")
        mapping = tracked.build_mapping()

        result = mapping.get_original_range(10, 15)
        assert result == (4, 5)  # Last char position


class TestRealWorldCases:
    """Tests with real-world TTS transformation examples."""

    def test_number_expansion(self):
        """Test number to words expansion."""
        tracked = TrackedText("Осталось 42 дня")
        tracked.sub(r"\b42\b", "сорок два")

        mapping = tracked.build_mapping()

        # "сорок два" should map back to "42"
        # Find position of "42" in original: "Осталось " = 9 chars, "42" at 9-11
        # In transformed: "Осталось " = 9 chars, "сорок два" = 9 chars
        for i in range(9, 18):  # "сорок два"
            assert mapping.char_map[i] == (9, 11)

    def test_code_identifier(self):
        """Test code identifier transliteration."""
        tracked = TrackedText("Вызови getUserData")
        tracked.sub(r"getUserData", "гет юзер дата")

        mapping = tracked.build_mapping()

        # "гет юзер дата" maps to "getUserData"
        original_start = 7  # "Вызови " = 7 chars
        original_end = 18  # "getUserData" = 11 chars

        for i in range(7, 7 + 13):  # "гет юзер дата"
            assert mapping.char_map[i] == (original_start, original_end)

    def test_multiple_transformations(self):
        """Test multiple sequential transformations."""
        tracked = TrackedText("Test 123 API")
        tracked.sub(r"\b123\b", "сто двадцать три")
        tracked.sub(r"\bAPI\b", "эй пи ай")

        mapping = tracked.build_mapping()

        assert "сто двадцать три" in mapping.transformed
        assert "эй пи ай" in mapping.transformed


class TestCreateTrackedText:
    """Tests for factory function."""

    def test_create_tracked_text(self):
        """Test factory function."""
        tracked = create_tracked_text("Hello world")

        assert isinstance(tracked, TrackedText)
        assert tracked.text == "Hello world"
        assert tracked.original == "Hello world"


class TestPipelineIntegration:
    """Integration tests with TTSPipeline.process_with_char_mapping."""

    def test_simple_text_mapping(self):
        """Test char mapping for simple text."""
        from fast_tts_rus.tts_pipeline import TTSPipeline

        pipeline = TTSPipeline()
        original = "Hello world"
        transformed, mapping = pipeline.process_with_char_mapping(original)

        assert mapping.original == original
        assert "хелло" in transformed.lower() or "ворлд" in transformed.lower()

    def test_number_expansion_mapping(self):
        """Test that number expansion works correctly.

        Note: CharMapping precision is limited for multi-step pipelines.
        The UI uses word-level matching instead for timestamp mapping.
        """
        from fast_tts_rus.tts_pipeline import TTSPipeline

        pipeline = TTSPipeline()
        original = "Осталось 42 дня"
        transformed, mapping = pipeline.process_with_char_mapping(original)

        # "42" should be expanded to "сорок два"
        assert "42" not in transformed
        assert "сорок" in transformed.lower() or "два" in transformed.lower()

        # Verify original text is preserved
        assert mapping.original == original

    def test_code_identifier_mapping(self):
        """Test code identifier expansion maps correctly."""
        from fast_tts_rus.tts_pipeline import TTSPipeline

        pipeline = TTSPipeline()
        original = "Вызови getUserData"
        transformed, mapping = pipeline.process_with_char_mapping(original)

        # "getUserData" should be expanded
        assert "getUserData" not in transformed
        assert "гет" in transformed.lower() or "юзер" in transformed.lower()

        # "Вызови" should remain and map to itself
        if "Вызови" in transformed:
            vyzovi_pos = transformed.find("Вызови")
            orig_range = mapping.get_original_range(vyzovi_pos, vyzovi_pos + 6)
            assert orig_range == (0, 6)

    def test_problematic_text_mapping(self):
        """Test mapping for text that previously had issues."""
        from fast_tts_rus.tts_pipeline import TTSPipeline

        pipeline = TTSPipeline()
        original = "Сейчас приоритетнее разобраться с ошибкой, почему в контекстном меню 3 команды читать сразу и читать отложено. Выдают аж ошибку, что оффер пустой."
        transformed, mapping = pipeline.process_with_char_mapping(original)

        # "оффер" should remain (it's Russian)
        assert "оффер" in transformed.lower() or "оффэр" in transformed.lower()

        # Number "3" should be expanded
        assert "три" in transformed.lower()

        # Find "три" and check it maps to "3"
        tri_pos = transformed.lower().find("три")
        if tri_pos >= 0:
            orig_range = mapping.get_original_range(tri_pos, tri_pos + 3)
            # Should be near position 69 where "3" is in original
            assert orig_range[0] >= 60 and orig_range[1] <= 80

    def test_english_clipboard_mapping(self):
        """Test mapping for English text with numbers."""
        from fast_tts_rus.tts_pipeline import TTSPipeline

        pipeline = TTSPipeline()
        original = "Test clipboard content 12345"
        transformed, mapping = pipeline.process_with_char_mapping(original)

        # English words should be transliterated
        assert "Test" not in transformed
        assert "clipboard" not in transformed

        # Number should be expanded
        assert "12345" not in transformed

    def test_word_boundary_from_char_mapping(self):
        """Test that word boundaries can be extracted from char mapping."""
        from fast_tts_rus.tts_pipeline import TTSPipeline

        pipeline = TTSPipeline()
        original = "Hello world"
        transformed, mapping = pipeline.process_with_char_mapping(original)

        # Get word boundary for position in transformed text
        if len(mapping.char_map) > 0:
            word_start, word_end = mapping.get_original_word_range(0)
            # Should be "Hello" boundaries
            assert word_start == 0
            assert original[word_start:word_end] in ["Hello", "hello"]
