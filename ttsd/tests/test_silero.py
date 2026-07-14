"""Smoke tests for SileroEngine — require torch and Silero model download."""

import pytest


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
