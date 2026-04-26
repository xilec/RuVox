"""Unit tests for timestamps.py — no torch required."""

from ttsd.timestamps import estimate_timestamps_chunked, extract_words_with_positions


class TestExtractWords:
    def test_simple_sentence(self):
        words = extract_words_with_positions("привет мир")
        assert len(words) == 2
        assert words[0] == ("привет", 0, 6)
        assert words[1] == ("мир", 7, 10)

    def test_punctuation_excluded(self):
        words = extract_words_with_positions("раз, два, три.")
        assert [w for w, _, _ in words] == ["раз", "два", "три"]

    def test_empty_string(self):
        assert extract_words_with_positions("") == []

    def test_only_punctuation(self):
        assert extract_words_with_positions("...!?") == []

    def test_positions_correct(self):
        text = "abc def"
        words = extract_words_with_positions(text)
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

    def test_char_mapping_dict_shape(self):
        # Dict shape: {"char_map": [[orig_start, orig_end], ...]} indexed by norm pos
        text = "ab"
        # norm positions 0,1 → orig positions 10,11; norm positions 1,2 → 11,12
        char_map = [[10, 12], [11, 13]]
        mapping = {"char_map": char_map}
        result = estimate_timestamps_chunked(text, [(0, len(text), 1.0)], mapping)
        assert len(result) == 1
        # norm_start=0, norm_end=2 → start_idx=0, end_idx=1 → orig_start=10, orig_end=13
        assert result[0].original_pos == (10, 13)

    def test_char_mapping_span_list_of_dicts(self):
        # Shape 1: list of dicts with norm_start/norm_end/orig_start/orig_end
        text = "слово"
        spans = [
            {"norm_start": 0, "norm_end": 5, "orig_start": 100, "orig_end": 105},
        ]
        result = estimate_timestamps_chunked(text, [(0, 5, 1.0)], spans)
        assert len(result) == 1
        assert result[0].original_pos == (100, 105)

    def test_char_mapping_positional_list(self):
        # Shape 3: plain list [[orig_start, orig_end], ...]
        text = "abc"
        char_map = [[5, 6], [6, 7], [7, 8]]
        result = estimate_timestamps_chunked(text, [(0, 3, 1.0)], char_map)
        assert len(result) == 1
        # norm_start=0, norm_end=3 → start_idx=0, end_idx=2 → orig [5,8]
        assert result[0].original_pos == (5, 8)

    def test_timestamps_are_pydantic_models(self):
        from ttsd.protocol import WordTimestamp

        text = "тест"
        result = estimate_timestamps_chunked(text, [(0, 4, 1.0)], None)
        assert len(result) == 1
        assert isinstance(result[0], WordTimestamp)
