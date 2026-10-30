#!/usr/bin/env python3
"""Assemble a versioned RuVox bundle from a TeraSpace/TeraTTSv2 snapshot.

The bundle is what the Rust engine loads and what the downloader fetches: the
four ONNX graphs the distilled model needs, the character indexer, the ten voice
styles, the RUAccent dictionary assets, the upstream licenses, and a
`manifest.json` that records every file's size and sha256 plus the upstream
revision. Everything is verified before the manifest is written, so a broken
snapshot fails here instead of at runtime.

Usage:

    uv run python export.py --snapshot <hf-snapshot> --out ../../tmp/teratts-bundle
    uv run python export.py --download --out ../../tmp/teratts-bundle

The `--download` path fetches only the files the bundle needs (not the 483 MB
RUAccent neural assets and not the teacher sampler) at the pinned revision.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import shutil
import subprocess
import sys
import tempfile
from datetime import datetime, timezone
from pathlib import Path

import numpy as np
import onnx

FORMAT_VERSION = 1
UPSTREAM_REPO = "TeraSpace/TeraTTSv2"
UPSTREAM_REVISION = "75983ac96062591d06ecd2ddb2aefb894104bc9e"
MODEL_ID = "terattsv2-distilled"
DEFAULT_MODEL_VERSION = "2026-10"

SAMPLE_RATE = 44_100

# Bundle-relative paths. The teacher sampler and ruaccent/nn are deliberately
# absent: the distilled sampler is the shipped one and dictionary mode needs no
# neural RUAccent assets.
MODEL_FILES = [
    "models/text_encoder.onnx",
    "models/duration_predictor.onnx",
    "models/sampler_distilled_cfg3_8step.onnx",
    "models/vocoder.onnx",
]
INDEXER = "unicode_indexer.json"
DICTIONARY_FILES = [
    "ruaccent/dictionary/accents.json.gz",
    "ruaccent/dictionary/omographs.json.gz",
    "ruaccent/dictionary/yo_words.json.gz",
    "ruaccent/dictionary/yo_homographs.json.gz",
]
LICENSE_FILES = ["LICENSE", "LICENSE-OPENRAIL", "RUACCENT_NOTICE.txt"]

VOICES = {
    "ru_f1": "ru",
    "ru_f2": "ru",
    "ru_m1": "ru",
    "ru_m5": "ru",
    "eng_f3": "en",
    "eng_f4_whisper": "en",
    "eng_f5": "en",
    "eng_m2_whisper": "en",
    "eng_m3": "en",
    "eng_m4": "en",
}

# Expected graph I/O, checked against the files so a silently changed upstream
# export cannot ship with the wrong tensor names.
EXPECTED_IO = {
    "models/text_encoder.onnx": (
        {"text_ids", "style_ttl", "text_mask"},
        {"output"},
    ),
    "models/duration_predictor.onnx": (
        {"text_ids", "style_dp", "text_mask"},
        {"output"},
    ),
    "models/sampler_distilled_cfg3_8step.onnx": (
        {"initial_latent", "text_emb", "style_ttl", "latent_mask", "text_mask", "guidance"},
        {"latent"},
    ),
    "models/vocoder.onnx": (
        {"latent"},
        {"output"},
    ),
}

EXPECTED_STYLE_SHAPES = {
    "style_ttl.npy": (1, 50, 256),
    "style_dp.npy": (1, 8, 16),
}


def fail(message: str) -> None:
    print(f"error: {message}", file=sys.stderr)
    raise SystemExit(1)


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def asset_name(path: str) -> str:
    """GitHub release assets are flat, so the bundle path becomes the name."""
    return path.replace("/", "__")


def download_snapshot(destination: Path) -> Path:
    from huggingface_hub import snapshot_download

    patterns = MODEL_FILES + [INDEXER] + DICTIONARY_FILES + LICENSE_FILES + ["styles/*"]
    return Path(
        snapshot_download(
            repo_id=UPSTREAM_REPO,
            revision=UPSTREAM_REVISION,
            local_dir=destination,
            allow_patterns=patterns,
        )
    )


def check_graph(snapshot: Path, relative: str) -> int:
    model = onnx.load(str(snapshot / relative), load_external_data=False)
    opsets = [entry.version for entry in model.opset_import if entry.domain in ("", "ai.onnx")]
    if len(opsets) != 1:
        fail(f"{relative}: expected exactly one default-domain opset, got {opsets}")
    inputs = {value.name for value in model.graph.input}
    outputs = {value.name for value in model.graph.output}
    expected_inputs, expected_outputs = EXPECTED_IO[relative]
    if inputs != expected_inputs:
        fail(f"{relative}: unexpected inputs {sorted(inputs)}")
    if outputs != expected_outputs:
        fail(f"{relative}: unexpected outputs {sorted(outputs)}")
    return opsets[0]


def check_indexer(snapshot: Path) -> None:
    table = json.loads((snapshot / INDEXER).read_text())
    if not isinstance(table, list) or len(table) != 65_536:
        fail(f"{INDEXER}: expected 65,536 entries, got {len(table) if isinstance(table, list) else type(table)}")


def check_styles(snapshot: Path) -> None:
    for voice in VOICES:
        for name, shape in EXPECTED_STYLE_SHAPES.items():
            path = snapshot / "styles" / voice / name
            if not path.is_file():
                fail(f"missing style file {path}")
            array = np.load(path)
            if array.shape != shape:
                fail(f"{path}: expected shape {shape}, got {array.shape}")


def collect_files(snapshot: Path) -> list[str]:
    files = MODEL_FILES + [INDEXER] + DICTIONARY_FILES + LICENSE_FILES
    for voice in VOICES:
        files.append(f"styles/{voice}/style_ttl.npy")
        files.append(f"styles/{voice}/style_dp.npy")
    for relative in files:
        if not (snapshot / relative).is_file():
            fail(f"missing file {relative}")
    return files


def copy_bundle(snapshot: Path, out: Path, files: list[str]) -> None:
    for relative in files:
        target = out / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(snapshot / relative, target)


def build_manifest(out: Path, files: list[str], opset: int, bundle_version: int, model_version: str) -> dict:
    entries = []
    for relative in sorted(files):
        path = out / relative
        entries.append(
            {
                "path": relative,
                "asset": asset_name(relative),
                "size": path.stat().st_size,
                "sha256": sha256_file(path),
            }
        )
    return {
        "format_version": FORMAT_VERSION,
        "bundle_version": bundle_version,
        "model_id": MODEL_ID,
        "model_version": model_version,
        "upstream_repo": UPSTREAM_REPO,
        "upstream_revision": UPSTREAM_REVISION,
        "opset": opset,
        "sample_rate": SAMPLE_RATE,
        "created_at_utc": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "voices": [{"id": voice, "language": language} for voice, language in VOICES.items()],
        "files": entries,
    }


def publish(out: Path, manifest: dict, tag: str, notes_file: Path) -> None:
    with tempfile.TemporaryDirectory(dir=out.parent, prefix=".upload-") as staging:
        staging_path = Path(staging)
        assets = []
        for entry in manifest["files"]:
            source = out / entry["path"]
            target = staging_path / entry["asset"]
            target.hardlink_to(source)
            assets.append(str(target))
        notes = notes_file.read_text()
        subprocess.run(
            ["gh", "release", "create", tag, "--title", f"TeraTTSv2 model bundle ({tag})", "--notes", notes],
            check=True,
        )
        subprocess.run(["gh", "release", "upload", tag, *assets], check=True)
        subprocess.run(["gh", "release", "upload", tag, str(out / "manifest.json")], check=True)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    source = parser.add_mutually_exclusive_group(required=True)
    source.add_argument("--snapshot", type=Path, help="local Hugging Face snapshot directory")
    source.add_argument("--download", action="store_true", help="download the pinned revision")
    parser.add_argument("--out", type=Path, required=True, help="bundle output directory")
    parser.add_argument("--bundle-version", type=int, default=1)
    parser.add_argument("--model-version", default=DEFAULT_MODEL_VERSION)
    parser.add_argument("--force", action="store_true", help="replace a non-empty output directory")
    parser.add_argument("--publish-tag", help="create a GitHub release with this tag and upload the bundle")
    parser.add_argument("--notes-file", type=Path, default=Path(__file__).with_name("RELEASE_NOTES.md"))
    args = parser.parse_args()

    out: Path = args.out.resolve()
    if out.exists() and any(out.iterdir()) and not args.force:
        fail(f"{out} is not empty; pass --force to replace it")

    snapshot = args.snapshot.resolve() if args.snapshot else download_snapshot(out.parent / f".hf-{UPSTREAM_REVISION[:8]}")
    if not snapshot.is_dir():
        fail(f"snapshot directory does not exist: {snapshot}")

    opset = check_graph(snapshot, MODEL_FILES[0])
    for relative in MODEL_FILES[1:]:
        if check_graph(snapshot, relative) != opset:
            fail(f"{relative}: opset differs from {MODEL_FILES[0]}")
    check_indexer(snapshot)
    check_styles(snapshot)
    files = collect_files(snapshot)

    if out.exists() and args.force:
        shutil.rmtree(out)
    out.mkdir(parents=True, exist_ok=True)
    copy_bundle(snapshot, out, files)
    manifest = build_manifest(out, files, opset, args.bundle_version, args.model_version)
    (out / "manifest.json").write_text(json.dumps(manifest, indent=2, ensure_ascii=False) + "\n")

    total = sum(entry["size"] for entry in manifest["files"])
    print(f"bundle: {out}")
    print(f"files:  {len(manifest['files'])} (+ manifest.json)")
    print(f"size:   {total / (1024 * 1024):.1f} MiB")
    print(f"opset:  {opset}")
    print(f"voices: {', '.join(VOICES)}")

    if args.publish_tag:
        if not args.notes_file.is_file():
            fail(f"release notes file not found: {args.notes_file}")
        publish(out, manifest, args.publish_tag, args.notes_file)
        print(f"published: {args.publish_tag}")


if __name__ == "__main__":
    main()
