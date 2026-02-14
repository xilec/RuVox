"""Tests for MarkdownPositionMapper."""

import pytest
from PyQt6.QtWidgets import QApplication

from ruvox.ui.utils.markdown_mapper import MarkdownPositionMapper



class TestMarkdownPositionMapper:
    """Test suite for MarkdownPositionMapper."""

    def test_plain_text_no_markdown(self, qapp):
        """Plain text without Markdown should have 1:1 mapping."""
        original = "Simple plain text"
        mapper = MarkdownPositionMapper(original)
        mapper.build_mapping()

        assert mapper.rendered_plain == original

        # Test mapping for each word
        result = mapper.get_rendered_range(0, 6)  # "Simple"
        assert result == (0, 6)

        result = mapper.get_rendered_range(7, 12)  # "plain"
        assert result == (7, 12)

        result = mapper.get_rendered_range(13, 17)  # "text"
        assert result == (13, 17)

    def test_bold_text(self, qapp):
        """Bold text should map correctly, removing ** markers."""
        original = "Some **bold** text"
        mapper = MarkdownPositionMapper(original)
        mapper.build_mapping()

        assert mapper.rendered_plain == "Some bold text"

        # "bold" at original[7:11]
        result = mapper.get_rendered_range(7, 11)
        assert result == (5, 9)

    def test_header(self, qapp):
        """Header should map correctly, removing # marker."""
        original = "# Header"
        mapper = MarkdownPositionMapper(original)
        mapper.build_mapping()

        assert mapper.rendered_plain == "Header"

        # "Header" at original[2:8]
        result = mapper.get_rendered_range(2, 8)
        assert result == (0, 6)

    def test_inline_code(self, qapp):
        """Inline code should map correctly, removing ` markers."""
        original = "Run `command` now"
        mapper = MarkdownPositionMapper(original)
        mapper.build_mapping()

        assert mapper.rendered_plain == "Run command now"

        # "command" at original[5:12]
        result = mapper.get_rendered_range(5, 12)
        assert result == (4, 11)

    def test_link(self, qapp):
        """Link should map correctly, extracting only visible text."""
        original = "[link text](https://example.com)"
        mapper = MarkdownPositionMapper(original)
        mapper.build_mapping()

        assert mapper.rendered_plain == "link text"

        # "link text" at original[1:10]
        result = mapper.get_rendered_range(1, 10)
        assert result == (0, 9)

    def test_multiple_bold(self, qapp):
        """Multiple bold sections should map independently."""
        original = "**first** and **second**"
        mapper = MarkdownPositionMapper(original)
        mapper.build_mapping()

        assert mapper.rendered_plain == "first and second"

        # "first" at original[2:7]
        result = mapper.get_rendered_range(2, 7)
        assert result == (0, 5)

        # "second" at original[16:22]
        result = mapper.get_rendered_range(16, 22)
        assert result == (10, 16)

    def test_repeated_words(self, qapp):
        """Repeated words should map to correct occurrences."""
        original = "word **word** word"
        mapper = MarkdownPositionMapper(original)
        mapper.build_mapping()

        assert mapper.rendered_plain == "word word word"

        # First "word" at original[0:4]
        result = mapper.get_rendered_range(0, 4)
        assert result == (0, 4)

        # Second "word" (in bold) at original[7:11]
        result = mapper.get_rendered_range(7, 11)
        assert result == (5, 9)

        # Third "word" at original[14:18]
        result = mapper.get_rendered_range(14, 18)
        assert result == (10, 14)

    def test_mixed_content(self, qapp):
        """Mixed Markdown content should map correctly."""
        original = "# Header\nSome **bold** text and `code`"
        mapper = MarkdownPositionMapper(original)
        mapper.build_mapping()

        assert "Header" in mapper.rendered_plain
        assert "bold" in mapper.rendered_plain
        assert "code" in mapper.rendered_plain

        # "Header" at original[2:8]
        result = mapper.get_rendered_range(2, 8)
        assert result is not None
        assert mapper.rendered_plain[result[0] : result[1]] == "Header"

        # "bold" at original[16:20]
        result = mapper.get_rendered_range(16, 20)
        assert result is not None
        assert mapper.rendered_plain[result[0] : result[1]] == "bold"

        # "code" at original[33:37]
        result = mapper.get_rendered_range(33, 37)
        assert result is not None
        assert mapper.rendered_plain[result[0] : result[1]] == "code"

    def test_list_items(self, qapp):
        """List items should map correctly."""
        original = "- item 1\n- item 2"
        mapper = MarkdownPositionMapper(original)
        mapper.build_mapping()

        # Both "item" words should be in rendered text
        assert mapper.rendered_plain.count("item") == 2

        # First "item" at original[2:6]
        result = mapper.get_rendered_range(2, 6)
        assert result is not None
        assert mapper.rendered_plain[result[0] : result[1]] == "item"

    def test_code_block(self, qapp):
        """Code block content should map correctly."""
        original = "```python\nprint('hello')\n```"
        mapper = MarkdownPositionMapper(original)
        html = mapper.build_mapping()

        # "print" should be in rendered text
        assert "print" in mapper.rendered_plain

        # Find "print" in original (after ```python\n)
        print_start = original.find("print")
        print_end = print_start + 5

        result = mapper.get_rendered_range(print_start, print_end)
        # With fallback word-level mapping, this should work
        assert result is not None, f"Mapping not found for 'print' at [{print_start}:{print_end}]"

        rendered_word = mapper.rendered_plain[result[0] : result[1]]
        # May include some extra characters due to word boundaries
        assert "print" in rendered_word or rendered_word == "print"

    def test_table(self, qapp):
        """Table content should map correctly."""
        original = "| A | B |\n|---|---|\n| 1 | 2 |"
        mapper = MarkdownPositionMapper(original)
        mapper.build_mapping()

        # All values should be in rendered text
        assert "A" in mapper.rendered_plain
        assert "B" in mapper.rendered_plain
        assert "1" in mapper.rendered_plain
        assert "2" in mapper.rendered_plain

    def test_empty_text(self, qapp):
        """Empty text should not cause errors."""
        mapper = MarkdownPositionMapper("")
        html = mapper.build_mapping()

        assert mapper.rendered_plain == ""
        assert html == ""

    def test_only_markdown_syntax(self, qapp):
        """Text with only Markdown syntax should handle gracefully."""
        original = "**  **"
        mapper = MarkdownPositionMapper(original)
        mapper.build_mapping()

        # Should be empty or whitespace after rendering
        assert len(mapper.rendered_plain.strip()) == 0

    def test_unicode_text(self, qapp):
        """Unicode text should map correctly."""
        original = "Текст с **кириллицей** и 中文"
        mapper = MarkdownPositionMapper(original)
        mapper.build_mapping()

        assert "кириллицей" in mapper.rendered_plain
        assert "中文" in mapper.rendered_plain

        # Find "кириллицей" in original
        word = "кириллицей"
        start = original.find(word)
        end = start + len(word)

        result = mapper.get_rendered_range(start, end)
        assert result is not None
        assert mapper.rendered_plain[result[0] : result[1]] == word

    def test_mapping_not_found(self, qapp):
        """Positions outside text should return None."""
        original = "Some text"
        mapper = MarkdownPositionMapper(original)
        mapper.build_mapping()

        # Range completely outside text
        result = mapper.get_rendered_range(100, 110)
        assert result is None

    def test_partial_mapping(self, qapp):
        """Range partially in Markdown syntax should find available mapping."""
        original = "text **bold** more"
        mapper = MarkdownPositionMapper(original)
        mapper.build_mapping()

        # Range includes ** marker at start: [5:11] = "**bold"
        # Should find mapping for "bold" part
        result = mapper.get_rendered_range(5, 11)
        assert result is not None
        # Should map to "bold" in rendered text
        assert "bold" in mapper.rendered_plain[result[0] : result[1]]

    def test_nested_formatting(self, qapp):
        """Nested formatting should map correctly."""
        original = "**bold _italic_ bold**"
        mapper = MarkdownPositionMapper(original)
        mapper.build_mapping()

        # "italic" inside bold
        italic_start = original.find("italic")
        italic_end = italic_start + 6

        result = mapper.get_rendered_range(italic_start, italic_end)
        assert result is not None
        assert mapper.rendered_plain[result[0] : result[1]] == "italic"

    def test_multiline_text(self, qapp):
        """Multiline text should preserve line breaks and map correctly."""
        original = "First line\n**Second** line\nThird line"
        mapper = MarkdownPositionMapper(original)
        mapper.build_mapping()

        # "Second" in bold
        second_start = original.find("Second")
        second_end = second_start + 6

        result = mapper.get_rendered_range(second_start, second_end)
        assert result is not None
        assert mapper.rendered_plain[result[0] : result[1]] == "Second"

    def test_real_scenario_with_timestamps(self, qapp):
        """Test real scenario matching timestamps from TTS pipeline."""
        # This simulates the actual use case: timestamps contain positions
        # of actual words (without Markdown markers)
        original = "# Header\nSome **bold** text and `code`"

        # Positions of actual words (as TTSPipeline would provide via CharMapping)
        words_with_positions = [
            ("Header", 2, 8),  # "Header" without "#"
            ("Some", 9, 13),  # "Some"
            ("bold", 16, 20),  # "bold" without "**"
            ("text", 23, 27),  # "text"
            ("and", 28, 31),  # "and"
            ("code", 33, 37),  # "code" without "`"
        ]

        mapper = MarkdownPositionMapper(original)
        mapper.build_mapping()

        # All words should map correctly
        for word, start, end in words_with_positions:
            result = mapper.get_rendered_range(start, end)
            assert result is not None, f"Mapping not found for {word!r}"

            rendered_word = mapper.rendered_plain[result[0] : result[1]]
            assert (
                rendered_word == word
            ), f"Expected {word!r}, got {rendered_word!r}"

    def test_single_char_variables(self, qapp):
        """Test repeated single-character variables (like x, y) in code."""
        # Regression test for issue with single-char variable highlighting
        original = """```
p.x += 1
p.x = p.y
```"""

        mapper = MarkdownPositionMapper(original)
        mapper.build_mapping()

        # Find all 'x' positions in original
        x_positions = [i for i, char in enumerate(original) if char == 'x']
        assert len(x_positions) == 2, f"Expected 2 'x' chars, found {len(x_positions)}"

        # Both 'x' should map correctly to different positions in rendered
        results = []
        for x_pos in x_positions:
            result = mapper.get_rendered_range(x_pos, x_pos + 1)
            assert result is not None, f"No mapping for 'x' at position {x_pos}"
            rend_start, rend_end = result
            rendered_char = mapper.rendered_plain[rend_start:rend_end]
            assert rendered_char == 'x', f"Expected 'x', got {rendered_char!r}"
            results.append((rend_start, rend_end))

        # Positions should be different (not both mapping to the same 'x')
        assert results[0] != results[1], "Both 'x' chars map to the same position!"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
