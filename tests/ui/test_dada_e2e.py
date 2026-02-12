"""E2E test for Dada article highlighting - полная проверка от TTS до UI."""

import pytest
from PyQt6.QtWidgets import QApplication

from fast_tts_rus.ui.widgets.text_viewer import TextViewerWidget, TextFormat
from fast_tts_rus.ui.models.entry import TextEntry
from fast_tts_rus.tts_pipeline import TTSPipeline
import re


# Dada article из бага
DADA_ARTICLE = """Вот перевод статьи на русский язык:

---

# Привет, Dada!

Продолжая мой пост [Fun with Dada](https://smallcultfollowing.com/babysteps/blog/2026/02/08/fun-with-dada/), в этой статье я начну обучать языку Dada. Я буду держать каждый пост коротким — по сути, только то, что я могу написать, пока пью утренний кофе.

## У вас есть право писать код

Вот самая первая программа на Dada:

```
println("Hello, Dada!")
```

Думаю, все вы сможете догадаться, что она делает. Тем не менее, даже в этой простой программе есть кое-что, на что стоит обратить внимание:"""


@pytest.fixture(scope="module")
def qapp():
    """Create QApplication instance for tests."""
    app = QApplication.instance()
    if app is None:
        app = QApplication([])
    yield app


@pytest.fixture
def text_viewer(qapp):
    """Create TextViewerWidget instance."""
    viewer = TextViewerWidget()
    yield viewer
    viewer.deleteLater()


def extract_words_with_positions(text: str) -> list[tuple[str, int, int]]:
    """Extract words from text with their positions (как в tts_worker.py)."""
    words = []
    for match in re.finditer(r'\b\w+\b', text):
        words.append((match.group(), match.start(), match.end()))
    return words


def test_dada_e2e_full_pipeline(text_viewer):
    """E2E тест: TTS pipeline → timestamps → UI highlighting.

    Этот тест воспроизводит полный пайплайн действий пользователя:
    1. Генерация timestamps через TTS pipeline
    2. Отображение в TextViewer (Markdown mode)
    3. Симуляция воспроизведения с подсветкой
    4. Проверка что все слова подсвечиваются (включая ссылку)
    """
    # STEP 1: TTS Pipeline - генерация timestamps
    pipeline = TTSPipeline()
    normalized, char_mapping = pipeline.process_with_char_mapping(DADA_ARTICLE)

    # URL не должен быть в нормализованном тексте
    assert "smallcultfollowing" not in normalized.lower()
    assert "babysteps" not in normalized.lower()

    # Текст ссылки должен быть транслитерирован (TTS не читает английский)
    assert "fun" not in normalized.lower(), "English 'Fun' should be transliterated"
    assert "фан" in normalized.lower(), "'Fun' should become 'фан'"

    # Extract words (как делает tts_worker)
    norm_words = extract_words_with_positions(normalized)

    # Generate mock timestamps
    timestamps = []
    for i, (word, norm_start, norm_end) in enumerate(norm_words):
        orig_start, orig_end = char_mapping.get_original_range(norm_start, norm_end)
        timestamps.append({
            "word": word,
            "start": i * 0.5,
            "end": (i + 1) * 0.5,
            "original_pos": [orig_start, orig_end]
        })

    # STEP 2: UI Setup - TextViewer в Markdown режиме
    entry = TextEntry(original_text=DADA_ARTICLE)
    text_viewer.set_format(TextFormat.MARKDOWN)
    text_viewer.set_entry(entry, timestamps)

    # STEP 3: Симуляция воспроизведения - все слова должны подсвечиваться
    highlighted_count = 0
    for ts in timestamps:
        time_pos = ts["start"] + 0.1
        text_viewer.highlight_at_position(time_pos)
        selections = text_viewer.extraSelections()
        if selections:
            highlighted_count += 1

    # Все слова должны иметь маппинг и подсвечиваться
    assert highlighted_count == len(timestamps), (
        f"Только {highlighted_count}/{len(timestamps)} слов подсвечено"
    )

    # STEP 4: Проверка конкретных слов

    # "обучать" - должно быть на правильной позиции
    obuchat_ts = [ts for ts in timestamps if ts["word"] == "обучать"]
    assert obuchat_ts, "Слово 'обучать' не найдено в timestamps"
    orig_start, orig_end = obuchat_ts[0]["original_pos"]
    expected_start = DADA_ARTICLE.find("обучать языку Dada")
    assert orig_start == expected_start and orig_end == expected_start + 7, (
        f"'обучать' mapped to [{orig_start}:{orig_end}], expected [{expected_start}:{expected_start+7}]"
    )

    # "дада" из ссылки [Fun with Dada] - маппинг должен быть точный
    # (указывать на "Dada", а не на весь "[Fun with Dada](url)")
    link_dada = [ts for ts in timestamps if ts["word"] == "дада"]
    assert link_dada, "Слово 'дада' не найдено в timestamps"
    # Первое вхождение "дада" — из ссылки
    orig_start, orig_end = link_dada[0]["original_pos"]
    orig_text = DADA_ARTICLE[orig_start:orig_end]
    assert orig_text == "Dada", (
        f"'дада' from link mapped to '{orig_text}' instead of 'Dada'"
    )


def test_words_after_link_are_highlighted(text_viewer):
    """Слова после Markdown-ссылки должны подсвечиваться.

    Баг: после [Fun with Dada](url) слова "в этой статье я начну обучать..."
    не подсвечивались, потому что их original_pos указывал за пределы
    rendered текста (из-за удалённого URL).
    """
    pipeline = TTSPipeline()
    normalized, char_mapping = pipeline.process_with_char_mapping(DADA_ARTICLE)

    norm_words = extract_words_with_positions(normalized)
    timestamps = []
    for i, (word, norm_start, norm_end) in enumerate(norm_words):
        orig_start, orig_end = char_mapping.get_original_range(norm_start, norm_end)
        timestamps.append({
            "word": word,
            "start": i * 0.5,
            "end": (i + 1) * 0.5,
            "original_pos": [orig_start, orig_end]
        })

    entry = TextEntry(original_text=DADA_ARTICLE)
    text_viewer.set_format(TextFormat.MARKDOWN)
    text_viewer.set_entry(entry, timestamps)

    # Проверяем конкретные слова ПОСЛЕ ссылки
    words_after_link = ["этой", "статье", "начну", "обучать", "языку",
                        "буду", "держать", "каждый", "пост", "коротким",
                        "могу", "написать", "пока", "пью", "кофе"]
    not_highlighted = []
    for target_word in words_after_link:
        ts = next((t for t in timestamps if t["word"] == target_word), None)
        assert ts is not None, f"Слово '{target_word}' не найдено в timestamps"

        text_viewer.highlight_at_position(ts["start"] + 0.1)
        selections = text_viewer.extraSelections()
        if not selections:
            orig_s, orig_e = ts["original_pos"]
            not_highlighted.append(
                f"'{target_word}' orig[{orig_s}:{orig_e}]="
                f"'{DADA_ARTICLE[orig_s:orig_e]}'"
            )

    assert not not_highlighted, (
        f"Слова после ссылки не подсвечены:\n  " + "\n  ".join(not_highlighted)
    )


if __name__ == "__main__":
    pytest.main([__file__, "-v", "-s"])
