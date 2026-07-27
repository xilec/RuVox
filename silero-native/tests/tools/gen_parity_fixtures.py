#!/usr/bin/env python3
"""Generate golden parity fixtures for the Rust engine end-to-end suite.

Reference waveforms come from the Python ONNX pipeline (the same
onnxruntime build the Rust engine links against), NOT from torch
`apply_tts`: the torch-vs-ONNX float drift reaches ~4e-3 on some phrases
(e.g. num_ops) and is already pinned by the exporter self-check on its own
phrase set. This suite pins the Rust engine against the bundle it ships.
The measured torch-vs-ONNX drift is recorded per case as informational
`torch_max_abs_diff`. The accented `spoken_text` reference comes from the
torch frontend (deterministic; the ONNX path uses the same frontend code).

References are taken AFTER `unpack_q_model()` (done inside `load_pack`) —
the quantized homosolver resolves stress differently.

Requires the exported ONNX bundle (default `tmp/bundle-v5`, override with
--bundle); export it first via `silero-native/export` if absent.

Run: `silero-native/tests/tools/regenerate_fixtures.sh`, or directly:
nix develop -c bash -c "cd silero-native/export && uv run python ../tests/tools/gen_parity_fixtures.py"
"""
import argparse
import json
import wave
from pathlib import Path

import numpy as np
import onnxruntime as ort

from _common import REPO, load_pack
from export import build_model_input

# (id, text, extra (speaker, rate) variants beyond the default aidar/48000)
PHRASES = [
    # tech text, already transliterated to Cyrillic by the app pipeline
    ("tech_api", "Вызови функцию гет юзер дата через эй пи ай и сохрани результат в кэш.", [("xenia", 24000)]),
    ("tech_http", "Сервер вернул ошибку пятьсот три на запрос к базе данных.", []),
    ("tech_script", "Запусти скрипт пай тест с флагом минус икс эс перед коммитом.", []),
    ("tech_config", "Конфигурация лежит в файле конфиг точка томл в домашней директории.", []),
    # numbers (pre-verbalized, as passed to the engine)
    ("num_years", "В тысяча девятьсот восемьдесят четвёртом году вышло две тысячи двадцать четыре номера журнала.", []),
    ("num_ops", "Процессор выполнил миллион двести тысяч операций за одну секунду.", []),
    # homographs
    ("homo_zamok", "Я уже стою у большого замка, но ключ от старого замка потерял.", [("xenia", 24000)]),
    ("homo_atlas", "Атлас лежал на столе рядом с картой атласа.", []),
    ("homo_gotov", "Готов ли ты к этому? Да, готов.", []),
    # ё handling
    ("yo_ezhik", "Ёжик в тумане нашёл ёлку и съел всё.", [("xenia", 24000), ("aidar", 8000)]),
    ("yo_vse", "Всё будет хорошо, я всё понял давно.", []),
    # punctuation
    ("punct_stop", "Стоп! Кто идёт? Отвечай быстро: друг, враг; время — деньги...", [("xenia", 24000)]),
    ("punct_poekhali", "Ну что, поехали? Поехали!", []),
    # long sentences
    ("long_engineer", "После долгой и тщательной проверки всех подсистем инженер наконец подтвердил, что обновление прошло успешно и никаких ошибок в журнале не обнаружено.", [("xenia", 24000)]),
    ("long_bridge", "Несмотря на сильный дождь, строительство моста через реку продолжалось все выходные без перерыва.", []),
    # single words
    ("word_privet", "Привет.", [("xenia", 24000)]),
    ("word_da", "Да.", []),
    ("word_experiment", "Эксперимент.", []),
    # mixed / misc
    ("misc_breath", "Сделай глубокий вдох, медленно выдохни и повтори упражнение ещё три раза.", []),
    ("misc_window", "Открой окно, пожалуйста, здесь очень душно.", []),
    ("misc_pilot", "Пилот самолёта доложил диспетчеру о готовности к вылету.", []),
    # explicit stress marker must win over automatic placement
    ("stress_marker", "Ударение ставится явно: з+амок.", []),
    ("misc_rainbow", "Каждый охотник желает знать, где сидит фазан.", []),
    ("misc_coffee", "Утром я выпил чашку кофе и сразу сел за работу.", []),
]


def save_wav(path: Path, audio: np.ndarray, sample_rate: int):
    # Truncation, exactly like upstream save_wav / export.py self-check.
    pcm = (np.clip(audio, -1.0, 1.0) * 32767).astype(np.int16)
    with wave.open(str(path), "wb") as wf:
        wf.setnchannels(1)
        wf.setsampwidth(2)
        wf.setframerate(sample_rate)
        wf.writeframes(pcm.tobytes())


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--model-path",
        default=None,
        help="path to v5_ru.pt (default: torch hub cache, then models.yml download)",
    )
    parser.add_argument(
        "--bundle",
        default=str(REPO / "tmp" / "bundle-v5"),
        help="path to the exported ONNX bundle (default: tmp/bundle-v5)",
    )
    args = parser.parse_args()

    bundle = Path(args.bundle)
    if not (bundle / "manifest.json").is_file():
        raise SystemExit(
            f"bundle not found at {bundle} — export it first "
            "(see silero-native/export/README.md) or pass --bundle"
        )
    out_dir = REPO / "silero-native/tests/fixtures/parity"

    providers = ["CPUExecutionProvider"]
    sess_main = ort.InferenceSession(str(bundle / "tts_main.onnx"), providers=providers)
    sess_istft = ort.InferenceSession(str(bundle / "istft.onnx"), providers=providers)
    sess_pqmf = {
        24000: ort.InferenceSession(str(bundle / "pqmf_24k.onnx"), providers=providers),
        8000: ort.InferenceSession(str(bundle / "pqmf_8k.onnx"), providers=providers),
    }

    pack = load_pack(args.model_path)
    accentor = pack.accentor  # SileroStress

    def spoken_text(text: str) -> str:
        """The accented sentence the model input sequence is built from."""
        sentence, _clean, _has_text = pack.prepare_text_input(text)
        return accentor(
            sentence,
            put_stress=True,
            put_stress_homo=True,
            put_yo=True,
            put_yo_homo=True,
            stress_single_vowel=True,
        )

    def onnx_tts(text: str, speaker: str, sample_rate: int) -> np.ndarray:
        """The reference ONNX pipeline: tts_main -> istft -> (pqmf)."""
        sequence, sp_ids, durs_rate, pitch_coefs = build_model_input(pack, text, speaker)
        mag, x, y, _dur_hat = sess_main.run(
            None,
            {
                "sequence": sequence.numpy(),
                "speaker_ids": sp_ids.numpy(),
                "durs_rate": durs_rate.numpy(),
                "pitch_coefs": pitch_coefs.numpy(),
            },
        )
        (audio,) = sess_istft.run(None, {"mag": mag, "x": x, "y": y})
        if sample_rate != 48000:
            (band,) = sess_pqmf[sample_rate].run(None, {"audio": audio.reshape(1, 1, -1)})
            audio = band.reshape(1, -1)
        return audio[0]

    out_dir.mkdir(parents=True, exist_ok=True)
    cases = []
    for phrase_id, text, extra in PHRASES:
        spoken = spoken_text(text)
        for speaker, rate in [("aidar", 48000)] + extra:
            onnx_audio = onnx_tts(text, speaker, rate)
            torch_audio = pack.apply_tts(text=text, speaker=speaker, sample_rate=rate).numpy()
            n = min(len(onnx_audio), len(torch_audio))
            torch_diff = float(np.abs(torch_audio[:n] - onnx_audio[:n]).max()) if n else float("inf")
            wav_name = f"{phrase_id}_{speaker}_{rate}.wav"
            save_wav(out_dir / wav_name, onnx_audio, rate)
            cases.append(
                {
                    "id": phrase_id,
                    "input": text,
                    "speaker": speaker,
                    "sample_rate": rate,
                    "spoken_text": spoken,
                    "wav": wav_name,
                    "samples": int(onnx_audio.shape[-1]),
                    "torch_max_abs_diff": torch_diff,
                    "torch_len_diff": abs(len(torch_audio) - len(onnx_audio)),
                }
            )
            print(f"[gen] {wav_name}: {len(onnx_audio)} samples, torch drift {torch_diff:.2e}")

    (out_dir / "parity.json").write_text(
        json.dumps({"cases": cases}, ensure_ascii=False, indent=1), encoding="utf-8"
    )
    print(f"wrote {len(cases)} cases to {out_dir / 'parity.json'}")


if __name__ == "__main__":
    main()
