"""Unit tests for ttsd.chunking — torch-free, run in CI without the ML stack."""

from ttsd.chunking import MAX_CHUNK_SIZE, sanitize_for_silero, split_into_chunks


def _assert_covers_source(text: str, chunks: list[tuple[str, int]]) -> None:
    """Every chunk must sit at its declared start position, stay within
    MAX_CHUNK_SIZE, and the gaps between (and after) chunks must be
    whitespace-only — i.e. the chunks collectively cover all non-whitespace
    content of the source text, in order, with nothing dropped or duplicated.
    """
    for chunk_text, start in chunks:
        assert text[start : start + len(chunk_text)] == chunk_text
        assert len(chunk_text) <= MAX_CHUNK_SIZE

    for (chunk_text, start), (_, next_start) in zip(chunks, chunks[1:], strict=False):
        gap = text[start + len(chunk_text) : next_start]
        assert gap.strip() == ""

    last_text, last_start = chunks[-1]
    assert text[last_start + len(last_text) :].strip() == ""


class TestSplitIntoChunks:
    """Unit tests for split_into_chunks that do not need the model."""

    def test_short_text_single_chunk(self):
        text = "Привет мир"
        chunks = split_into_chunks(text)
        assert chunks == [(text, 0)]

    def test_long_text_splits_on_sentence_boundary(self):
        sentence = "Это предложение. "
        text = sentence * 60  # well over MAX_CHUNK_SIZE
        chunks = split_into_chunks(text)
        assert len(chunks) > 1
        # Every chunk must be non-empty
        for chunk_text, _ in chunks:
            assert chunk_text.strip()
        _assert_covers_source(text, chunks)
        # Every split but the last must land right after a sentence boundary.
        for chunk_text, _ in chunks[:-1]:
            assert chunk_text.rstrip().endswith(".")

    def test_chunks_cover_full_text(self):
        sentence = "Слово слово слово. "
        text = sentence * 60
        chunks = split_into_chunks(text)
        # The start positions must be ordered
        starts = [s for _, s in chunks]
        assert starts == sorted(starts)
        _assert_covers_source(text, chunks)

    def test_start_positions_non_negative(self):
        text = "A" * 2000
        chunks = split_into_chunks(text)
        for _, start in chunks:
            assert start >= 0
        _assert_covers_source(text, chunks)


class TestSanitizeForSilero:
    def test_newlines_replaced_by_space(self):
        result = sanitize_for_silero("один\nдва")
        assert "\n" not in result
        assert "один два" == result

    def test_multiple_spaces_collapsed(self):
        result = sanitize_for_silero("один   два")
        assert result == "один два"

    def test_leading_trailing_stripped(self):
        result = sanitize_for_silero("  текст  ")
        assert result == "текст"
