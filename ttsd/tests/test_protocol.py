"""Unit tests for ttsd protocol models."""

import pytest
from pydantic import ValidationError

from ttsd.protocol import (
    CharMappingEntry,
    ErrResponse,
    OkShutdown,
    OkSynthesize,
    OkWarmup,
    RequestAdapter,
    ShutdownRequest,
    SynthesizeRequest,
    WarmupRequest,
    WordTimestamp,
)


class TestWarmupRequest:
    def test_valid_warmup_parses(self) -> None:
        req = RequestAdapter.validate_json('{"cmd": "warmup"}')
        assert isinstance(req, WarmupRequest)
        assert req.cmd == "warmup"

    def test_warmup_missing_cmd_raises(self) -> None:
        with pytest.raises(ValidationError):
            RequestAdapter.validate_json('{}')


class TestSynthesizeRequest:
    VALID_JSON = (
        '{"cmd": "synthesize", "text": "привет мир", "speaker": "xenia",'
        ' "sample_rate": 48000, "out_wav": "/tmp/test.wav"}'
    )

    def test_valid_synthesize_parses(self) -> None:
        req = RequestAdapter.validate_json(self.VALID_JSON)
        assert isinstance(req, SynthesizeRequest)
        assert req.text == "привет мир"
        assert req.speaker == "xenia"
        assert req.sample_rate == 48000
        assert req.out_wav == "/tmp/test.wav"
        assert req.char_mapping is None

    def test_synthesize_with_char_mapping(self) -> None:
        json_with_mapping = (
            '{"cmd": "synthesize", "text": "hi", "speaker": "xenia",'
            ' "sample_rate": 48000, "out_wav": "/tmp/test.wav",'
            ' "char_mapping": [{"norm_start": 0, "norm_end": 2, "orig_start": 0, "orig_end": 2}]}'
        )
        req = RequestAdapter.validate_json(json_with_mapping)
        assert isinstance(req, SynthesizeRequest)
        assert req.char_mapping is not None
        assert len(req.char_mapping) == 1
        assert isinstance(req.char_mapping[0], CharMappingEntry)

    def test_synthesize_roundtrip(self) -> None:
        req = RequestAdapter.validate_json(self.VALID_JSON)
        assert isinstance(req, SynthesizeRequest)
        dumped = req.model_dump_json()
        req2 = RequestAdapter.validate_json(dumped)
        assert isinstance(req2, SynthesizeRequest)
        assert req2.text == req.text
        assert req2.speaker == req.speaker
        assert req2.sample_rate == req.sample_rate
        assert req2.out_wav == req.out_wav

    def test_synthesize_missing_required_field_raises(self) -> None:
        json_missing_text = (
            '{"cmd": "synthesize", "speaker": "xenia",'
            ' "sample_rate": 48000, "out_wav": "/tmp/test.wav"}'
        )
        with pytest.raises(ValidationError):
            RequestAdapter.validate_json(json_missing_text)


class TestShutdownRequest:
    def test_valid_shutdown_parses(self) -> None:
        req = RequestAdapter.validate_json('{"cmd": "shutdown"}')
        assert isinstance(req, ShutdownRequest)
        assert req.cmd == "shutdown"


class TestWordTimestamp:
    def test_word_timestamp_fields(self) -> None:
        ts = WordTimestamp(word="привет", start=0.0, end=0.5, original_pos=(0, 6))
        assert ts.word == "привет"
        assert ts.start == 0.0
        assert ts.end == 0.5
        assert ts.original_pos == (0, 6)

    def test_word_timestamp_serialization(self) -> None:
        ts = WordTimestamp(word="мир", start=0.55, end=0.9, original_pos=(7, 10))
        data = ts.model_dump()
        assert data["word"] == "мир"
        assert data["original_pos"] == (7, 10)


class TestOkSynthesize:
    def test_ok_synthesize_shape(self) -> None:
        ts1 = WordTimestamp(word="вызови", start=0.0, end=0.42, original_pos=(0, 6))
        ts2 = WordTimestamp(word="функцию", start=0.45, end=1.10, original_pos=(7, 14))
        resp = OkSynthesize(timestamps=[ts1, ts2], duration_sec=12.34)
        assert resp.ok is True
        assert len(resp.timestamps) == 2
        assert resp.duration_sec == 12.34

    def test_ok_synthesize_json_matches_contract(self) -> None:
        """Serialized shape must match docs/ipc-contract.md Layer 3."""
        ts = WordTimestamp(word="привет", start=0.0, end=0.5, original_pos=(0, 6))
        resp = OkSynthesize(timestamps=[ts], duration_sec=0.9)
        data = resp.model_dump()
        assert data["ok"] is True
        assert "timestamps" in data
        assert "duration_sec" in data
        assert data["timestamps"][0]["word"] == "привет"
        assert data["timestamps"][0]["original_pos"] == (0, 6)


class TestOkWarmup:
    def test_ok_warmup_has_version(self) -> None:
        resp = OkWarmup(version="0.1.0")
        assert resp.ok is True
        assert resp.version == "0.1.0"

    def test_ok_warmup_serializes(self) -> None:
        resp = OkWarmup(version="0.1.0")
        data = resp.model_dump()
        assert data["ok"] is True
        assert data["version"] == "0.1.0"


class TestOkShutdown:
    def test_ok_shutdown_serializes(self) -> None:
        resp = OkShutdown()
        data = resp.model_dump()
        assert data["ok"] is True


class TestErrResponse:
    def test_err_response_model_not_loaded(self) -> None:
        resp = ErrResponse(error="model_not_loaded", message="Model not loaded")
        assert resp.ok is False
        assert resp.error == "model_not_loaded"

    def test_err_response_bad_input(self) -> None:
        resp = ErrResponse(error="bad_input", message="text must not be empty")
        data = resp.model_dump()
        assert data["ok"] is False
        assert data["error"] == "bad_input"
        assert data["message"] == "text must not be empty"

    def test_err_response_invalid_error_code_raises(self) -> None:
        with pytest.raises(ValidationError):
            ErrResponse(error="unknown_code", message="test")  # type: ignore[arg-type]
