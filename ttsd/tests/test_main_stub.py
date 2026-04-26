"""Integration test: spawn ttsd as a subprocess and verify the JSON protocol."""

import json
import subprocess
import sys

import pytest


@pytest.mark.slow
def test_warmup_and_shutdown() -> None:
    """Spawn python -m ttsd, send warmup, expect ok response, then shutdown."""
    proc = subprocess.Popen(
        [sys.executable, "-m", "ttsd"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    try:
        # Send warmup
        assert proc.stdin is not None
        proc.stdin.write('{"cmd": "warmup"}\n')
        proc.stdin.flush()

        # Read warmup response
        assert proc.stdout is not None
        line = proc.stdout.readline()
        response = json.loads(line)
        assert response["ok"] is True
        assert "version" in response
        assert response["version"] == "0.1.0"

        # Send shutdown
        proc.stdin.write('{"cmd": "shutdown"}\n')
        proc.stdin.flush()

        # Read shutdown response
        line = proc.stdout.readline()
        response = json.loads(line)
        assert response["ok"] is True

        # Process should exit cleanly
        proc.wait(timeout=5)
        assert proc.returncode == 0
    finally:
        if proc.poll() is None:
            proc.terminate()
            proc.wait(timeout=5)


@pytest.mark.slow
def test_bad_input_returns_error() -> None:
    """Malformed JSON should return an ErrResponse with error='bad_input'."""
    proc = subprocess.Popen(
        [sys.executable, "-m", "ttsd"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    try:
        assert proc.stdin is not None
        assert proc.stdout is not None

        # Send invalid JSON (missing required fields)
        proc.stdin.write('{"cmd": "synthesize"}\n')
        proc.stdin.flush()

        line = proc.stdout.readline()
        response = json.loads(line)
        assert response["ok"] is False
        assert response["error"] == "bad_input"

        # Clean shutdown
        proc.stdin.write('{"cmd": "shutdown"}\n')
        proc.stdin.flush()
        proc.wait(timeout=5)
        assert proc.returncode == 0
    finally:
        if proc.poll() is None:
            proc.terminate()
            proc.wait(timeout=5)
