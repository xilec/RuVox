#!/usr/bin/env python
"""Generate golden fixtures from legacy pipeline for Rust pipeline tests."""

import json
import sys
from pathlib import Path

# Add legacy to sys.path so we can import ruvox.tts_pipeline
sys.path.insert(0, str(Path(__file__).parent.parent / "legacy" / "src"))

from ruvox.tts_pipeline import TTSPipeline

FIXTURES_DIR = Path(__file__).parent.parent / "src-tauri" / "tests" / "fixtures" / "pipeline"

TEST_CASES: dict[str, str] = {
    # Numbers
    "number_plain": "Было 42 яблока.",
    "number_large": "Население 100500 человек.",
    "number_decimal": "Скорость 3.14 метра в секунду.",
    # Sizes and durations
    "size_mb": "Файл весит 100MB.",
    "size_gb": "Диск на 5GB.",
    "size_kb": "Документ занимает 512KB.",
    "duration_ms": "Задержка 300ms.",
    # Versions
    "version_semver": "Версия v1.2.3.",
    "version_prerelease": "Сборка 1.0.0-alpha.",
    "version_patch": "Обновление до v2.0.1.",
    # Ranges
    "range_years": "С 1990 по 2020 годы.",
    "range_simple": "Диапазон 10-20 страниц.",
    # Percentages
    "percentage_int": "Скидка 50%.",
    "percentage_decimal": "Точность 99.5%.",
    # English words and IT terms
    "english_word_common": "Нужно сделать download и install.",
    "english_word_it": "Это API и HTTP-протокол.",
    # Abbreviations
    "abbreviation_upper": "Использую JSON и XML.",
    "abbreviation_http": "Запрос через HTTP и HTTPS.",
    # Code identifiers
    "camelcase": "Функция getUserData принимает id.",
    "pascalcase": "Класс UserController наследуется.",
    "snake_case": "Переменная user_id теперь nullable.",
    "kebab_case": "Параметр max-width установлен.",
    # URLs and network
    "url_https": "Репозиторий https://github.com/snakers4/silero-models .",
    "email": "Пишите на user@example.com .",
    "ip_address": "Сервер 192.168.1.1 доступен.",
    # File paths
    "filepath_absolute": "Файл /home/user/docs/file.pdf.",
    # Symbols
    "greek_letters": "Формула α + β = γ.",
    "math_symbols": "Условие x <= y.",
    # Operators
    "operators": "Код a != b && c == d.",
    "arrow_symbols": "Переход A → B.",
    # Markdown structures
    "markdown_header": "# Заголовок\nТекст.",
    "markdown_list_number": "1. Первое\n2. Второе\n3. Третье.",
    "markdown_inline_code": "Переменная `count` больше 10.",
    "markdown_code_block": "```python\ndef hello():\n    print('hi')\n```",
    "markdown_mermaid": "```mermaid\ngraph TD\nA-->B\n```",
    "markdown_link": "Ссылка [GitHub](https://github.com).",
    # Complex mixed content
    "mixed_paragraph": (
        "Установите Docker версии >= 20.10 с https://docker.com "
        "и проверьте getUserData() возвращает корректный JSON с полем user_id."
    ),
}


def main() -> int:
    pipeline = TTSPipeline()
    FIXTURES_DIR.mkdir(parents=True, exist_ok=True)
    errors = []
    for slug, text in TEST_CASES.items():
        try:
            normalized, char_mapping = pipeline.process_with_char_mapping(text)
            (FIXTURES_DIR / f"{slug}.input.txt").write_text(text, encoding="utf-8")
            (FIXTURES_DIR / f"{slug}.expected.txt").write_text(normalized, encoding="utf-8")
            char_map_json = {
                "original": char_mapping.original,
                "transformed": char_mapping.transformed,
                "char_map": char_mapping.char_map,
            }
            (FIXTURES_DIR / f"{slug}.char_map.json").write_text(
                json.dumps(char_map_json, ensure_ascii=False, indent=2),
                encoding="utf-8",
            )
            print(f"OK {slug}")
        except Exception as exc:
            errors.append((slug, exc))
            print(f"ERROR {slug}: {exc}", file=sys.stderr)

    print(f"\nGenerated {len(TEST_CASES) - len(errors)} fixtures in {FIXTURES_DIR}")
    if errors:
        print(f"Errors: {len(errors)}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
