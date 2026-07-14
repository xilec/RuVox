"""Unit tests for timestamps.py — no torch required."""

import pytest

from ttsd.protocol import CharMappingEntry
from ttsd.timestamps import (
    _map_via_positional,
    _map_via_spans,
    estimate_timestamps_chunked,
    extract_words_with_positions,
)


class TestExtractWords:
    @pytest.mark.parametrize(
        "text, expected_words",
        [
            pytest.param("привет мир", ["привет", "мир"], id="simple_sentence"),
            pytest.param("раз, два, три.", ["раз", "два", "три"], id="punctuation_excluded"),
            pytest.param("", [], id="empty_string"),
            pytest.param("...!?", [], id="only_punctuation"),
            pytest.param("abc def", ["abc", "def"], id="positions_correct"),
        ],
    )
    def test_extract_words(self, text, expected_words):
        words = extract_words_with_positions(text)
        assert [w for w, _, _ in words] == expected_words
        # Every returned span must round-trip to its own word in the source.
        for word, start, end in words:
            assert text[start:end] == word


class TestEstimateTimestampsChunked:
    def test_empty_text_returns_empty(self):
        result = estimate_timestamps_chunked("", [], None)
        assert result == []

    def test_chunk_with_no_words(self):
        result = estimate_timestamps_chunked("...", [(0, 3, 1.0)], None)
        assert result == []

    def test_single_chunk_two_words_proportional(self):
        # "аб" (2 chars) and "абвгд" (5 chars), total=7 chars, duration=7.0s
        # word1 duration = 2/7 * 7 = 2.0s, word2 = 5/7 * 7 = 5.0s
        text = "аб абвгд"
        result = estimate_timestamps_chunked(text, [(0, len(text), 7.0)], None)

        assert len(result) == 2
        w1, w2 = result

        assert w1.word == "аб"
        assert w1.start == 0.0
        assert abs(w1.end - 2.0) < 0.01

        assert w2.word == "абвгд"
        assert abs(w2.start - 2.0) < 0.01
        assert abs(w2.end - 7.0) < 0.01

    def test_two_chunks_audio_offset(self):
        text = "один два три четыре"
        # Two chunks: [0..7] duration 2.0s and [8..19] duration 4.0s
        chunks = [(0, 7, 2.0), (8, len(text), 4.0)]
        result = estimate_timestamps_chunked(text, chunks, None)

        # All words from first chunk must end before 2.0s
        first_chunk_words = [ts for ts in result if ts.end <= 2.001]
        # All words from second chunk must start >= 2.0s
        second_chunk_words = [ts for ts in result if ts.start >= 1.999]

        assert len(first_chunk_words) >= 1
        assert len(second_chunk_words) >= 1

    def test_without_char_mapping_positions_are_normalized(self):
        text = "слово"
        result = estimate_timestamps_chunked(text, [(0, len(text), 1.0)], None)
        assert len(result) == 1
        ts = result[0]
        assert ts.original_pos == (0, 5)

    @pytest.mark.parametrize(
        "text, chunks, mapping, expected_pos",
        [
            pytest.param(
                "ab",
                [(0, 2, 1.0)],
                {"char_map": [[10, 12], [11, 13]]},
                (10, 13),
                id="dict_shape",
                # norm_start=0, norm_end=2 → start_idx=0, end_idx=1 → orig_start=10, orig_end=13
            ),
            pytest.param(
                "слово",
                [(0, 5, 1.0)],
                [{"norm_start": 0, "norm_end": 5, "orig_start": 100, "orig_end": 105}],
                (100, 105),
                id="span_list_of_dicts",
            ),
            pytest.param(
                "abc",
                [(0, 3, 1.0)],
                [[5, 6], [6, 7], [7, 8]],
                (5, 8),
                id="positional_list",
                # norm_start=0, norm_end=3 → start_idx=0, end_idx=2 → orig [5,8]
            ),
            pytest.param(
                "слово",
                [(0, 5, 1.0)],
                [CharMappingEntry(norm_start=0, norm_end=5, orig_start=100, orig_end=105)],
                (100, 105),
                id="pydantic_span_entries",
                # Exercises the hasattr branch of _is_span_entry (non-dict span
                # with a norm_start attribute) via real CharMappingEntry objects.
            ),
        ],
    )
    def test_char_mapping_shapes(self, text, chunks, mapping, expected_pos):
        result = estimate_timestamps_chunked(text, chunks, mapping)
        assert len(result) == 1
        assert result[0].original_pos == expected_pos

    def test_timestamps_are_pydantic_models(self):
        from ttsd.protocol import WordTimestamp

        text = "тест"
        result = estimate_timestamps_chunked(text, [(0, 4, 1.0)], None)
        assert len(result) == 1
        assert isinstance(result[0], WordTimestamp)

    def test_timestamps_rounded_to_three_decimals(self):
        # "a"(1 char) + "bb"(2 chars), total 3, duration 1.0s.
        # w1 end = round(1/3, 3) = 0.333; w2 end = round(1.0, 3) = 1.0.
        result = estimate_timestamps_chunked("a bb", [(0, 4, 1.0)], None)
        assert len(result) == 2
        assert result[0].start == 0.0
        assert result[0].end == 0.333
        assert result[1].start == 0.333
        assert result[1].end == 1.0


class TestMapViaSpans:
    def test_merges_multiple_overlapping_spans(self):
        spans = [
            {"norm_start": 0, "norm_end": 3, "orig_start": 10, "orig_end": 13},
            {"norm_start": 3, "norm_end": 6, "orig_start": 13, "orig_end": 16},
        ]
        # Range [1, 5) overlaps both spans → widen to min orig_start / max orig_end.
        assert _map_via_spans(spans, 1, 5) == (10, 16)

    def test_break_stops_at_span_past_range(self):
        # The out-of-order poison span (index 2) would widen orig_end to 999 if
        # reached, but the loop must break at the sorted span whose norm_start
        # is >= norm_end before ever getting there.
        spans = [
            {"norm_start": 0, "norm_end": 2, "orig_start": 0, "orig_end": 2},
            {"norm_start": 10, "norm_end": 12, "orig_start": 20, "orig_end": 22},
            {"norm_start": 1, "norm_end": 2, "orig_start": 500, "orig_end": 999},
        ]
        assert _map_via_spans(spans, 0, 3) == (0, 2)

    def test_no_overlap_falls_back_to_norm_positions(self):
        # All spans end at/before norm_start → no overlap → fallback (norm_start, norm_end).
        spans = [{"norm_start": 0, "norm_end": 2, "orig_start": 0, "orig_end": 2}]
        assert _map_via_spans(spans, 5, 8) == (5, 8)


class TestMapViaPositional:
    def test_empty_char_map_falls_back(self):
        assert _map_via_positional([], 3, 7) == (3, 7)

    def test_indices_clamped_within_bounds(self):
        # char_map shorter than the requested norm positions → indices clamp to
        # the last available entry.
        char_map = [[5, 6]]
        assert _map_via_positional(char_map, 10, 20) == (5, 6)

    def test_negative_norm_end_clamped_to_zero(self):
        # norm_end - 1 can be negative; end_idx must clamp up to 0.
        char_map = [[5, 6], [6, 7]]
        assert _map_via_positional(char_map, 0, 0) == (5, 6)
