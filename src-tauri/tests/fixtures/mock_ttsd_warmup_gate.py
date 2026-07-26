"""Mock ttsd that rejects synthesize until warmup is called.

Used by `tests/supervisor.rs` to verify that the supervisor's retry loop
waits for the post-respawn warmup before sending requests to a freshly
respawned process. Like the real Silero ttsd, `synthesize` answers
`model_not_loaded` until this process has received a `warmup` request.

To keep the test deterministic, `warmup` replies only after a short delay
(env var MOCK_TTSD_WARMUP_DELAY_SEC, default 0.5 s): without the readiness
wait the retried synthesize would race the background warmup and reliably
lose.

Behaviour:
  - `warmup`     -> sleep the delay, then reply ok; from then on synthesize
                    succeeds.
  - `synthesize` -> before warmup: error `model_not_loaded` (no side
                    effects); after warmup: ok with empty timestamps.
  - `shutdown`   -> replies ok and exits.

Run via:  MOCK_TTSD_WARMUP_DELAY_SEC=0.5 python tests/fixtures/mock_ttsd_warmup_gate.py
"""

from __future__ import annotations

import json
import os
import sys
import time

WARMUP_DELAY_SEC = float(os.environ.get("MOCK_TTSD_WARMUP_DELAY_SEC", "0.5"))

_warmed_up = False


def _write(payload: dict) -> None:
    sys.stdout.write(json.dumps(payload) + "\n")
    sys.stdout.flush()


def main() -> None:
    global _warmed_up
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
            time.sleep(WARMUP_DELAY_SEC)
            _warmed_up = True
            _write({"ok": True, "version": "mock-0.0.0"})
        elif cmd == "synthesize":
            if not _warmed_up:
                _write(
                    {
                        "ok": False,
                        "error": "model_not_loaded",
                        "message": "Silero model is not loaded",
                    }
                )
            else:
                _write({"ok": True, "timestamps": [], "duration_sec": 0.0})
        elif cmd == "shutdown":
            _write({"ok": True})
            return
        else:
            _write({"ok": False, "error": "bad_cmd", "message": f"unknown cmd: {cmd}"})


if __name__ == "__main__":
    main()
