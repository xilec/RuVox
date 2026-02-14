"""Tests for TextViewer highlighting - regression test for repeated words bug.

Bug description: In Markdown mode, document.find(word) searches from the
beginning and returns the FIRST match. When "Модель" appears multiple times,
the second and third occurrences highlight the first one instead.
"""

import pytest


class TestHighlightBugReproduction:
    """Reproduce the highlight bug without requiring Qt widgets.

    The bug is in TextViewerWidget._highlight_range() which uses:
        cursor = self.document().find(word)

    This always returns the FIRST occurrence of the word, regardless
    of the position parameter passed to _highlight_range().
    """

    def test_find_nth_occurrence_logic(self):
        """Test that we can find the Nth occurrence of a word."""
        text = """Модель обучалась на системах NVIDIA GB200.

Модель объединяет возможности GPT-5.2-Codex.

Модель может работать часами."""

        # Find all occurrences
        word = "Модель"
        positions = []
        pos = 0
        while True:
            idx = text.find(word, pos)
            if idx == -1:
                break
            positions.append(idx)
            pos = idx + 1

        print(f"\n'Модель' positions: {positions}")
        assert len(positions) == 3

        # Verify positions
        assert positions[0] == 0
        # Second "Модель" is after first paragraph
        assert text[positions[1]:positions[1] + 6] == "Модель"
        assert text[positions[2]:positions[2] + 6] == "Модель"

    def test_bug_demonstration(self):
        """Demonstrate the bug: find() always returns first match.

        This shows why the current implementation is broken.
        """
        text = "Модель один. Модель два. Модель три."

        # Using str.find() correctly:
        # To find the 2nd "Модель", we'd need to search from after the first
        first_pos = text.find("Модель")
        second_pos = text.find("Модель", first_pos + 1)
        third_pos = text.find("Модель", second_pos + 1)

        print(f"\nPositions: first={first_pos}, second={second_pos}, third={third_pos}")
        print(f"Text at positions: '{text[first_pos:first_pos+6]}', "
              f"'{text[second_pos:second_pos+6]}', '{text[third_pos:third_pos+6]}'")

        # Verify all positions point to "Модель"
        assert text[first_pos:first_pos + 6] == "Модель"
        assert text[second_pos:second_pos + 6] == "Модель"
        assert text[third_pos:third_pos + 6] == "Модель"

        # The bug: if we just call find("Модель") it always returns first_pos
        # So highlighting the 2nd occurrence at second_pos would
        # actually highlight position first_pos


class TestTextViewerHighlightFix:
    """Test that the fix correctly highlights Nth occurrence."""

    @pytest.fixture
    def mock_text_viewer(self):
        """Create a mock that simulates TextViewerWidget behavior."""
        class MockTextViewer:
            def __init__(self):
                self.original_text = ""
                self._last_highlighted_pos = None
                self.highlighted_positions = []

            def set_text(self, text):
                self.original_text = text

            def find_nth_occurrence(self, word: str, n: int) -> int:
                """Find the Nth (0-indexed) occurrence of word in text.

                This is what _highlight_range should use instead of
                document.find() which only returns the first match.
                """
                pos = 0
                for i in range(n + 1):
                    idx = self.original_text.find(word, pos)
                    if idx == -1:
                        return -1
                    if i == n:
                        return idx
                    pos = idx + 1
                return -1

            def find_occurrence_by_position(self, word: str, target_pos: int) -> int:
                """Find which occurrence of word is at target_pos.

                Returns the occurrence index (0-indexed) or -1 if not found.
                This can be used to find the correct occurrence in rendered HTML.
                """
                pos = 0
                occurrence = 0
                while True:
                    idx = self.original_text.find(word, pos)
                    if idx == -1:
                        return -1
                    if idx == target_pos:
                        return occurrence
                    if idx > target_pos:
                        return -1
                    occurrence += 1
                    pos = idx + 1
                return -1

            def highlight_word_at_position_fixed(self, word: str, target_pos: int):
                """Fixed implementation: highlight correct occurrence.

                Instead of document.find(word), we:
                1. Find which occurrence (0-indexed) the target_pos corresponds to
                2. Use find() with start position to find Nth occurrence in rendered doc
                """
                occurrence_idx = self.find_occurrence_by_position(word, target_pos)
                if occurrence_idx == -1:
                    return None

                # Simulate finding Nth occurrence in rendered document
                # (In real code, this would iterate document.find() N times)
                actual_pos = self.find_nth_occurrence(word, occurrence_idx)
                self._last_highlighted_pos = (actual_pos, actual_pos + len(word))
                self.highlighted_positions.append(self._last_highlighted_pos)
                return actual_pos

        return MockTextViewer()

    def test_highlight_second_occurrence(self, mock_text_viewer):
        """Test that second occurrence is correctly identified and highlighted."""
        text = "Модель один. Модель два. Модель три."
        mock_text_viewer.set_text(text)

        # Target: highlight "Модель" at position 13 (second occurrence)
        target_pos = 13

        # Verify the word is actually at that position
        assert text[target_pos:target_pos + 6] == "Модель"

        # Use the fixed method
        result = mock_text_viewer.highlight_word_at_position_fixed("Модель", target_pos)

        print(f"\nTarget position: {target_pos}")
        print(f"Highlighted position: {result}")

        # Should highlight at position 13, NOT 0
        assert result == 13, f"Expected to highlight at 13, got {result}"
        assert mock_text_viewer._last_highlighted_pos == (13, 19)

    def test_highlight_third_occurrence(self, mock_text_viewer):
        """Test that third occurrence is correctly identified and highlighted."""
        text = "Модель один. Модель два. Модель три."
        mock_text_viewer.set_text(text)

        # Find actual third position
        pos = 0
        for _ in range(3):
            pos = text.find("Модель", pos)
            if _ < 2:
                pos += 1
        target_pos = pos

        assert text[target_pos:target_pos + 6] == "Модель", \
            f"Expected 'Модель' at {target_pos}, got '{text[target_pos:target_pos + 6]}'"

        result = mock_text_viewer.highlight_word_at_position_fixed("Модель", target_pos)

        assert result == target_pos, f"Expected to highlight at {target_pos}, got {result}"

    def test_highlight_all_occurrences_sequentially(self, mock_text_viewer):
        """Test highlighting each occurrence in sequence."""
        text = """Модель обучалась на системах NVIDIA.

Модель объединяет возможности GPT.

Модель может работать часами."""

        mock_text_viewer.set_text(text)

        # Find all "Модель" positions
        positions = []
        pos = 0
        while True:
            idx = text.find("Модель", pos)
            if idx == -1:
                break
            positions.append(idx)
            pos = idx + 1

        print(f"\n'Модель' positions in text: {positions}")

        # Highlight each one and verify
        for i, target_pos in enumerate(positions):
            mock_text_viewer.highlighted_positions.clear()
            result = mock_text_viewer.highlight_word_at_position_fixed("Модель", target_pos)

            print(f"Occurrence {i + 1}: target={target_pos}, highlighted={result}")

            assert result == target_pos, \
                f"Occurrence {i + 1}: expected position {target_pos}, got {result}"

    def test_occurrence_index_calculation(self, mock_text_viewer):
        """Test that occurrence index is correctly calculated from position."""
        text = "A B A C A D"
        mock_text_viewer.set_text(text)

        # "A" appears at positions 0, 4, 8
        assert mock_text_viewer.find_occurrence_by_position("A", 0) == 0
        assert mock_text_viewer.find_occurrence_by_position("A", 4) == 1
        assert mock_text_viewer.find_occurrence_by_position("A", 8) == 2
        assert mock_text_viewer.find_occurrence_by_position("A", 2) == -1  # Not at "A"


class TestIntegrationWithTimestamps:
    """Integration test: verify timestamps + highlighting work together."""

    def test_timestamps_to_correct_highlight(self):
        """End-to-end: timestamps should lead to correct highlight positions."""
        from ruvox.tts_pipeline import TTSPipeline
        from ruvox.ui.services.tts_worker import TTSRunnable
        from ruvox.ui.models.entry import TextEntry, EntryStatus

        # Use the actual article text that caused the bug
        original = """Модель обучалась на системах NVIDIA GB200.

Модель объединяет возможности GPT-5.2-Codex.

Модель может работать часами."""

        # Process through pipeline
        pipeline = TTSPipeline()
        normalized, char_mapping = pipeline.process_with_char_mapping(original)

        # Create runnable for timestamp estimation
        entry = TextEntry(id="test", original_text=original, status=EntryStatus.PENDING)

        class MockConfig:
            speaker = "xenia"
            sample_rate = 48000

        runnable = TTSRunnable(
            entry=entry,
            config=MockConfig(),
            storage=None,
            silero_model=None,
        )

        # Generate timestamps
        chunk_durations = [(0, len(normalized), 10.0)]
        timestamps = runnable._estimate_timestamps_chunked(
            normalized, chunk_durations, char_mapping
        )

        # Find all "Модель" in original text
        expected_positions = []
        pos = 0
        while True:
            idx = original.find("Модель", pos)
            if idx == -1:
                break
            expected_positions.append(idx)
            pos = idx + 1

        print(f"\nExpected 'Модель' positions: {expected_positions}")

        # Find "Модель" entries in timestamps
        model_timestamps = [ts for ts in timestamps if ts["word"] == "Модель"]

        print(f"Found {len(model_timestamps)} 'Модель' timestamps:")
        for i, ts in enumerate(model_timestamps):
            print(f"  {i + 1}: original_pos={ts['original_pos']}, "
                  f"expected={expected_positions[i] if i < len(expected_positions) else 'N/A'}")

        # Verify each timestamp points to correct position
        assert len(model_timestamps) == len(expected_positions), \
            f"Expected {len(expected_positions)} 'Модель' timestamps, got {len(model_timestamps)}"

        for i, ts in enumerate(model_timestamps):
            actual_start = ts["original_pos"][0]
            expected_start = expected_positions[i]
            assert actual_start == expected_start, \
                f"'Модель' #{i + 1}: expected position {expected_start}, got {actual_start}"
