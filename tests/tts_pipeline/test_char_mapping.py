"""Integration tests for CharMapping in TTS pipeline.

Tests that character-level position mapping correctly tracks all transformations
through the pipeline, ensuring char_map length matches transformed text length.
"""

import pytest
from fast_tts_rus.tts_pipeline import TTSPipeline


class TestCharMappingConsistency:
    """Tests for char_map and transformed text length consistency."""

    @pytest.fixture
    def pipeline(self):
        return TTSPipeline()

    def test_simple_text(self, pipeline):
        """Simple Russian text should have identity mapping."""
        text = "Привет мир"
        normalized, mapping = pipeline.process_with_char_mapping(text)
        assert len(mapping.char_map) == len(mapping.transformed)

    def test_english_word(self, pipeline):
        """English word expansion should maintain correct mapping."""
        text = "Используем Docker"
        normalized, mapping = pipeline.process_with_char_mapping(text)
        assert len(mapping.char_map) == len(mapping.transformed)
        assert "докер" in normalized.lower()

    def test_arrow_symbol(self, pipeline):
        """Arrow symbol replacement should maintain correct mapping."""
        text = '"NVIDIA" → "эн"'
        normalized, mapping = pipeline.process_with_char_mapping(text)
        assert len(mapping.char_map) == len(mapping.transformed)
        assert "стрелка" in normalized.lower()

    def test_arrow_with_spaces(self, pipeline):
        """Arrow with surrounding spaces should not cause mismatch."""
        text = "A → B"
        normalized, mapping = pipeline.process_with_char_mapping(text)
        assert len(mapping.char_map) == len(mapping.transformed)

    def test_multiple_arrows(self, pipeline):
        """Multiple arrows should all be handled correctly."""
        text = "A → B → C → D"
        normalized, mapping = pipeline.process_with_char_mapping(text)
        assert len(mapping.char_map) == len(mapping.transformed)

    def test_operators(self, pipeline):
        """Operators should be replaced without causing mismatch."""
        text = "if x >= 10 && y <= 20"
        normalized, mapping = pipeline.process_with_char_mapping(text)
        assert len(mapping.char_map) == len(mapping.transformed)
        assert "больше или равно" in normalized
        assert "меньше или равно" in normalized

    def test_code_identifiers(self, pipeline):
        """CamelCase and snake_case identifiers should map correctly."""
        text = "Функция getUserData вызывает get_user_info"
        normalized, mapping = pipeline.process_with_char_mapping(text)
        assert len(mapping.char_map) == len(mapping.transformed)

    def test_numbers_and_percentages(self, pipeline):
        """Numbers and percentages should expand correctly."""
        text = "Прогресс 50% завершён, осталось 100 файлов"
        normalized, mapping = pipeline.process_with_char_mapping(text)
        assert len(mapping.char_map) == len(mapping.transformed)

    def test_abbreviations(self, pipeline):
        """Abbreviations should expand to letter-by-letter spelling."""
        text = "Используем API и HTTP протокол"
        normalized, mapping = pipeline.process_with_char_mapping(text)
        assert len(mapping.char_map) == len(mapping.transformed)

    def test_markdown_list(self, pipeline):
        """Markdown numbered list should convert correctly."""
        text = """Шаги:
1. Первый шаг
2. Второй шаг
3. Третий шаг"""
        normalized, mapping = pipeline.process_with_char_mapping(text)
        assert len(mapping.char_map) == len(mapping.transformed)

    def test_complex_text_with_code(self, pipeline):
        """Complex text with various replacements should maintain consistency."""
        text = """Баг с перекрытием замен в CharMapping

Проблема

При нормализации текста для TTS происходит множество замен:
- "NVIDIA" → "эн ви ай ди ай эй"
- "5.2-Codex" → "пять точка два Codex"
- "25%" → "двадцать пять процентов"

После основных замен выполняется постобработка — нормализация пробелов (r" +" → " "). Проблема возникала, когда эта нормализация находила пробелы внутри уже заменённого текста.

Например:
1. "5.2-Codex" заменяется на "пять точка два Codex" (позиция в оригинале: [105:114])
2. Позже нормализация пробелов находит пробел внутри "пять точка два Codex"
3. _current_to_original() преобразует эту позицию обратно в оригинальные координаты и возвращает [105:105] (точка внутри замены)
4. Эта "замена пробела" записывалась как отдельная запись с orig_start=105

В результате при построении char_map в build_mapping() регион [105:114] обрабатывался дважды, что приводило к несоответствию длин: char_map имел 290 записей, а transformed текст — 286 символов.

Из-за этого третье слово "Модель" (позиция 165) отображалось на позицию 161, и подсветка показывала "е.\\n\\nМо" вместо "Модель".

Решение

Добавил проверку перекрытия в координатах оригинального текста:

def _find_containing_replacement(self, orig_start: int, orig_end: int) -> OffsetEntry | None:
    for entry in self._offset_entries:
        if orig_start == orig_end:
            if entry.orig_start <= orig_start < entry.orig_end:
                return entry
        else:
            if orig_start < entry.orig_end and entry.orig_start < orig_end:
                return entry
    return None

В методе sub() теперь:
containing_entry = self._find_containing_replacement(orig_start, orig_end)

if containing_entry is not None:
    continue

Если новая замена находится внутри уже существующей, она полностью пропускается."""

        normalized, mapping = pipeline.process_with_char_mapping(text)
        assert len(mapping.char_map) == len(mapping.transformed), (
            f"char_map length ({len(mapping.char_map)}) != "
            f"transformed length ({len(mapping.transformed)})"
        )

    def test_word_position_mapping(self, pipeline):
        """Verify word positions map back to original text correctly."""
        text = "Привет Docker мир"
        normalized, mapping = pipeline.process_with_char_mapping(text)

        assert len(mapping.char_map) == len(mapping.transformed)

        # Find "докер" in normalized text
        docker_pos = normalized.lower().find("докер")
        if docker_pos >= 0:
            orig_start, orig_end = mapping.get_original_range(
                docker_pos, docker_pos + len("докер")
            )
            # Should map back to "Docker" in original
            assert text[orig_start:orig_end] == "Docker"

    def test_greek_letters(self, pipeline):
        """Greek letters should expand without mapping issues."""
        text = "Функция f(α) = α² + β"
        normalized, mapping = pipeline.process_with_char_mapping(text)
        assert len(mapping.char_map) == len(mapping.transformed)
        assert "альфа" in normalized.lower()

    def test_math_symbols(self, pipeline):
        """Math symbols should expand correctly."""
        text = "Если x ≥ 0 и y ≤ 10"
        normalized, mapping = pipeline.process_with_char_mapping(text)
        assert len(mapping.char_map) == len(mapping.transformed)

    def test_version_numbers(self, pipeline):
        """Version numbers should be read correctly."""
        text = "Версия 2.3.1 выпущена"
        normalized, mapping = pipeline.process_with_char_mapping(text)
        assert len(mapping.char_map) == len(mapping.transformed)

    def test_empty_text(self, pipeline):
        """Empty text should return empty mapping."""
        text = ""
        normalized, mapping = pipeline.process_with_char_mapping(text)
        assert normalized == ""
        assert len(mapping.char_map) == 0

    def test_whitespace_only(self, pipeline):
        """Whitespace-only text should return empty after strip."""
        text = "   \n\t  "
        normalized, mapping = pipeline.process_with_char_mapping(text)
        assert normalized == ""


class TestCharMappingRanges:
    """Tests for get_original_range functionality."""

    @pytest.fixture
    def pipeline(self):
        return TTSPipeline()

    def test_identity_range(self, pipeline):
        """Unchanged text should have identity range mapping."""
        text = "Простой текст"
        normalized, mapping = pipeline.process_with_char_mapping(text)

        # First word "Простой"
        orig_start, orig_end = mapping.get_original_range(0, 7)
        assert orig_start == 0
        assert orig_end == 7

    def test_expanded_word_range(self, pipeline):
        """Expanded word should map entire range to original word."""
        text = "API"
        normalized, mapping = pipeline.process_with_char_mapping(text)

        # "API" expands to "эй пи ай"
        assert len(mapping.char_map) == len(normalized)

        # All characters should map back to original [0:3]
        for i in range(len(normalized)):
            orig_start, orig_end = mapping.get_original_range(i, i + 1)
            assert orig_start >= 0
            assert orig_end <= 3

    def test_mixed_content_range(self, pipeline):
        """Mixed Russian and English should have correct ranges."""
        text = "Привет Docker"
        normalized, mapping = pipeline.process_with_char_mapping(text)

        # "Привет" at start
        orig_start, orig_end = mapping.get_original_range(0, 6)
        assert text[orig_start:orig_end] == "Привет"

        # Find "докер"
        docker_pos = normalized.lower().find("докер")
        if docker_pos >= 0:
            orig_start, orig_end = mapping.get_original_range(
                docker_pos, docker_pos + 5
            )
            assert text[orig_start:orig_end] == "Docker"
