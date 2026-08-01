#!/usr/bin/env python3
"""Generate golden fixtures for the Rust port of ttsd's text preprocessing.

Runs the actual `ttsd.chunking` functions and dumps input/output pairs to
`silero-native/tests/fixtures/chunking/`:

- `sanitize.json` — `sanitize_for_silero` cases (newline/space cleanup that
  the native engine must reproduce before its symbol filter, otherwise words
  across a line break get glued together);
- `split.json` — `split_into_chunks` cases (long-text chunking ported into
  the native engine's synthesize path).

Needs no model and no torch (ttsd.chunking is pure stdlib), so it runs in
any Python; the shared entry point runs it inside the export uv environment
together with the other generators.

Run: `silero-native/tests/tools/regenerate_fixtures.sh`, or directly:
nix develop -c bash -c "cd silero-native/export && uv run python ../tests/tools/gen_chunking_fixtures.py"
"""
import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPO / "ttsd"))

from ttsd.chunking import MAX_CHUNK_SIZE, sanitize_for_silero, split_into_chunks  # noqa: E402

SANITIZE_INPUTS = [
    "один\nдва",
    "один   два",
    "  текст  ",
    "абзац один.\n\n  абзац два",
    "строки\nновая",
    "а \n \n\t б",
    "\n\nтекст\n",
    "без переносов",
    "много\n\n\nпереносов\n\nподряд",
    "табы\tи  пробелы \n вперемешку\n",
]

SPLIT_INPUTS = [
    "Привет мир",
    "Это предложение. " * 60,
    "Слово слово слово. " * 60,
    "A" * 2000,
    "Без точек и запятых тут " * 60,
    "Одно. Два! Три? " * 80,
    "абзац один.\n\nабзац два. " * 50,
]


def main() -> None:
    payload = {
        "cases": [
            {"input": text, "output": sanitize_for_silero(text)}
            for text in SANITIZE_INPUTS
        ]
    }
    out = REPO / "silero-native/tests/fixtures/chunking/sanitize.json"
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(payload, ensure_ascii=False, indent=1))
    print(f"wrote {len(payload['cases'])} sanitize fixtures to {out}")

    payload = {
        "max_chunk_size": MAX_CHUNK_SIZE,
        "cases": [
            {
                "input": text,
                "chunks": [[chunk_text, start] for chunk_text, start in split_into_chunks(text)],
            }
            for text in SPLIT_INPUTS
        ],
    }
    out = REPO / "silero-native/tests/fixtures/chunking/split.json"
    out.write_text(json.dumps(payload, ensure_ascii=False, indent=1))
    print(f"wrote {len(payload['cases'])} split fixtures to {out}")


if __name__ == "__main__":
    main()
