"""Mock ttsd whose first-ever synthesize call sleeps until killed.

Used by `tests/supervisor.rs` to verify `TtsSupervisor::kill_current`
terminates an in-flight request and the next request transparently
respawns. A marker file (path from the MOCK_TTSD_SLEEP_MARKER env var)
distinguishes the first process from respawned ones: the first process
creates the marker and sleeps ~forever on synthesize; any later process
(marker already present) replies immediately.

Behaviour:
  - `warmup`     -> always replies ok.
  - `synthesize` -> first process (no marker): create marker, sleep 30 s
                    (long enough that only a real kill unblocks it), then
                    reply ok. Later processes: reply ok immediately.
  - `shutdown`   -> replies ok and exits.

Run via:  MOCK_TTSD_SLEEP_MARKER=/tmp/marker python tests/fixtures/mock_ttsd_sleepy.py
"""

from __future__ import annotations

import json
import os
import sys
import time

SLEEP_SEC = 30


def _write(payload: dict) -> None:
    sys.stdout.write(json.dumps(payload) + "\n")
    sys.stdout.flush()


def main() -> None:
    marker = os.environ["MOCK_TTSD_SLEEP_MARKER"]
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
            if not os.path.exists(marker):
                # Create the marker BEFORE sleeping so a respawned process
                # replies instantly.
                with open(marker, "w"):
                    pass
                time.sleep(SLEEP_SEC)
            _write({"ok": True, "timestamps": [], "duration_sec": 0.0})
        elif cmd == "shutdown":
            _write({"ok": True})
            return
        else:
            _write({"ok": False, "error": "bad_cmd", "message": f"unknown cmd: {cmd}"})


if __name__ == "__main__":
    main()
