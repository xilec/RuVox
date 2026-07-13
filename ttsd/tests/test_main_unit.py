"""Unit tests for ttsd.main — cover the request loop and handler error paths.

These tests use a fake engine (no torch/Silero) and monkeypatched stdin/stdout,
so they run in CI under the default (`not slow`) marker set. They intentionally
never import ttsd.silero.
"""

import io
import json
from types import SimpleNamespace

import pytest

from ttsd import main as ttsd_main
from ttsd.protocol import CharMappingEntry, SynthesizeRequest


class FakeEngine:
    """Minimal stand-in for SileroEngine with no ML dependencies."""

    def __init__(self, *, loaded=False, load_exc=None, synth_exc=None):
        self._loaded = loaded
        self._load_exc = load_exc
        self._synth_exc = synth_exc
        self.load_called = False
        self.synth_calls: list[dict] = []

    def is_loaded(self) -> bool:
        return self._loaded

    def load(self) -> None:
        self.load_called = True
        if self._load_exc is not None:
            raise self._load_exc
        self._loaded = True

    def synthesize(self, *, text, speaker, sample_rate, out_wav, char_mapping=None):
        self.synth_calls.append(
            {
                "text": text,
                "speaker": speaker,
                "sample_rate": sample_rate,
                "out_wav": out_wav,
                "char_mapping": char_mapping,
            }
        )
        if self._synth_exc is not None:
            raise self._synth_exc
        return SimpleNamespace(timestamps=[], duration_sec=1.5)


@pytest.fixture
def use_engine(monkeypatch):
    """Install a FakeEngine as the module singleton and return it."""

    def _install(engine: FakeEngine) -> FakeEngine:
        monkeypatch.setattr(ttsd_main, "_engine", engine)
        return engine

    return _install


@pytest.fixture(autouse=True)
def _reset_engine(monkeypatch):
    # Ensure no leftover singleton from another test; the lazy _get_engine would
    # otherwise import ttsd.silero (torch) if left as None and exercised.
    monkeypatch.setattr(ttsd_main, "_engine", None)
    yield


def _make_synth_request(text="привет", char_mapping=None) -> SynthesizeRequest:
    return SynthesizeRequest(
        cmd="synthesize",
        text=text,
        speaker="xenia",
        sample_rate=48000,
        out_wav="/tmp/out.wav",
        char_mapping=char_mapping,
    )


def _run_main(monkeypatch, stdin) -> tuple[int, list[dict]]:
    """Run main() with the given stdin object, returning (rc, parsed responses)."""
    out = io.StringIO()
    monkeypatch.setattr(ttsd_main.sys, "stdin", stdin)
    monkeypatch.setattr(ttsd_main.sys, "stdout", out)
    rc = ttsd_main.main()
    responses = [json.loads(line) for line in out.getvalue().splitlines() if line.strip()]
    return rc, responses


# ---------------------------------------------------------------------------
# _write_response / _setup_logging
# ---------------------------------------------------------------------------


def test_write_response_emits_json_line(monkeypatch):
    out = io.StringIO()
    monkeypatch.setattr(ttsd_main.sys, "stdout", out)
    ttsd_main._write_response(ttsd_main.OkShutdown())
    written = out.getvalue()
    assert written.endswith("\n")
    assert json.loads(written) == {"ok": True}


def test_setup_logging_runs():
    # Should not raise; configures a stderr handler.
    ttsd_main._setup_logging()


# ---------------------------------------------------------------------------
# _handle_warmup
# ---------------------------------------------------------------------------


def test_handle_warmup_ok(use_engine):
    engine = use_engine(FakeEngine())
    resp = ttsd_main._handle_warmup()
    assert engine.load_called
    assert isinstance(resp, ttsd_main.OkWarmup)
    assert resp.ok is True
    assert resp.version == ttsd_main.__version__


def test_handle_warmup_failure_returns_model_not_loaded(use_engine):
    use_engine(FakeEngine(load_exc=RuntimeError("no network")))
    resp = ttsd_main._handle_warmup()
    assert isinstance(resp, ttsd_main.ErrResponse)
    assert resp.error == "model_not_loaded"
    assert "no network" in resp.message


# ---------------------------------------------------------------------------
# _handle_synthesize
# ---------------------------------------------------------------------------


def test_handle_synthesize_not_loaded(use_engine):
    use_engine(FakeEngine(loaded=False))
    resp = ttsd_main._handle_synthesize(_make_synth_request())
    assert isinstance(resp, ttsd_main.ErrResponse)
    assert resp.error == "model_not_loaded"


def test_handle_synthesize_empty_text(use_engine):
    use_engine(FakeEngine(loaded=True))
    resp = ttsd_main._handle_synthesize(_make_synth_request(text="   "))
    assert isinstance(resp, ttsd_main.ErrResponse)
    assert resp.error == "bad_input"
    assert "empty" in resp.message


@pytest.mark.parametrize(
    "exc, expected_error, expected_substr",
    [
        pytest.param(ValueError("bad char"), "bad_input", "bad char", id="value_error_to_bad_input"),
        pytest.param(RuntimeError("boom"), "synthesis_failed", "boom", id="runtime_error_to_synthesis_failed"),
    ],
)
def test_handle_synthesize_exception_mapping(use_engine, exc, expected_error, expected_substr):
    use_engine(FakeEngine(loaded=True, synth_exc=exc))
    resp = ttsd_main._handle_synthesize(_make_synth_request())
    assert isinstance(resp, ttsd_main.ErrResponse)
    assert resp.error == expected_error
    assert expected_substr in resp.message


def test_handle_synthesize_ok_without_char_mapping(use_engine):
    engine = use_engine(FakeEngine(loaded=True))
    resp = ttsd_main._handle_synthesize(_make_synth_request())
    assert isinstance(resp, ttsd_main.OkSynthesize)
    assert resp.ok is True
    assert resp.duration_sec == 1.5
    assert engine.synth_calls[0]["char_mapping"] is None


def test_handle_synthesize_ok_builds_char_mapping_dicts(use_engine):
    engine = use_engine(FakeEngine(loaded=True))
    mapping = [CharMappingEntry(norm_start=0, norm_end=3, orig_start=10, orig_end=13)]
    resp = ttsd_main._handle_synthesize(_make_synth_request(char_mapping=mapping))
    assert isinstance(resp, ttsd_main.OkSynthesize)
    passed = engine.synth_calls[0]["char_mapping"]
    assert passed == [{"norm_start": 0, "norm_end": 3, "orig_start": 10, "orig_end": 13}]


# ---------------------------------------------------------------------------
# _handle_shutdown
# ---------------------------------------------------------------------------


def test_handle_shutdown():
    resp = ttsd_main._handle_shutdown()
    assert isinstance(resp, ttsd_main.OkShutdown)
    assert resp.ok is True


# ---------------------------------------------------------------------------
# main() loop
# ---------------------------------------------------------------------------


def test_main_warmup_then_shutdown(monkeypatch, use_engine):
    use_engine(FakeEngine())
    stdin = io.StringIO('{"cmd": "warmup"}\n{"cmd": "shutdown"}\n')
    rc, responses = _run_main(monkeypatch, stdin)
    assert rc == 0
    assert responses[0]["ok"] is True
    assert responses[0]["version"] == ttsd_main.__version__
    assert responses[1] == {"ok": True}


def test_main_synthesize_dispatch(monkeypatch, use_engine):
    use_engine(FakeEngine(loaded=True))
    req = _make_synth_request().model_dump_json()
    stdin = io.StringIO(req + "\n" + '{"cmd": "shutdown"}\n')
    rc, responses = _run_main(monkeypatch, stdin)
    assert rc == 0
    assert responses[0]["ok"] is True
    assert responses[0]["duration_sec"] == 1.5


def test_main_skips_blank_lines(monkeypatch, use_engine):
    use_engine(FakeEngine())
    stdin = io.StringIO("\n   \n{\"cmd\": \"shutdown\"}\n")
    rc, responses = _run_main(monkeypatch, stdin)
    assert rc == 0
    # Only the shutdown produced a response; blank lines yielded nothing.
    assert responses == [{"ok": True}]


def test_main_invalid_json_returns_bad_input(monkeypatch, use_engine):
    use_engine(FakeEngine())
    stdin = io.StringIO('{"cmd": "synthesize"}\n{"cmd": "shutdown"}\n')
    rc, responses = _run_main(monkeypatch, stdin)
    assert rc == 0
    assert responses[0]["ok"] is False
    assert responses[0]["error"] == "bad_input"


def test_main_unknown_cmd(monkeypatch, use_engine):
    use_engine(FakeEngine())

    # RequestAdapter's discriminated union cannot produce an unknown cmd, so we
    # stub validate_json to return a request object with an out-of-range cmd,
    # which is the only way to reach the else-branch.
    fake_adapter = SimpleNamespace(validate_json=lambda line: SimpleNamespace(cmd="bogus"))
    monkeypatch.setattr(ttsd_main, "RequestAdapter", fake_adapter)
    stdin = io.StringIO('{"cmd": "bogus"}\n')
    rc, responses = _run_main(monkeypatch, stdin)
    assert rc == 0
    assert responses[0]["ok"] is False
    assert responses[0]["error"] == "bad_input"
    assert "unknown cmd: bogus" in responses[0]["message"]


class _RaisingStdin:
    """An stdin whose iteration raises the given exception on first next()."""

    def __init__(self, exc):
        self._exc = exc

    def __iter__(self):
        return self

    def __next__(self):
        raise self._exc


def test_main_keyboard_interrupt_returns_zero(monkeypatch, use_engine):
    use_engine(FakeEngine())
    rc, responses = _run_main(monkeypatch, _RaisingStdin(KeyboardInterrupt()))
    assert rc == 0
    assert responses == []


def test_main_fatal_error_returns_one(monkeypatch, use_engine):
    use_engine(FakeEngine())
    rc, responses = _run_main(monkeypatch, _RaisingStdin(RuntimeError("stdin exploded")))
    assert rc == 1
    assert responses == []
