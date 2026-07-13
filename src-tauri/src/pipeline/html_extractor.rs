//! HTML-to-plain-text extractor for TTS pipeline.
//!
//! Extracts readable text from HTML documents, preserving structural context
//! (headings, lists, paragraphs) and mapping extracted text positions back to
//! the original HTML character positions for word highlighting.
//!
//! # Why scraper?
//! `scraper` wraps `html5ever` (Mozilla's spec-compliant HTML5 parser) and
//! provides CSS selector-based querying. It is actively maintained, handles
//! malformed real-world HTML correctly, and gives us direct access to the DOM
//! tree for selective element exclusion.

use scraper::{Html, Node, Selector};
use thiserror::Error;

/// Errors produced by the HTML extractor.
#[derive(Debug, Error)]
pub enum HtmlExtractError {
    #[error("failed to build CSS selector: {0}")]
    SelectorBuild(String),
}

/// A span linking a range in the extracted plain text to the corresponding
/// range in the original HTML source.
///
/// Both ranges use **byte** offsets (not codepoints) to match `String::get`
/// semantics on the Rust side and to keep integration with the TTS pipeline
/// straightforward.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HtmlCharSpan {
    /// Start byte offset in the extracted plain text.
    pub text_start: usize,
    /// End byte offset (exclusive) in the extracted plain text.
    pub text_end: usize,
    /// Start byte offset in the original HTML source.
    pub html_start: usize,
    /// End byte offset (exclusive) in the original HTML source.
    pub html_end: usize,
}

/// Result of HTML extraction: plain text suitable for TTS and a mapping of
/// text positions back to the original HTML source.
#[derive(Debug, Clone)]
pub struct TrackedHtml {
    /// Plain text extracted from the HTML, ready to be fed into the TTS
    /// pipeline.
    pub text: String,
    /// Position mapping from text byte ranges to original HTML byte ranges.
    pub spans: Vec<HtmlCharSpan>,
}

impl TrackedHtml {
    /// Look up the HTML source range that corresponds to a given byte range in
    /// `self.text`.  Returns the tightest bounding span, or `None` if the
    /// text range falls entirely in synthesised whitespace.
    pub fn html_range_for(&self, text_start: usize, text_end: usize) -> Option<(usize, usize)> {
        let mut html_start = usize::MAX;
        let mut html_end = 0;

        for span in &self.spans {
            // Overlapping condition: spans overlap if they are not disjoint.
            if span.text_end <= text_start || span.text_start >= text_end {
                continue;
            }
            if span.html_start < html_start {
                html_start = span.html_start;
            }
            if span.html_end > html_end {
                html_end = span.html_end;
            }
        }

        if html_start == usize::MAX {
            None
        } else {
            Some((html_start, html_end))
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tag classification
// ─────────────────────────────────────────────────────────────────────────────

/// Tags whose entire subtree is excluded from TTS output (navigation, chrome,
/// non-prose).
const EXCLUDED_TAGS: &[&str] = &[
    "nav", "footer", "aside", "script", "style", "head", "noscript", "template", "svg", "math",
    "button", "select", "option", "optgroup", "datalist",
];

/// Block-level tags that should be preceded by a newline in the output when
/// they contain visible text.
const BLOCK_TAGS: &[&str] = &[
    "p",
    "div",
    "section",
    "article",
    "main",
    "header",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "blockquote",
    "pre",
    "ul",
    "ol",
    "li",
    "dt",
    "dd",
    "dl",
    "figure",
    "figcaption",
    "table",
    "thead",
    "tbody",
    "tfoot",
    "tr",
    "th",
    "td",
    "details",
    "summary",
    "br",
    "hr",
];

fn is_excluded(tag: &str) -> bool {
    EXCLUDED_TAGS.contains(&tag)
}

fn is_block(tag: &str) -> bool {
    BLOCK_TAGS.contains(&tag)
}

// ─────────────────────────────────────────────────────────────────────────────
// Public entry point
// ─────────────────────────────────────────────────────────────────────────────

/// Extract plain text from `html` for TTS consumption, together with a
/// character-level mapping back to the source.
///
/// The function is intentionally lenient: malformed HTML is handled by
/// `html5ever`'s error-recovery and the output will still be sensible.
pub fn extract_text_for_tts(html: &str) -> TrackedHtml {
    let document = Html::parse_document(html);
    let mut ctx = ExtractionCtx::new(html);
    ctx.walk(document.root_element());
    ctx.finish()
}

// ─────────────────────────────────────────────────────────────────────────────
// Internal extraction context
// ─────────────────────────────────────────────────────────────────────────────

struct ExtractionCtx<'h> {
    /// The original HTML source, used for source-position queries.
    _html: &'h str,
    output: String,
    spans: Vec<HtmlCharSpan>,
    /// Whether the last character written to `output` was whitespace / newline.
    last_was_space: bool,
}

impl<'h> ExtractionCtx<'h> {
    fn new(html: &'h str) -> Self {
        Self {
            _html: html,
            output: String::new(),
            spans: Vec::new(),
            last_was_space: true,
        }
    }

    fn finish(self) -> TrackedHtml {
        // Trim leading/trailing whitespace from the final text.
        let trimmed = self.output.trim().to_string();
        // Rebuild spans with adjusted offsets after trim.
        let trim_offset = self
            .output
            .as_bytes()
            .iter()
            .take_while(|&&b| b == b' ' || b == b'\n' || b == b'\t' || b == b'\r')
            .count();

        let spans = if trim_offset == 0 {
            self.spans
        } else {
            self.spans
                .into_iter()
                .filter_map(|mut s| {
                    if s.text_end <= trim_offset {
                        return None;
                    }
                    s.text_start = s.text_start.saturating_sub(trim_offset);
                    s.text_end -= trim_offset;
                    Some(s)
                })
                .collect()
        };

        TrackedHtml {
            text: trimmed,
            spans,
        }
    }

    /// Recursively walk the DOM tree starting from `element`.
    fn walk(&mut self, element: scraper::ElementRef<'_>) {
        let tag = element.value().name().to_lowercase();

        if is_excluded(&tag) {
            // Skip this subtree entirely.
            return;
        }

        let is_block_elem = is_block(&tag);

        // `<br>` and `<hr>` emit newlines but have no children to walk.
        if tag == "br" || tag == "hr" {
            self.push_newline();
            return;
        }

        // Block elements get a preceding newline to separate them from prior text.
        if is_block_elem {
            self.push_newline();
        }

        // Walk children.
        for child in element.children() {
            match child.value() {
                Node::Text(text) => {
                    let s: &str = text;
                    // scraper gives us the raw text node; we normalise whitespace
                    // the same way browsers do for inline text.
                    self.push_text(s);
                }
                Node::Element(_) => {
                    if let Some(child_elem) = scraper::ElementRef::wrap(child) {
                        self.walk(child_elem);
                    }
                }
                _ => {}
            }
        }

        // Block elements get a trailing newline.
        if is_block_elem {
            self.push_newline();
        }
    }

    /// Append plain text, collapsing consecutive whitespace and recording a span.
    fn push_text(&mut self, raw: &str) {
        let mut pending = String::new();

        for ch in raw.chars() {
            if ch.is_ascii_whitespace() || ch == '\u{00a0}' {
                // Non-breaking space treated as regular space.
                if !self.last_was_space {
                    pending.push(' ');
                    self.last_was_space = true;
                }
            } else {
                self.last_was_space = false;
                pending.push(ch);
            }
        }

        if !pending.is_empty() {
            let text_start = self.output.len();
            self.output.push_str(&pending);
            let text_end = self.output.len();

            // We cannot reliably get scraper source offsets from text nodes
            // without patching the library, so we mark html_start == html_end
            // as a sentinel meaning "source position unknown for this span."
            // The word highlighter in the UI will gracefully degrade.
            self.spans.push(HtmlCharSpan {
                text_start,
                text_end,
                html_start: 0,
                html_end: 0,
            });
        }
    }

    fn push_newline(&mut self) {
        if !self.output.is_empty() && !self.output.ends_with('\n') {
            self.output.push('\n');
            self.last_was_space = true;
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// CSS-selector helper (for tests / future use)
// ─────────────────────────────────────────────────────────────────────────────

/// Build an exclusion selector for structural HTML elements that should never
/// be read aloud.
#[allow(dead_code)]
pub fn build_exclusion_selector() -> Result<Selector, HtmlExtractError> {
    Selector::parse("nav, footer, aside, script, style")
        .map_err(|e| HtmlExtractError::SelectorBuild(format!("{e:?}")))
}

// ─────────────────────────────────────────────────────────────────────────────
// Normalise extracted text
// ─────────────────────────────────────────────────────────────────────────────

/// Collapse multiple consecutive blank lines / newlines into a single newline
/// and trim the result.
///
/// The scraper walk can produce multiple adjacent newlines when block elements
/// nest deeply.  This pass reduces them to a single separator so that the TTS
/// pipeline sees clean paragraph boundaries.
pub fn normalise_extracted(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut prev_was_newline = false;

    for line in text.split('\n') {
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            // Skip — multiple blank lines collapse to nothing between content.
            prev_was_newline = true;
        } else {
            if prev_was_newline && !result.is_empty() {
                result.push('\n');
            }
            result.push_str(trimmed);
            prev_was_newline = true;
        }
    }

    result.trim().to_string()
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn extract(html: &str) -> String {
        let tracked = extract_text_for_tts(html);
        normalise_extracted(&tracked.text)
    }

    // ── 1. Simple paragraph ──────────────────────────────────────────────────

    #[test]
    fn test_simple_paragraph() {
        let html = "<p>Привет, мир!</p>";
        assert_eq!(extract(html), "Привет, мир!");
    }

    // ── 2. Multiple paragraphs ───────────────────────────────────────────────

    #[test]
    fn test_multiple_paragraphs() {
        let html = "<p>Первый абзац.</p><p>Второй абзац.</p>";
        assert_eq!(extract(html), "Первый абзац.\nВторой абзац.");
    }

    // ── 3. Heading tags ──────────────────────────────────────────────────────

    #[test]
    fn test_headings() {
        let html = "<h1>Заголовок</h1><p>Текст статьи.</p>";
        assert_eq!(extract(html), "Заголовок\nТекст статьи.");
    }

    // ── 4. Nested block elements ─────────────────────────────────────────────

    #[test]
    fn test_nested_blocks() {
        let html = "<article><section><p>Вложенный текст.</p></section></article>";
        assert_eq!(extract(html), "Вложенный текст.");
    }

    // ── 5. nav excluded ──────────────────────────────────────────────────────

    #[test]
    fn test_nav_excluded() {
        let html = "<nav><a href='/'>Домой</a></nav><p>Основное содержимое.</p>";
        assert_eq!(extract(html), "Основное содержимое.");
    }

    // ── 6. footer excluded ───────────────────────────────────────────────────

    #[test]
    fn test_footer_excluded() {
        let html = "<p>Тело страницы.</p><footer><p>Копирайт 2026</p></footer>";
        assert_eq!(extract(html), "Тело страницы.");
    }

    // ── 7. aside excluded ────────────────────────────────────────────────────

    #[test]
    fn test_aside_excluded() {
        let html = "<p>Главный текст.</p><aside><p>Боковая заметка</p></aside>";
        assert_eq!(extract(html), "Главный текст.");
    }

    // ── 8. script and style excluded ─────────────────────────────────────────

    #[test]
    fn test_script_style_excluded() {
        let html = r#"<style>body{color:red}</style><script>alert(1)</script><p>Виден</p>"#;
        assert_eq!(extract(html), "Виден");
    }

    // ── 9. Unordered list ────────────────────────────────────────────────────

    #[test]
    fn test_unordered_list() {
        let html = "<ul><li>Первый пункт</li><li>Второй пункт</li></ul>";
        assert_eq!(extract(html), "Первый пункт\nВторой пункт");
    }

    // ── 10. Ordered list ─────────────────────────────────────────────────────

    #[test]
    fn test_ordered_list() {
        let html = "<ol><li>Шаг один</li><li>Шаг два</li><li>Шаг три</li></ol>";
        assert_eq!(extract(html), "Шаг один\nШаг два\nШаг три");
    }

    // ── 11. Inline code ──────────────────────────────────────────────────────

    #[test]
    fn test_inline_code() {
        let html = "<p>Вызови функцию <code>getUserData()</code> для получения данных.</p>";
        assert_eq!(
            extract(html),
            "Вызови функцию getUserData() для получения данных."
        );
    }

    // ── 12. Pre/code block ───────────────────────────────────────────────────

    #[test]
    fn test_pre_code_block() {
        let html =
            r#"<pre><code class="language-rust">fn main() { println!("hello"); }</code></pre>"#;
        assert_eq!(extract(html), r#"fn main() { println!("hello"); }"#);
    }

    // ── 13. Full article structure with exclusions ───────────────────────────

    #[test]
    fn test_full_article_structure() {
        let html = r#"
            <html>
            <head><title>Тест</title></head>
            <body>
                <nav><a href="/">Главная</a></nav>
                <main>
                    <article>
                        <h1>Заголовок статьи</h1>
                        <p>Первый параграф содержимого.</p>
                        <aside><p>Боковая панель</p></aside>
                        <p>Второй параграф содержимого.</p>
                    </article>
                </main>
                <footer>Подвал сайта</footer>
            </body>
            </html>
        "#;
        assert_eq!(
            extract(html),
            "Заголовок статьи\nПервый параграф содержимого.\nВторой параграф содержимого."
        );
    }

    // ── 14. Whitespace normalisation ─────────────────────────────────────────

    #[test]
    fn test_whitespace_normalisation() {
        let html = "<p>Текст   с   лишними   пробелами.</p>";
        // Multiple spaces in HTML source collapse to a single space in text.
        assert_eq!(extract(html), "Текст с лишними пробелами.");
    }

    // ── 15. TrackedHtml span boundaries ──────────────────────────────────────

    // NOTE: `push_text` (see production code) records one span per raw text
    // node, with `text_start`/`text_end` as *byte* offsets into `self.output`
    // (matching `TrackedHtml.text` once the leading-whitespace trim offset is
    // applied). Crucially, `html_start`/`html_end` are hard-coded to `0` for
    // every span — the doc comment on `push_text` explains that scraper does
    // not expose source byte offsets for text nodes, so the field is a
    // sentinel ("source position unknown"), not a real mapping back into the
    // HTML source. This test therefore verifies the one boundary that *is*
    // implemented (text-side spans slicing back to the correct substrings)
    // and pins down the current sentinel behaviour of the html-side fields,
    // rather than asserting a source-position guarantee the code does not
    // provide.
    #[test]
    fn test_tracked_html_spans_exist() {
        // Three text nodes: "Первый " (before <b>), "второй" (inside <b>),
        // " третий" (after </b>) — none of them adjacent to a block-level
        // newline, so each becomes its own span with no trim adjustment.
        let html = "<p>Первый <b>второй</b> третий</p>";
        let tracked = extract_text_for_tts(html);

        assert_eq!(tracked.text, "Первый второй третий");
        assert_eq!(tracked.spans.len(), 3, "expected one span per text node");

        let expected = [(0, 13, "Первый "), (13, 25, "второй"), (25, 38, " третий")];
        for (span, (start, end, fragment)) in tracked.spans.iter().zip(expected) {
            assert_eq!(
                (span.text_start, span.text_end),
                (start, end),
                "span byte range mismatch for {fragment:?}"
            );
            assert_eq!(
                &tracked.text[span.text_start..span.text_end],
                fragment,
                "span does not slice back to its own text"
            );
            // Sentinel: source-position tracking for text nodes is not
            // implemented, so html_start/html_end are always (0, 0).
            assert_eq!((span.html_start, span.html_end), (0, 0));
        }

        // Spans are contiguous and cover the whole trimmed text exactly.
        assert_eq!(tracked.spans.last().unwrap().text_end, tracked.text.len());
    }

    // ── 16. html_range_for ────────────────────────────────────────────────────

    #[test]
    fn test_html_range_for_out_of_bounds_range_returns_none() {
        let html = "<p>Привет</p>";
        let tracked = extract_text_for_tts(html);
        // The range (1000, 1000) lies entirely past the end of the text, so
        // no span overlaps it.
        let result = tracked.html_range_for(1000, 1000);
        assert!(result.is_none(), "out-of-bounds range should return None");
    }

    #[test]
    fn test_html_range_for_whitespace_gap_between_blocks_returns_none() {
        // The newline separating the two paragraphs is synthesised by
        // `push_newline` and is not covered by any span, so a text range
        // that falls entirely inside it must return None.
        let html = "<p>Первый абзац.</p><p>Второй абзац.</p>";
        let tracked = extract_text_for_tts(html);
        assert_eq!(tracked.text, "Первый абзац.\nВторой абзац.");
        let newline_at = tracked.text.find('\n').expect("expected a newline gap");
        assert_eq!(newline_at, 24);

        let result = tracked.html_range_for(newline_at, newline_at + 1);
        assert!(
            result.is_none(),
            "a range inside the inter-paragraph gap should return None"
        );
    }

    #[test]
    fn test_html_range_for_valid_overlap_returns_range() {
        // A request range that overlaps a real span must return Some(range).
        // Because html_start/html_end are currently a (0, 0) sentinel for
        // every text-node span (see note on test_tracked_html_spans_exist),
        // the returned range is (0, 0) for any overlapping query today; this
        // pins the current overlap-detection behaviour precisely rather than
        // asserting a source-position guarantee the implementation does not
        // yet provide.
        let html = "<p>Первый <b>второй</b> третий</p>";
        let tracked = extract_text_for_tts(html);
        assert_eq!(tracked.text, "Первый второй третий");

        // Query exactly the middle span ("второй"), byte range [13, 25).
        let result = tracked.html_range_for(13, 25);
        assert_eq!(result, Some((0, 0)));

        // A partial overlap with the first span also matches.
        let partial = tracked.html_range_for(5, 15);
        assert_eq!(partial, Some((0, 0)));
    }

    // ── 17. normalise_extracted collapses blank lines ────────────────────────

    #[test]
    fn test_normalise_multiple_blank_lines() {
        let text = "Первый\n\n\n\nВторой";
        let normalised = normalise_extracted(text);
        assert_eq!(normalised, "Первый\nВторой");
    }

    // ── 18. blockquote content preserved ─────────────────────────────────────

    #[test]
    fn test_blockquote_preserved() {
        let html = "<blockquote><p>Цитата из источника.</p></blockquote>";
        assert_eq!(extract(html), "Цитата из источника.");
    }

    // ── 19. table cells concatenated ─────────────────────────────────────────

    #[test]
    fn test_table_cells() {
        let html = "<table><tr><td>Ячейка один</td><td>Ячейка два</td></tr></table>";
        assert_eq!(extract(html), "Ячейка один\nЯчейка два");
    }

    // ── 20. Empty HTML returns empty string ──────────────────────────────────

    #[test]
    fn test_empty_html() {
        let tracked = extract_text_for_tts("");
        let out = normalise_extracted(&tracked.text);
        assert!(
            out.is_empty(),
            "empty HTML should yield empty text: {out:?}"
        );
    }
}
