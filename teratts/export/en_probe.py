#!/usr/bin/env python3
"""Render the #311 English-routing probe samples with the upstream model.

`src-tauri/tests/teratts_en_probe.rs` writes `probe.json` (raw fragment,
today's Russian narration, native-profile output). This script turns every row
into two 44.1 kHz WAVs:

- `<kind>-native.wav`   — the raw fragment inside `<en>…</en>` with `eng_f5`;
- `<kind>-narrated.wav` — the current `TERATTS_PROFILE` narration with `ru_m5`.

Usage:

    uv run python en_probe.py \
      --snapshot /path/to/hf/snapshot \
      --bundle ../../tmp/teratts-bundle \
      --inputs ../../tmp/teratts-en-probe/probe.json \
      --out ../../tmp/teratts-en-probe
"""

from __future__ import annotations

import argparse
import json
import sys
import wave
from pathlib import Path

import numpy as np

SAMPLE_RATE = 44_100
DURATION_SCALE = 0.9
SEED = 1234
NATIVE_VOICE = "eng_f5"
NARRATED_VOICE = "ru_m5"


def write_wav(path: Path, samples: np.ndarray) -> None:
    pcm = np.clip(samples, -1.0, 1.0)
    pcm = (pcm * 32767.0).astype("<i2")
    with wave.open(str(path), "wb") as out:
        out.setnchannels(1)
        out.setsampwidth(2)
        out.setframerate(SAMPLE_RATE)
        out.writeframes(pcm.tobytes())


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--snapshot", type=Path, required=True)
    parser.add_argument("--bundle", type=Path, required=True)
    parser.add_argument("--inputs", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()

    sys.path.insert(0, str(args.snapshot.resolve()))
    import teratts  # noqa: PLC0415 - the path is only known at run time

    loaded = teratts.load_model(
        args.bundle.resolve(),
        model="distilled",
        threads=4,
        russian_stress=False,
    )
    rows = json.loads(args.inputs.read_text())
    args.out.mkdir(parents=True, exist_ok=True)
    for row in rows:
        kind = row["kind"]
        native_text = f"<en>{row['raw']}</en>"
        native = teratts.generate_speech(
            loaded,
            native_text,
            NATIVE_VOICE,
            duration_scale=DURATION_SCALE,
            seed=SEED,
        )
        write_wav(args.out / f"{kind}-native.wav", native)
        narrated = teratts.generate_speech(
            loaded,
            f"<ru>{row['narrated']}</ru>",
            NARRATED_VOICE,
            duration_scale=DURATION_SCALE,
            seed=SEED,
        )
        write_wav(args.out / f"{kind}-narrated.wav", narrated)
        print(f"{kind}: native {len(native) / SAMPLE_RATE:.2f}s, narrated {len(narrated) / SAMPLE_RATE:.2f}s")
    print(f"wrote {len(rows) * 2} WAVs to {args.out}")


if __name__ == "__main__":
    main()
