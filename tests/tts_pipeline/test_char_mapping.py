"""Integration tests for CharMapping in TTS pipeline.

Tests that character-level position mapping correctly tracks all transformations
through the pipeline, ensuring char_map length matches transformed text length.
"""

import pytest

from ruvox.tts_pipeline import TTSPipeline


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
            f"char_map length ({len(mapping.char_map)}) != transformed length ({len(mapping.transformed)})"
        )

    def test_word_position_mapping(self, pipeline):
        """Verify word positions map back to original text correctly."""
        text = "Привет Docker мир"
        normalized, mapping = pipeline.process_with_char_mapping(text)

        assert len(mapping.char_map) == len(mapping.transformed)

        # Find "докер" in normalized text
        docker_pos = normalized.lower().find("докер")
        if docker_pos >= 0:
            orig_start, orig_end = mapping.get_original_range(docker_pos, docker_pos + len("докер"))
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


class TestPreprocessingTracking:
    """Tests that preprocessing changes are tracked correctly."""

    @pytest.fixture
    def pipeline(self):
        return TTSPipeline()

    def test_multiple_spaces_tracked(self, pipeline):
        """Multiple spaces should be collapsed but positions tracked."""
        text = "Привет   мир"  # 3 spaces
        normalized, mapping = pipeline.process_with_char_mapping(text)

        # mapping.original should be the TRUE original
        assert mapping.original == text
        assert len(mapping.original) == 12

        # "мир" is at position 9 in original (after 3 spaces)
        assert text[9:12] == "мир"

        # In normalized, "мир" is at position 7 (after 1 space)
        mir_pos = normalized.find("мир")
        assert mir_pos == 7

        # Mapping should correctly point back to original
        orig_start, orig_end = mapping.get_original_range(mir_pos, mir_pos + 3)
        assert text[orig_start:orig_end] == "мир"
        assert orig_start == 9

    def test_tabs_converted_to_space(self, pipeline):
        """Tab characters should be converted but tracked."""
        text = "Привет\tмир"
        normalized, mapping = pipeline.process_with_char_mapping(text)

        assert mapping.original == text
        # "мир" at position 7 in original (after tab)
        assert text[7:10] == "мир"

        # In normalized, tab becomes space
        mir_pos = normalized.find("мир")
        orig_start, orig_end = mapping.get_original_range(mir_pos, mir_pos + 3)
        assert text[orig_start:orig_end] == "мир"

    def test_special_quotes_tracked(self, pipeline):
        """Special quote characters should be normalized but tracked."""
        text = "«Привет» мир"
        normalized, mapping = pipeline.process_with_char_mapping(text)

        assert mapping.original == text
        # "мир" at position 10 in original
        mir_pos_orig = text.find("мир")

        mir_pos_norm = normalized.find("мир")
        orig_start, orig_end = mapping.get_original_range(mir_pos_norm, mir_pos_norm + 3)
        assert text[orig_start:orig_end] == "мир"
        assert orig_start == mir_pos_orig

    def test_em_dash_tracked(self, pipeline):
        """Em-dash should be normalized but tracked."""
        text = "Привет — мир"  # em-dash
        normalized, mapping = pipeline.process_with_char_mapping(text)

        assert mapping.original == text
        mir_pos_orig = text.find("мир")

        mir_pos_norm = normalized.find("мир")
        orig_start, orig_end = mapping.get_original_range(mir_pos_norm, mir_pos_norm + 3)
        assert text[orig_start:orig_end] == "мир"

    def test_multiple_newlines_collapsed(self, pipeline):
        """Multiple newlines should be collapsed but tracked."""
        text = "Привет\n\n\n\nмир"  # 4 newlines
        normalized, mapping = pipeline.process_with_char_mapping(text)

        assert mapping.original == text
        mir_pos_orig = text.find("мир")
        assert mir_pos_orig == 10  # after 4 newlines

        mir_pos_norm = normalized.find("мир")
        orig_start, orig_end = mapping.get_original_range(mir_pos_norm, mir_pos_norm + 3)
        assert text[orig_start:orig_end] == "мир"
        assert orig_start == mir_pos_orig

    def test_bom_removed_tracked(self, pipeline):
        """BOM character should be removed but tracked."""
        text = "\ufeffПривет мир"  # BOM at start
        normalized, mapping = pipeline.process_with_char_mapping(text)

        assert mapping.original == text
        # "Привет" at position 1 in original (after BOM)
        assert text[1:7] == "Привет"

        # In normalized, no BOM
        assert not normalized.startswith("\ufeff")
        privet_pos = normalized.find("Привет")
        assert privet_pos == 0

        # Mapping should point back correctly
        orig_start, orig_end = mapping.get_original_range(0, 6)
        assert text[orig_start:orig_end] == "Привет"
        assert orig_start == 1  # After BOM

    def test_all_preprocessing_combined(self, pipeline):
        """Combined preprocessing should maintain correct mapping."""
        # BOM + special quotes + multiple spaces + em-dash
        text = "\ufeff«Привет»   мир — всё"
        normalized, mapping = pipeline.process_with_char_mapping(text)

        assert mapping.original == text

        # Check each word maps correctly
        for word in ["Привет", "мир", "всё"]:
            # Find in normalized
            norm_pos = normalized.find(word)
            if norm_pos < 0:
                # Word might be changed (e.g., всё -> все)
                continue

            # Map back to original
            orig_start, orig_end = mapping.get_original_range(norm_pos, norm_pos + len(word))
            assert text[orig_start:orig_end] == word, (
                f"Word '{word}' at norm_pos {norm_pos} "
                f"mapped to [{orig_start}:{orig_end}] = '{text[orig_start:orig_end]}'"
            )


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
            orig_start, orig_end = mapping.get_original_range(docker_pos, docker_pos + 5)
            assert text[orig_start:orig_end] == "Docker"


class TestQtPositionCorrespondence:
    """Tests that Python string positions match Qt QTextDocument positions.

    When we use setPlainText() in Qt, we expect character positions to be 1:1
    with Python string indices. These tests verify this assumption.
    """

    @pytest.fixture
    def qt_app(self):
        """Create QApplication for Qt tests."""
        import sys

        from PyQt6.QtWidgets import QApplication

        # Check if app already exists
        app = QApplication.instance()
        if app is None:
            app = QApplication(sys.argv)
        return app

    @pytest.fixture
    def text_document(self, qt_app):
        """Create a QTextDocument for testing."""
        from PyQt6.QtGui import QTextDocument

        return QTextDocument()

    def test_simple_text_positions(self, text_document):
        """Simple text should have matching positions."""
        text = "Привет мир"
        text_document.setPlainText(text)

        qt_text = text_document.toPlainText()
        assert qt_text == text
        assert len(qt_text) == len(text)

        # Check each character position
        for i, char in enumerate(text):
            assert qt_text[i] == char

    def test_text_with_multiple_spaces(self, text_document):
        """Multiple spaces should be preserved in plain text mode."""
        text = "Привет   мир"  # 3 spaces
        text_document.setPlainText(text)

        qt_text = text_document.toPlainText()
        assert qt_text == text
        assert len(qt_text) == len(text)

        # Verify "мир" position
        assert qt_text[9:12] == "мир"
        assert text[9:12] == "мир"

    def test_text_with_newlines(self, text_document):
        """Newlines should be preserved."""
        text = "Привет\nмир\nвсё"
        text_document.setPlainText(text)

        qt_text = text_document.toPlainText()
        assert qt_text == text
        assert len(qt_text) == len(text)

    def test_text_with_tabs(self, text_document):
        """Tab characters should be preserved."""
        text = "Привет\tмир"
        text_document.setPlainText(text)

        qt_text = text_document.toPlainText()
        assert qt_text == text
        assert len(qt_text) == len(text)

    def test_unicode_characters(self, text_document):
        """Unicode characters should have correct positions."""
        text = "Привет → мир"  # Arrow is single character
        text_document.setPlainText(text)

        qt_text = text_document.toPlainText()
        assert qt_text == text
        assert len(qt_text) == len(text)

        # Arrow is at position 7
        assert qt_text[7] == "→"
        assert text[7] == "→"

    def test_special_quotes(self, text_document):
        """Special quote characters should be preserved."""
        text = "«Привет» мир"
        text_document.setPlainText(text)

        qt_text = text_document.toPlainText()
        assert qt_text == text
        assert len(qt_text) == len(text)

    def test_em_dash(self, text_document):
        """Em-dash should be preserved."""
        text = "Привет — мир"  # em-dash
        text_document.setPlainText(text)

        qt_text = text_document.toPlainText()
        assert qt_text == text
        assert len(qt_text) == len(text)

    def test_bom_character_stripped_by_qt(self, text_document):
        """Qt strips BOM character - this is expected behavior.

        IMPORTANT: Qt's QTextDocument.setPlainText() automatically removes
        the BOM character. This means we must strip BOM from text before
        storing it, otherwise positions will be off by 1.
        """
        text_with_bom = "\ufeffПривет мир"
        text_without_bom = "Привет мир"

        text_document.setPlainText(text_with_bom)
        qt_text = text_document.toPlainText()

        # Qt removes BOM!
        assert qt_text == text_without_bom
        assert len(qt_text) == len(text_with_bom) - 1

        # This is why we strip BOM in storage.add_entry()

    def test_cursor_position_matches_string_index(self, text_document):
        """QTextCursor positions should match Python string indices."""
        from PyQt6.QtGui import QTextCursor

        text = "Привет мир и всё"
        text_document.setPlainText(text)

        cursor = QTextCursor(text_document)

        # Move to position 7 (should be "м" from "мир")
        cursor.setPosition(7)
        cursor.setPosition(10, QTextCursor.MoveMode.KeepAnchor)

        selected = cursor.selectedText()
        assert selected == text[7:10]
        assert selected == "мир"

    def test_highlighting_position_accuracy(self, text_document):
        """Simulates highlighting and verifies position accuracy."""
        from PyQt6.QtGui import QColor, QTextCharFormat, QTextCursor

        text = "Привет   мир  и  всё"  # Multiple spaces
        text_document.setPlainText(text)

        # Create highlight format
        highlight_format = QTextCharFormat()
        highlight_format.setBackground(QColor("#FFFF00"))

        # Highlight "мир" at its actual position (9-12)
        mir_start = text.find("мир")
        mir_end = mir_start + len("мир")
        assert mir_start == 9

        cursor = QTextCursor(text_document)
        cursor.setPosition(mir_start)
        cursor.setPosition(mir_end, QTextCursor.MoveMode.KeepAnchor)

        # Verify selection
        assert cursor.selectedText() == "мир"

    def test_complex_text_positions(self, text_document):
        """Complex text with various unicode should maintain positions."""
        # Note: No BOM here since Qt strips it
        text = "«Привет»   мир — всё → конец"
        text_document.setPlainText(text)

        qt_text = text_document.toPlainText()
        assert qt_text == text
        assert len(qt_text) == len(text)

        # Verify each word can be found at correct position
        for word in ["Привет", "мир", "всё", "конец"]:
            py_pos = text.find(word)
            qt_pos = qt_text.find(word)
            assert py_pos == qt_pos, f"Word '{word}' position mismatch: Python={py_pos}, Qt={qt_pos}"
