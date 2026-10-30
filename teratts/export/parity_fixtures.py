#!/usr/bin/env python3
"""Generate the Python-side parity fixtures for the Rust inference core.

The Rust unit test `parity::matches_python_fixtures` replays these arrays
through its own ONNX sessions and compares the text embedding, the raw
duration, the sampled latent and the first audio samples. The reference is the
upstream `teratts.py` from the pinned snapshot, not a reimplementation, so a
layout/transpose/scaling bug on either side shows up as a mismatch.

Usage (inside the repo dev shell or any venv with onnxruntime + numpy):

    uv run python parity_fixtures.py \
      --snapshot /path/to/hf/snapshot \
      --bundle ../../tmp/teratts-bundle \
      --out ../tests/fixtures

The bundle must be the same version the test runs against; the test reads the
metadata JSON next to the arrays and is skipped when the directory is absent.
"""

from __future__ import annotations

import argparse
import json
import math
import sys
from pathlib import Path

import numpy as np

SAMPLE_RATE = 44_100
SAMPLES_PER_FRAME = 3_072
SPEED = 1.05
DURATION_SCALE = 0.9
GUIDANCE = 3.0
SEED = 1234
VOICE = "ru_m5"
TEXT = "<ru>Привет, мир. Это проверка паритета.</ru>"
AUDIO_SAMPLES = 8192


def write_f32(path: Path, array: np.ndarray) -> None:
    np.ascontiguousarray(array, dtype="<f4").tofile(path)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--snapshot", type=Path, required=True)
    parser.add_argument("--bundle", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()

    sys.path.insert(0, str(args.snapshot.resolve()))
    import teratts  # noqa: PLC0415 - the path is only known at run time

    bundle = args.bundle.resolve()
    loaded = teratts.load_model(
        bundle,
        model="distilled",
        threads=1,
        russian_stress=False,
    )

    text_ids, text_mask = loaded.indexer.batch(TEXT)
    style_dir = bundle / "styles" / VOICE
    style_ttl = np.load(style_dir / "style_ttl.npy").astype(np.float32)
    style_dp = np.load(style_dir / "style_dp.npy").astype(np.float32)

    text_emb = loaded.text_encoder.run(
        None,
        {"text_ids": text_ids, "style_ttl": style_ttl, "text_mask": text_mask},
    )[0]
    raw_duration = float(
        loaded.duration_predictor.run(
            None,
            {"text_ids": text_ids, "style_dp": style_dp, "text_mask": text_mask},
        )[0][0]
    )
    duration_seconds = raw_duration * DURATION_SCALE / SPEED
    frames = max(
        1,
        math.ceil(duration_seconds * SAMPLE_RATE / SAMPLES_PER_FRAME),
    )
    maximum_samples = round(duration_seconds * SAMPLE_RATE)
    initial_latent = (
        np.random.default_rng(SEED)
        .standard_normal((1, 144, frames))
        .astype(np.float32)
    )
    latent_mask = np.ones((1, 1, frames), dtype=np.float32)
    sampled_latent = loaded.sampler.run(
        None,
        {
            "initial_latent": initial_latent,
            "text_emb": text_emb,
            "style_ttl": style_ttl,
            "latent_mask": latent_mask,
            "text_mask": text_mask,
            "guidance": np.asarray([GUIDANCE], dtype=np.float32),
        },
    )[0]
    audio = np.concatenate(
        list(
            teratts.iter_vocoder_audio(
                loaded.vocoder,
                sampled_latent,
                chunk_frames=16,
                maximum_samples=maximum_samples,
            )
        )
    )

    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    write_f32(out / "text_emb.bin", text_emb)
    write_f32(out / "initial_latent.bin", initial_latent)
    write_f32(out / "sampler_latent.bin", sampled_latent)
    write_f32(out / "audio.bin", audio[:AUDIO_SAMPLES])
    metadata = {
        "text": TEXT,
        "voice": VOICE,
        "duration_scale": DURATION_SCALE,
        "guidance": GUIDANCE,
        "seed": SEED,
        "raw_duration": raw_duration,
        "duration_seconds": duration_seconds,
        "frames": frames,
        "maximum_samples": maximum_samples,
        "text_ids": text_ids.ravel().tolist(),
        "text_emb_shape": list(text_emb.shape),
        "sampler_latent_shape": list(sampled_latent.shape),
        "audio_samples": min(AUDIO_SAMPLES, audio.shape[0]),
    }
    (out / "parity.json").write_text(json.dumps(metadata, indent=2) + "\n")
    print(
        f"wrote {out}: {len(text_ids.ravel())} tokens, {frames} frames, "
        f"{metadata['audio_samples']} audio samples"
    )


if __name__ == "__main__":
    main()
