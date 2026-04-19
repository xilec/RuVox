"""Tests for MermaidRenderer service."""

from pathlib import Path
from unittest.mock import MagicMock, patch

import pytest
from PyQt6.QtGui import QPixmap


@pytest.fixture
def renderer(qapp):
    """Create MermaidRenderer instance."""
    from ruvox.ui.services.mermaid_renderer import MermaidRenderer

    r = MermaidRenderer()
    yield r
    r.cleanup()
    r.deleteLater()


class TestCodeHash:
    """Test code hashing utility."""

    def test_same_code_same_hash(self):
        from ruvox.ui.services.mermaid_renderer import _code_hash

        code = "graph TD\n  A --> B"
        assert _code_hash(code) == _code_hash(code)

    def test_different_code_different_hash(self):
        from ruvox.ui.services.mermaid_renderer import _code_hash

        h1 = _code_hash("graph TD\n  A --> B")
        h2 = _code_hash("graph LR\n  A --> B")
        assert h1 != h2

    def test_strips_whitespace(self):
        from ruvox.ui.services.mermaid_renderer import _code_hash

        h1 = _code_hash("graph TD\n  A --> B")
        h2 = _code_hash("  graph TD\n  A --> B  \n")
        assert h1 == h2


class TestSvgCache:
    """Test SVG caching."""

    def test_cache_miss_returns_none(self, renderer):
        assert renderer.get_cached_svg("graph TD\n  A --> B") is None

    def test_cache_hit_after_manual_insert(self, renderer):
        from ruvox.ui.services.mermaid_renderer import _code_hash

        code = "graph TD\n  A --> B"
        h = _code_hash(code)
        renderer._svg_cache[h] = "<svg>test</svg>"
        assert renderer.get_cached_svg(code) == "<svg>test</svg>"

    def test_cached_pixmap_returns_none_when_no_svg(self, renderer):
        assert renderer.get_cached_pixmap("graph TD\n  A --> B", 600) is None

    def test_cached_pixmap_returns_pixmap_when_svg_valid(self, renderer):
        from ruvox.ui.services.mermaid_renderer import _code_hash

        code = "graph TD\n  A --> B"
        h = _code_hash(code)
        # Minimal valid SVG
        svg = '<svg xmlns="http://www.w3.org/2000/svg" width="100" height="50"><rect width="100" height="50" fill="white"/></svg>'
        renderer._svg_cache[h] = svg
        pixmap = renderer.get_cached_pixmap(code, 200)
        assert pixmap is not None
        assert isinstance(pixmap, QPixmap)
        assert pixmap.width() == 200


class TestMermaidJsDownload:
    """Test mermaid.min.js downloading."""

    def test_download_called_when_missing(self, renderer, tmp_path):
        """Should call urlretrieve when mermaid.min.js doesn't exist."""
        with patch(
            "ruvox.ui.services.mermaid_renderer._mermaid_cache_dir",
            return_value=tmp_path,
        ):
            callback = MagicMock()
            with patch("urllib.request.urlretrieve") as mock_dl:
                # Simulate download creating the file
                def fake_download(url, path):
                    Path(path).write_text("// mermaid.min.js")

                mock_dl.side_effect = fake_download

                renderer._ensure_mermaid_js(callback)

                mock_dl.assert_called_once()
                callback.assert_called_once()
                assert renderer._mermaid_js_path == tmp_path / "mermaid.min.js"

    def test_no_download_when_cached(self, renderer, tmp_path):
        """Should NOT download when mermaid.min.js already exists."""
        js_path = tmp_path / "mermaid.min.js"
        js_path.write_text("// already cached")

        with patch(
            "ruvox.ui.services.mermaid_renderer._mermaid_cache_dir",
            return_value=tmp_path,
        ):
            callback = MagicMock()
            with patch("urllib.request.urlretrieve") as mock_dl:
                renderer._ensure_mermaid_js(callback)
                mock_dl.assert_not_called()
                callback.assert_called_once()

    def test_download_failure_handled(self, renderer, tmp_path):
        """Should handle download failure gracefully."""
        with patch(
            "ruvox.ui.services.mermaid_renderer._mermaid_cache_dir",
            return_value=tmp_path,
        ):
            callback = MagicMock()
            with patch("urllib.request.urlretrieve", side_effect=OSError("Network error")):
                renderer._ensure_mermaid_js(callback)
                callback.assert_not_called()


class TestRenderQueue:
    """Test render queue management."""

    def test_cached_code_emits_immediately(self, renderer):
        from ruvox.ui.services.mermaid_renderer import _code_hash

        code = "graph TD\n  A --> B"
        h = _code_hash(code)
        renderer._svg_cache[h] = "<svg>cached</svg>"

        received = []
        renderer.svg_ready.connect(lambda ch, svg: received.append((ch, svg)))

        # Mock _ensure_mermaid_js to avoid real download
        renderer._js_ready = True
        renderer._mermaid_js_path = Path("/tmp/fake.js")

        renderer.render(code)

        assert len(received) == 1
        assert received[0] == (h, "<svg>cached</svg>")

    def test_no_duplicate_in_queue(self, renderer):
        code = "graph TD\n  A --> B"

        # Mock _ensure_mermaid_js to just record queue without processing
        renderer._ensure_mermaid_js = lambda cb: None

        renderer.render(code)
        renderer.render(code)

        assert len(renderer._queue) == 1


class TestRenderBaseUrl:
    """Test that _render_in_webview passes base URL."""

    def test_sethtml_called_with_base_url(self, renderer, tmp_path):
        """_render_in_webview should pass base URL to setHtml."""
        js_path = tmp_path / "mermaid.min.js"
        js_path.write_text("// mock")
        renderer._mermaid_js_path = js_path
        renderer._js_ready = True

        # Lazy-init webview with mock
        mock_web_view = MagicMock()
        renderer._web_view = mock_web_view

        renderer._render_in_webview("abc123", "graph TD\n  A --> B")

        mock_web_view.setHtml.assert_called_once()
        args, kwargs = mock_web_view.setHtml.call_args
        html = args[0]
        base_url = args[1]

        assert 'src="mermaid.min.js"' in html
        assert (
            "file:///" not in html or "mermaid.min.js" not in html.split("file:///")[0] if "file:///" in html else True
        )
        assert str(tmp_path) in base_url.toLocalFile()

    def test_html_uses_relative_script_src(self, renderer, tmp_path):
        """HTML should reference mermaid.min.js without file:// prefix."""
        js_path = tmp_path / "mermaid.min.js"
        js_path.write_text("// mock")
        renderer._mermaid_js_path = js_path
        renderer._js_ready = True

        mock_web_view = MagicMock()
        renderer._web_view = mock_web_view

        renderer._render_in_webview("abc123", "graph TD\n  A --> B")

        html = mock_web_view.setHtml.call_args[0][0]
        assert 'src="mermaid.min.js"' in html


class TestSvgToPixmap:
    """Test static SVG to QPixmap conversion."""

    def test_invalid_svg_returns_none(self, qapp):
        from ruvox.ui.services.mermaid_renderer import MermaidRenderer

        result = MermaidRenderer._svg_to_pixmap("not valid svg", 200)
        assert result is None

    def test_valid_svg_returns_pixmap(self, qapp):
        from ruvox.ui.services.mermaid_renderer import MermaidRenderer

        svg = '<svg xmlns="http://www.w3.org/2000/svg" width="100" height="50"><rect width="100" height="50" fill="red"/></svg>'
        result = MermaidRenderer._svg_to_pixmap(svg, 300)
        assert result is not None
        assert result.width() == 300
        assert result.height() == 150  # aspect ratio preserved
