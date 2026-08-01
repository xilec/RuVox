#!/usr/bin/env python3
"""Generate golden fixtures for the Rust accentor + homosolver port.

Runs the real upstream pipeline (SileroStress = HomoSolver -> AccentorNgram)
from the v5_ru torch.package model on top of `prepare_text_input`, and dumps
per-stage outputs so the Rust test can compare each stage separately.

References are taken AFTER `unpack_q_model()` (done inside `load_pack`) —
the quantized homosolver resolves stress differently.

Run: `silero-native/tests/tools/regenerate_fixtures.sh`, or directly:
nix develop -c bash -c "cd silero-native/export && uv run python ../tests/tools/gen_accentor_fixtures.py"
"""
import argparse
import json

from _common import REPO, load_pack

INPUTS = [
    "Открыть замок было непросто.",
    "По тропинке вдоль большого замка шла процессия.",
    "Ёлка стояла посреди двора, а ёжик под ней.",
    "Я уже всё понял про своё будущее.",
    "Поставь ударение: з+амок - это явный маркер.",
    "что-то случилось совсем недавно",
    "Атлас лежал на столе, большой атлас мира.",
    "Готов ли ты к этому? Да, готов.",
    "Мы стоим у большого замка, а ключи от замка потерялись.",
    "Привет! Это тестовый текст для проверки синтеза речи.",
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
    accentor = pack.accentor  # SileroStress

    cases = []
    for text in INPUTS:
        sentence, _clean_sentence, _has_text = pack.prepare_text_input(text)
        homosolved = accentor.homosolver(
            sentence, put_stress=True, put_yo=True, stress_single_vowel=True
        )
        accented = accentor(
            sentence,
            put_stress=True,
            put_stress_homo=True,
            put_yo=True,
            put_yo_homo=True,
            stress_single_vowel=True,
        )
        full = pack.sos_token + accented + pack.eos_token
        sequence = [pack.symbol_to_id[c] for c in full if c in pack.symbol_to_id]
        cases.append(
            {
                "input": text,
                "prepared": sentence,
                "homosolved": homosolved,
                "accented": accented,
                "sequence": sequence,
            }
        )

    out = REPO / "silero-native/tests/fixtures/frontend/accentor.json"
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps({"cases": cases}, ensure_ascii=False, indent=1))
    print(f"wrote {len(cases)} cases to {out}")
    for c in cases:
        print(" ", c["input"], "->", c["accented"])


if __name__ == "__main__":
    main()
