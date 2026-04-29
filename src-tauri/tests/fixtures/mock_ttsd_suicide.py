"""Mock ttsd that responds once, then exits ungracefully on the second call.

Used by `tests/supervisor.rs` to verify the supervisor respawns the
subprocess after a crash. The protocol matches the real ttsd just enough
for `TtsSubprocess::warmup` and `TtsSubprocess::synthesize` to round-trip
without needing Silero or torch.

Behaviour:
  - `warmup`     -> always replies ok.
  - `synthesize` -> first call replies ok with empty timestamps and a fixed
                    duration; subsequent calls call `os._exit(1)` *before*
                    flushing a response, simulating a hard crash mid-request.

Run via:  python tests/fixtures/mock_ttsd_suicide.py
"""

from __future__ import annotations

import json
import os
import sys

_synthesize_calls = 0


def _write(payload: dict) -> None:
    sys.stdout.write(json.dumps(payload) + "\n")
    sys.stdout.flush()


def main() -> None:
    global _synthesize_calls
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        try:
            req = json.loads(line)
        except json.JSONDecodeError as exc:
            _write({"ok": False, "error": "bad_json", "message": str(exc)})
            continue

        cmd = req.get("cmd")
        if cmd == "warmup":
            _write({"ok": True, "version": "mock-0.0.0"})
        elif cmd == "synthesize":
            _synthesize_calls += 1
            if _synthesize_calls == 1:
                _write({"ok": True, "timestamps": [], "duration_sec": 0.0})
            else:
                # Hard exit BEFORE flushing a response — drops stdout and
                # forces the Rust side to observe Died on the next read.
                os._exit(1)
        elif cmd == "shutdown":
            _write({"ok": True})
            return
        else:
            _write({"ok": False, "error": "bad_cmd", "message": f"unknown cmd: {cmd}"})


if __name__ == "__main__":
    main()
