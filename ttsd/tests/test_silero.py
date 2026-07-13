"""Smoke tests for SileroEngine — require torch and Silero model download.

The TestSplitIntoChunks / TestSanitizeForSilero classes exercise the torch-free
helpers in ttsd.chunking and intentionally do NOT import ttsd.silero, so they run
in CI without the ML stack.
"""

import pytest

from ttsd.chunking import MAX_CHUNK_SIZE, sanitize_for_silero, split_into_chunks


def _import_silero_engine():
    from ttsd.silero import SileroEngine

    return SileroEngine


@pytest.mark.slow
def test_silero_load_and_synthesize(tmp_path):
    engine = _import_silero_engine()()
    engine.load()
    assert engine.is_loaded()

    out_wav = tmp_path / "smoke.wav"
    result = engine.synthesize(
        text="Привет, мир.",
        speaker="xenia",
        sample_rate=48000,
        out_wav=out_wav,
    )

    assert out_wav.exists()
    assert out_wav.stat().st_size > 0
    assert result.duration_sec > 0
    assert len(result.timestamps) > 0


@pytest.mark.slow
def test_silero_second_load_is_noop():
    """Calling load() twice must not raise or reset the model."""
    engine = _import_silero_engine()()
    engine.load()
    model_before = engine._model
    engine.load()
    assert engine._model is model_before


@pytest.mark.slow
def test_silero_empty_text_raises(tmp_path):
    engine = _import_silero_engine()()
    engine.load()
    with pytest.raises(ValueError, match="empty"):
        engine.synthesize(
            text="   ",
            speaker="xenia",
            sample_rate=48000,
            out_wav=tmp_path / "empty.wav",
        )


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
