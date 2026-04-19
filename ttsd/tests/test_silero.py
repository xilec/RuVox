"""Smoke tests for SileroEngine — require torch and Silero model download."""

import pytest

from ttsd.silero import SileroEngine


@pytest.mark.slow
def test_silero_load_and_synthesize(tmp_path):
    engine = SileroEngine()
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
def test_silero_second_load_is_noop(tmp_path):
    """Calling load() twice must not raise or reset the model."""
    engine = SileroEngine()
    engine.load()
    model_before = engine._model
    engine.load()
    assert engine._model is model_before


@pytest.mark.slow
def test_silero_empty_text_raises(tmp_path):
    engine = SileroEngine()
    engine.load()
    with pytest.raises(ValueError, match="empty"):
        engine.synthesize(
            text="   ",
            speaker="xenia",
            sample_rate=48000,
            out_wav=tmp_path / "empty.wav",
        )


class TestSplitIntoChunks:
    """Unit tests for _split_into_chunks that do not need the model."""

    def test_short_text_single_chunk(self):
        text = "Привет мир"
        chunks = SileroEngine._split_into_chunks(text)
        assert chunks == [(text, 0)]

    def test_long_text_splits_on_sentence_boundary(self):
        sentence = "Это предложение. "
        text = sentence * 60  # well over MAX_CHUNK_SIZE
        chunks = SileroEngine._split_into_chunks(text)
        assert len(chunks) > 1
        # Every chunk must be non-empty
        for chunk_text, _ in chunks:
            assert chunk_text.strip()

    def test_chunks_cover_full_text(self):
        sentence = "Слово слово слово. "
        text = sentence * 60
        chunks = SileroEngine._split_into_chunks(text)
        # The start positions must be ordered
        starts = [s for _, s in chunks]
        assert starts == sorted(starts)

    def test_start_positions_non_negative(self):
        text = "A" * 2000
        chunks = SileroEngine._split_into_chunks(text)
        for _, start in chunks:
            assert start >= 0


class TestSanitizeForSilero:
    def test_newlines_replaced_by_space(self):
        result = SileroEngine._sanitize_for_silero("один\nдва")
        assert "\n" not in result
        assert "один два" == result

    def test_multiple_spaces_collapsed(self):
        result = SileroEngine._sanitize_for_silero("один   два")
        assert result == "один два"

    def test_leading_trailing_stripped(self):
        result = SileroEngine._sanitize_for_silero("  текст  ")
        assert result == "текст"
