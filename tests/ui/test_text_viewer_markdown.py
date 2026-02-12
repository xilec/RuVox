"""Tests for TextViewerWidget Markdown mode with position mapping."""

import pytest
from PyQt6.QtWidgets import QApplication

from fast_tts_rus.ui.widgets.text_viewer import TextViewerWidget, TextFormat
from fast_tts_rus.ui.models.entry import TextEntry, EntryStatus


@pytest.fixture(scope="module")
def qapp():
    """Create QApplication instance for tests."""
    app = QApplication.instance()
    if app is None:
        app = QApplication([])
    yield app


@pytest.fixture
def text_viewer(qapp):
    """Create TextViewerWidget instance."""
    viewer = TextViewerWidget()
    yield viewer
    viewer.deleteLater()


class TestMarkdownRendering:
    """Test Markdown rendering with position mapping."""

    def test_plain_mode_renders_as_plain(self, text_viewer):
        """Plain mode should render text without HTML."""
        entry = TextEntry(original_text="Some **bold** text")
        text_viewer.set_format(TextFormat.PLAIN)
        text_viewer.set_entry(entry)

        # Should render as plain text (with Markdown markers)
        rendered = text_viewer.toPlainText()
        assert "**bold**" in rendered
        assert text_viewer._markdown_mapper is None

    def test_markdown_mode_renders_as_html(self, text_viewer):
        """Markdown mode should render with HTML formatting."""
        entry = TextEntry(original_text="Some **bold** text")
        text_viewer.set_format(TextFormat.MARKDOWN)
        text_viewer.set_entry(entry)

        # Should render without Markdown markers
        rendered = text_viewer.toPlainText()
        assert "**" not in rendered
        assert "bold" in rendered
        assert text_viewer._markdown_mapper is not None

    def test_mapper_built_on_render(self, text_viewer):
        """Position mapper should be built when rendering Markdown."""
        entry = TextEntry(original_text="# Header\nSome text")
        text_viewer.set_format(TextFormat.MARKDOWN)
        text_viewer.set_entry(entry)

        assert text_viewer._markdown_mapper is not None
        assert len(text_viewer._markdown_mapper.position_map) > 0

    def test_mapper_cleared_on_plain_mode(self, text_viewer):
        """Position mapper should be cleared in plain mode."""
        entry = TextEntry(original_text="Some **bold** text")

        # First render in Markdown mode
        text_viewer.set_format(TextFormat.MARKDOWN)
        text_viewer.set_entry(entry)
        assert text_viewer._markdown_mapper is not None

        # Switch to plain mode
        text_viewer.set_format(TextFormat.PLAIN)
        assert text_viewer._markdown_mapper is None


class TestHighlightingInMarkdownMode:
    """Test word highlighting with Markdown position mapping."""

    def test_highlight_bold_text(self, text_viewer):
        """Highlighting should work for bold text in Markdown mode."""
        original = "Some **bold** text"
        entry = TextEntry(original_text=original)

        # Timestamps with positions in original text
        timestamps = [
            {"word": "Some", "start": 0.0, "end": 0.5, "original_pos": [0, 4]},
            {"word": "bold", "start": 0.5, "end": 1.0, "original_pos": [7, 11]},
            {"word": "text", "start": 1.0, "end": 1.5, "original_pos": [14, 18]},
        ]

        text_viewer.set_format(TextFormat.MARKDOWN)
        text_viewer.set_entry(entry, timestamps)

        # Highlight "bold" word
        text_viewer.highlight_at_position(0.7)

        # Check that highlight was applied
        assert text_viewer._last_highlighted_pos == (7, 11)
        assert text_viewer._last_highlight_doc_range is not None

    def test_highlight_header(self, text_viewer):
        """Highlighting should work for header text."""
        original = "# Header\nSome text"
        entry = TextEntry(original_text=original)

        timestamps = [
            {"word": "Header", "start": 0.0, "end": 0.5, "original_pos": [2, 8]},
            {"word": "Some", "start": 0.5, "end": 1.0, "original_pos": [9, 13]},
        ]

        text_viewer.set_format(TextFormat.MARKDOWN)
        text_viewer.set_entry(entry, timestamps)

        # Highlight "Header"
        text_viewer.highlight_at_position(0.2)

        assert text_viewer._last_highlighted_pos == (2, 8)
        assert text_viewer._last_highlight_doc_range is not None

    def test_highlight_code(self, text_viewer):
        """Highlighting should work for inline code."""
        original = "Run `command` now"
        entry = TextEntry(original_text=original)

        timestamps = [
            {"word": "Run", "start": 0.0, "end": 0.3, "original_pos": [0, 3]},
            {"word": "command", "start": 0.3, "end": 0.8, "original_pos": [5, 12]},
            {"word": "now", "start": 0.8, "end": 1.0, "original_pos": [14, 17]},
        ]

        text_viewer.set_format(TextFormat.MARKDOWN)
        text_viewer.set_entry(entry, timestamps)

        # Highlight "command"
        text_viewer.highlight_at_position(0.5)

        assert text_viewer._last_highlighted_pos == (5, 12)
        assert text_viewer._last_highlight_doc_range is not None

    def test_highlight_link_text(self, text_viewer):
        """Highlighting should work for link text (URL hidden)."""
        original = "[link text](https://example.com)"
        entry = TextEntry(original_text=original)

        timestamps = [
            {"word": "link", "start": 0.0, "end": 0.5, "original_pos": [1, 5]},
            {"word": "text", "start": 0.5, "end": 1.0, "original_pos": [6, 10]},
        ]

        text_viewer.set_format(TextFormat.MARKDOWN)
        text_viewer.set_entry(entry, timestamps)

        # Highlight "link"
        text_viewer.highlight_at_position(0.2)

        assert text_viewer._last_highlighted_pos == (1, 5)
        # Should find mapping even though URL is hidden
        assert text_viewer._last_highlight_doc_range is not None

    def test_highlight_repeated_words(self, text_viewer):
        """Highlighting should work for repeated words."""
        original = "word **word** word"
        entry = TextEntry(original_text=original)

        timestamps = [
            {"word": "word", "start": 0.0, "end": 0.3, "original_pos": [0, 4]},
            {"word": "word", "start": 0.3, "end": 0.6, "original_pos": [7, 11]},
            {"word": "word", "start": 0.6, "end": 0.9, "original_pos": [14, 18]},
        ]

        text_viewer.set_format(TextFormat.MARKDOWN)
        text_viewer.set_entry(entry, timestamps)

        # Highlight first occurrence
        text_viewer.highlight_at_position(0.1)
        assert text_viewer._last_highlighted_pos == (0, 4)

        # Highlight second occurrence (in bold)
        text_viewer.highlight_at_position(0.4)
        assert text_viewer._last_highlighted_pos == (7, 11)

        # Highlight third occurrence
        text_viewer.highlight_at_position(0.7)
        assert text_viewer._last_highlighted_pos == (14, 18)

    def test_switch_format_during_playback(self, text_viewer):
        """Switching format during playback should work correctly."""
        original = "Some **bold** text"
        entry = TextEntry(original_text=original)

        timestamps = [
            {"word": "bold", "start": 0.5, "end": 1.0, "original_pos": [7, 11]},
        ]

        # Start in Markdown mode
        text_viewer.set_format(TextFormat.MARKDOWN)
        text_viewer.set_entry(entry, timestamps)
        text_viewer.highlight_at_position(0.7)

        assert text_viewer._markdown_mapper is not None
        markdown_pos = text_viewer._last_highlighted_pos
        assert markdown_pos == (7, 11)

        # Switch to plain mode
        text_viewer.set_format(TextFormat.PLAIN)

        # Mapper should be cleared when switching to plain
        assert text_viewer._markdown_mapper is None

        # Highlight again in plain mode
        text_viewer.highlight_at_position(0.7)

        # Position should be the same in original_pos
        assert text_viewer._last_highlighted_pos == (7, 11)
        # In plain mode, doc range should match (1:1 mapping)
        # Both should work without errors
        assert text_viewer._last_highlight_doc_range is not None


class TestComplexMarkdown:
    """Test highlighting with complex Markdown content."""

    def test_mixed_formatting(self, text_viewer):
        """Test with multiple Markdown elements."""
        original = "# Title\n\nSome **bold** and `code` text"
        entry = TextEntry(original_text=original)

        timestamps = [
            {"word": "Title", "start": 0.0, "end": 0.3, "original_pos": [2, 7]},
            {"word": "bold", "start": 0.5, "end": 0.8, "original_pos": [17, 21]},
            {"word": "code", "start": 1.0, "end": 1.3, "original_pos": [28, 32]},
        ]

        text_viewer.set_format(TextFormat.MARKDOWN)
        text_viewer.set_entry(entry, timestamps)

        # All highlights should work
        for ts in timestamps:
            text_viewer.highlight_at_position(ts["start"] + 0.1)
            assert text_viewer._last_highlighted_pos == tuple(ts["original_pos"])

    def test_multiline_content(self, text_viewer):
        """Test with multiline Markdown."""
        original = "First line\n**Second** line\nThird line"
        entry = TextEntry(original_text=original)

        timestamps = [
            {"word": "First", "start": 0.0, "end": 0.3, "original_pos": [0, 5]},
            {"word": "Second", "start": 0.5, "end": 0.8, "original_pos": [13, 19]},
            {"word": "Third", "start": 1.0, "end": 1.3, "original_pos": [27, 32]},
        ]

        text_viewer.set_format(TextFormat.MARKDOWN)
        text_viewer.set_entry(entry, timestamps)

        # All lines should highlight correctly
        for ts in timestamps:
            text_viewer.highlight_at_position(ts["start"] + 0.1)
            assert text_viewer._last_highlighted_pos == tuple(ts["original_pos"])


class TestEdgeCases:
    """Test edge cases and error handling."""

    def test_empty_text(self, text_viewer):
        """Empty text should not cause errors."""
        entry = TextEntry(original_text="")
        text_viewer.set_format(TextFormat.MARKDOWN)
        text_viewer.set_entry(entry, [])

        # Should handle gracefully
        rendered = text_viewer.toPlainText()
        assert rendered == ""

    def test_no_timestamps(self, text_viewer):
        """Highlighting without timestamps should not crash."""
        entry = TextEntry(original_text="Some text")
        text_viewer.set_format(TextFormat.MARKDOWN)
        text_viewer.set_entry(entry, None)

        # Should not crash
        text_viewer.highlight_at_position(0.5)

    def test_invalid_position(self, text_viewer):
        """Invalid timestamp position should be handled."""
        entry = TextEntry(original_text="Some text")
        timestamps = [
            {"word": "Some", "start": 0.0, "end": 0.5, "original_pos": [100, 104]},
        ]

        text_viewer.set_format(TextFormat.MARKDOWN)
        text_viewer.set_entry(entry, timestamps)

        # Should handle gracefully
        text_viewer.highlight_at_position(0.2)
        # Highlight may fail, but should not crash

    def test_clear_entry_clears_mapper(self, text_viewer):
        """Clearing entry should clear the mapper."""
        entry = TextEntry(original_text="Some **bold** text")
        text_viewer.set_format(TextFormat.MARKDOWN)
        text_viewer.set_entry(entry)

        assert text_viewer._markdown_mapper is not None

        text_viewer.clear_entry()

        assert text_viewer._markdown_mapper is None
        assert text_viewer.current_entry is None


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
