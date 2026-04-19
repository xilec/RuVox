"""Tests for Mermaid integration in TextViewerWidget."""

from unittest.mock import MagicMock, patch

import pytest
from PyQt6.QtCore import QUrl

from ruvox.ui.models.entry import TextEntry
from ruvox.ui.widgets.text_viewer import TextFormat, TextViewerWidget

SAMPLE_MERMAID = "graph TD\n  A --> B"

SAMPLE_TEXT_WITH_MERMAID = f"""# Title

Some text before.

```mermaid
{SAMPLE_MERMAID}
```

Some text after.
"""


@pytest.fixture
def text_viewer(qapp):
    """Create TextViewerWidget instance."""
    viewer = TextViewerWidget()
    yield viewer
    viewer.deleteLater()


class TestExtractMermaidBlocks:
    """Test mermaid block extraction from Markdown text."""

    def test_no_mermaid_blocks(self, text_viewer):
        """Text without mermaid blocks returns empty list."""
        text = "# Hello\n\nSome text"
        result_text, blocks = text_viewer._extract_mermaid_blocks(text)
        assert blocks == []
        assert result_text == text

    def test_single_mermaid_block(self, text_viewer):
        """Single mermaid block is extracted correctly."""
        text = "Before\n\n```mermaid\ngraph TD\n  A --> B\n```\n\nAfter"
        result_text, blocks = text_viewer._extract_mermaid_blocks(text)
        assert len(blocks) == 1
        assert blocks[0] == "graph TD\n  A --> B"
        assert "<!--MERMAID:0-->" in result_text
        assert "```mermaid" not in result_text

    def test_multiple_mermaid_blocks(self, text_viewer):
        """Multiple mermaid blocks are all extracted."""
        text = "```mermaid\ngraph TD\n  A --> B\n```\nMiddle text\n```mermaid\nsequenceDiagram\n  A->>B: Hello\n```"
        result_text, blocks = text_viewer._extract_mermaid_blocks(text)
        assert len(blocks) == 2
        assert blocks[0] == "graph TD\n  A --> B"
        assert blocks[1] == "sequenceDiagram\n  A->>B: Hello"
        assert "<!--MERMAID:0-->" in result_text
        assert "<!--MERMAID:1-->" in result_text

    def test_non_mermaid_code_blocks_preserved(self, text_viewer):
        """Non-mermaid code blocks are NOT extracted."""
        text = "```python\nprint('hello')\n```\n\n```mermaid\ngraph TD\n  A\n```"
        result_text, blocks = text_viewer._extract_mermaid_blocks(text)
        assert len(blocks) == 1
        assert "```python" in result_text
        assert "<!--MERMAID:0-->" in result_text


class TestMermaidPlaceholdersInHtml:
    """Test placeholder injection when SVG is not cached."""

    def test_placeholder_shown_when_no_cache(self, text_viewer):
        """When no SVG is cached, a text placeholder link is shown."""
        text_viewer._mermaid_renderer = None  # no renderer = no cache
        html = "<p><!--MERMAID:0--></p>"
        blocks = ["graph TD\n  A --> B"]
        result = text_viewer._inject_mermaid_images(html, blocks)
        assert "mermaid-placeholder" in result
        assert 'href="mermaid:0"' in result
        assert "загружается" in result

    def test_image_shown_when_cached(self, text_viewer):
        """When SVG is cached, an img tag is shown."""
        mock_renderer = MagicMock()
        mock_renderer.get_cached_svg.return_value = "<svg>test</svg>"
        text_viewer._mermaid_renderer = mock_renderer

        html = "<p><!--MERMAID:0--></p>"
        blocks = ["graph TD\n  A --> B"]
        result = text_viewer._inject_mermaid_images(html, blocks)
        assert "mermaid-diagram" in result
        assert 'src="mermaid-img:0"' in result
        assert 'href="mermaid:0"' in result


class TestAnchorClicked:
    """Test anchor click handling."""

    def test_mermaid_anchor_opens_preview(self, text_viewer):
        """Clicking mermaid:N URL opens MermaidPreviewDialog."""
        text_viewer._mermaid_blocks = [SAMPLE_MERMAID]

        with patch.object(text_viewer, "_show_mermaid_preview") as mock:
            text_viewer._on_anchor_clicked(QUrl("mermaid:0"))
            mock.assert_called_once_with(0)

    def test_http_anchor_opens_browser(self, text_viewer):
        """Clicking http:// URL opens in default browser."""
        with patch("ruvox.ui.widgets.text_viewer.QDesktopServices.openUrl") as mock:
            url = QUrl("https://example.com")
            text_viewer._on_anchor_clicked(url)
            mock.assert_called_once_with(url)

    def test_invalid_mermaid_index_ignored(self, text_viewer):
        """Invalid mermaid index does not crash."""
        text_viewer._mermaid_blocks = [SAMPLE_MERMAID]

        with patch.object(text_viewer, "_show_mermaid_preview") as mock:
            text_viewer._on_anchor_clicked(QUrl("mermaid:99"))
            mock.assert_not_called()


class TestPositionMappingWithMermaid:
    """Test that word highlighting works correctly with mermaid blocks."""

    def test_words_before_and_after_mermaid(self, text_viewer):
        """Words before and after mermaid block should highlight correctly."""
        original = "Hello world\n\n```mermaid\ngraph TD\n  A --> B\n```\n\nGoodbye world"

        entry = TextEntry(original_text=original)

        # Positions in original text
        hello_start = 0
        goodbye_start = original.find("Goodbye")

        timestamps = [
            {"word": "Hello", "start": 0.0, "end": 0.5, "original_pos": [hello_start, hello_start + 5]},
            {"word": "Goodbye", "start": 1.0, "end": 1.5, "original_pos": [goodbye_start, goodbye_start + 7]},
        ]

        # Mock _start_mermaid_rendering to avoid WebEngine dependency
        with patch.object(text_viewer, "_start_mermaid_rendering"):
            text_viewer.set_format(TextFormat.MARKDOWN)
            text_viewer.set_entry(entry, timestamps)

        # Highlight "Hello"
        text_viewer.highlight_at_position(0.2)
        assert text_viewer._last_highlighted_pos == (hello_start, hello_start + 5)
        assert text_viewer.extraSelections()

        # Highlight "Goodbye"
        text_viewer.highlight_at_position(1.2)
        assert text_viewer._last_highlighted_pos == (goodbye_start, goodbye_start + 7)
        assert text_viewer.extraSelections()


class TestPlainModeNoMermaid:
    """Test that plain mode does not trigger mermaid rendering."""

    def test_plain_mode_shows_raw_mermaid(self, text_viewer):
        """In plain mode, mermaid blocks are shown as raw text."""
        original = "Text\n\n```mermaid\ngraph TD\n```\n\nEnd"
        entry = TextEntry(original_text=original)

        text_viewer.set_format(TextFormat.PLAIN)
        text_viewer.set_entry(entry)

        rendered = text_viewer.toPlainText()
        assert "```mermaid" in rendered
        assert text_viewer._mermaid_blocks == []

    def test_no_renderer_init_in_plain_mode(self, text_viewer):
        """MermaidRenderer should NOT be initialized in plain mode."""
        original = "```mermaid\ngraph TD\n```"
        entry = TextEntry(original_text=original)

        text_viewer.set_format(TextFormat.PLAIN)
        text_viewer.set_entry(entry)

        assert text_viewer._mermaid_renderer is None


class TestLoadResource:
    """Test loadResource override for mermaid images."""

    def test_returns_pixmap_for_mermaid_img_scheme(self, text_viewer):
        """loadResource returns QPixmap for mermaid-img: URLs when cached."""
        from PyQt6.QtGui import QPixmap, QTextDocument

        mock_renderer = MagicMock()
        pixmap = QPixmap(100, 50)
        mock_renderer.get_cached_pixmap.return_value = pixmap

        text_viewer._mermaid_renderer = mock_renderer
        text_viewer._mermaid_blocks = [SAMPLE_MERMAID]

        result = text_viewer.loadResource(
            QTextDocument.ResourceType.ImageResource.value,
            QUrl("mermaid-img:0"),
        )
        assert result is pixmap
        mock_renderer.get_cached_pixmap.assert_called_once_with(SAMPLE_MERMAID, 600)

    def test_returns_none_for_uncached_mermaid_img(self, text_viewer):
        """loadResource falls through to super when pixmap not cached."""
        from PyQt6.QtGui import QTextDocument

        mock_renderer = MagicMock()
        mock_renderer.get_cached_pixmap.return_value = None

        text_viewer._mermaid_renderer = mock_renderer
        text_viewer._mermaid_blocks = [SAMPLE_MERMAID]

        result = text_viewer.loadResource(
            QTextDocument.ResourceType.ImageResource.value,
            QUrl("mermaid-img:0"),
        )
        # Falls through to super().loadResource() which returns None for unknown
        assert result is None or not isinstance(result, MagicMock)

    def test_non_mermaid_url_delegates_to_super(self, text_viewer):
        """loadResource delegates non-mermaid URLs to parent."""
        from PyQt6.QtGui import QTextDocument

        # Should not crash, just return whatever super returns
        result = text_viewer.loadResource(
            QTextDocument.ResourceType.ImageResource.value,
            QUrl("https://example.com/img.png"),
        )
        # No assertion on value — just verify no crash

    def test_invalid_index_delegates_to_super(self, text_viewer):
        """loadResource with out-of-range index delegates to super."""
        from PyQt6.QtGui import QTextDocument

        mock_renderer = MagicMock()
        text_viewer._mermaid_renderer = mock_renderer
        text_viewer._mermaid_blocks = [SAMPLE_MERMAID]

        result = text_viewer.loadResource(
            QTextDocument.ResourceType.ImageResource.value,
            QUrl("mermaid-img:99"),
        )
        mock_renderer.get_cached_pixmap.assert_not_called()


class TestLazyRendererInit:
    """Test lazy initialization of MermaidRenderer."""

    def test_renderer_not_created_on_init(self, text_viewer):
        """MermaidRenderer should not exist at widget creation."""
        assert text_viewer._mermaid_renderer is None

    def test_renderer_created_on_first_mermaid_render(self, text_viewer):
        """MermaidRenderer created when first mermaid block is encountered."""
        renderer = text_viewer._init_mermaid_renderer()
        assert renderer is not None
        assert text_viewer._mermaid_renderer is renderer

    def test_renderer_reused(self, text_viewer):
        """Same renderer instance reused across calls."""
        r1 = text_viewer._init_mermaid_renderer()
        r2 = text_viewer._init_mermaid_renderer()
        assert r1 is r2
