#!/usr/bin/env python3
"""Generate golden fixtures for the Rust port of `prepare_text_input`.

Runs the real upstream frontend from the v5_ru torch.package model and dumps
(input, sentence, clean_sentence, has_text, sequence) tuples to
`silero-native/tests/fixtures/frontend/prepare_text_input.json`.

Sequence ids follow the engine contract (chars without a symbol id are
dropped); upstream `preprocess_tacotron` would KeyError on those, so no
upstream reference exists for that step — the sequence here is derived
mechanically from the upstream-produced sentence.

Run: `silero-native/tests/tools/regenerate_fixtures.sh`, or directly:
nix develop -c bash -c "cd silero-native/export && uv run python ../tests/tools/gen_frontend_fixtures.py"
"""
import argparse
import json

from _common import REPO, load_pack

INPUTS = [
    "Привет, World! 123",
    "раз—два‑три–четыре",
    "12345 !!!",
    "Ёлка и ёжик",
    "з+амок и зам+ок",
    "слово^",
    "^^сло^^во^ ^",
    "а  ^  б",
    "Много   пробелов\tи\nпереносов",
    "API и getUserData() через API",
    "",
    "   ",
    "Очень ДЛИННАЯ Фраза С Разным Регистром!",
]


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--model-path",
        default=None,
        help="path to v5_ru.pt (default: torch hub cache, then models.yml download)",
    )
    args = parser.parse_args()

    pack = load_pack(args.model_path)

    fixtures = []
    for text in INPUTS:
        sentence, clean_sentence, has_text = pack.prepare_text_input(text)
        full = pack.sos_token + sentence + pack.eos_token
        sequence = [pack.symbol_to_id[c] for c in full if c in pack.symbol_to_id]
        fixtures.append(
            {
                "input": text,
                "sentence": sentence,
                "clean_sentence": clean_sentence,
                "has_text": has_text,
                "sequence": sequence,
            }
        )

    payload = {
        "symbols": pack.symbols,
        "sos_token": pack.sos_token,
        "eos_token": pack.eos_token,
        "symbol_to_id": pack.symbol_to_id,
        "cases": fixtures,
    }
    out = REPO / "silero-native/tests/fixtures/frontend/prepare_text_input.json"
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(payload, ensure_ascii=False, indent=1))
    print(f"wrote {len(fixtures)} fixtures to {out}")


if __name__ == "__main__":
    main()
