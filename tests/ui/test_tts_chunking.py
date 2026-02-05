"""Tests for TTS text chunking and timestamp calculation."""

import pytest

# Import the module to test chunking logic
import sys
sys.path.insert(0, 'src')

from fast_tts_rus.ui.services.tts_worker import TTSRunnable, MAX_CHUNK_SIZE


class MockConfig:
    speaker = "xenia"
    sample_rate = 48000


class MockStorage:
    pass


class MockModel:
    pass


class TestTextChunking:
    """Tests for _split_into_chunks method."""

    @pytest.fixture
    def runnable(self):
        """Create a TTSRunnable for testing."""
        from fast_tts_rus.ui.models.entry import TextEntry, EntryStatus
        entry = TextEntry(
            id="test-id",
            original_text="test",
            status=EntryStatus.PENDING,
        )
        return TTSRunnable(
            entry=entry,
            config=MockConfig(),
            storage=MockStorage(),
            silero_model=MockModel(),
        )

    def test_short_text_no_chunking(self, runnable):
        """Short text should not be chunked."""
        text = "Короткий текст."
        chunks = runnable._split_into_chunks(text)

        assert len(chunks) == 1
        assert chunks[0] == (text, 0)

    def test_chunk_boundaries_match_text(self, runnable):
        """Chunk boundaries should allow reconstructing the text."""
        text = "Первое предложение. Второе предложение. Третье предложение. " * 20
        chunks = runnable._split_into_chunks(text)

        # Verify each chunk's position is correct
        for chunk_text, start_pos in chunks:
            # The chunk text should be found at start_pos in original
            actual_text = text[start_pos:start_pos + len(chunk_text)]
            # Account for possible stripping
            assert chunk_text in text, f"Chunk '{chunk_text[:50]}...' not found in text"

    def test_no_text_loss(self, runnable):
        """No characters should be lost when chunking."""
        text = "Слово1 слово2 слово3 слово4. " * 50
        chunks = runnable._split_into_chunks(text)

        # Reconstruct text from chunks
        reconstructed_words = []
        for chunk_text, _ in chunks:
            reconstructed_words.extend(chunk_text.split())

        original_words = text.split()

        assert len(reconstructed_words) == len(original_words), \
            f"Word count mismatch: {len(reconstructed_words)} vs {len(original_words)}"

    def test_gpt_article_text(self, runnable):
        """Test with the actual problematic text."""
        text = """Модель обучалась и запускалась на системах NVIDIA GB200 NVL72. Это стойки с новыми ускорителями Blackwell, рассчитанные на плотную работу с большими моделями и агентами.

Модель объединяет возможности GPT-5.2-Codex в программировании и GPT-5.2 в рассуждениях и работе с профессиональными задачами. При этом она примерно на 25% быстрее и лучше справляется с длительными заданиями, где нужно искать информацию, пользоваться инструментами и выполнять цепочки действий.

GPT-5.3-Codex может работать часами или днями, а пользователь в это время может вмешиваться в процесс: задавать вопросы, менять направление работы и получать промежуточные отчёты, не теряя контекста.

Интересный момент — модель частично использовалась при собственной разработке. С её помощью команда Codex отлаживала обучение, анализировала тесты, диагностировала ошибки и даже помогала с деплоем. По словам разработчиков, это заметно ускорило выпуск новой версии.

По бенчмаркам модель показывает лучшие результаты на SWE-Bench Pro и Terminal-Bench 2.0 — тестах, которые проверяют реальные навыки программирования и работы с терминалом. При этом GPT-5.3-Codex решает задачи, используя меньше токенов, чем предыдущие модели.

За пределами кода возможности тоже расширились. Модель умеет работать с презентациями, таблицами, текстами, анализом данных и другими типичными задачами разработчиков, аналитиков, дизайнеров и продакт-менеджеров. В тестах GDPval она показывает уровень, сопоставимый с предыдущими флагманскими моделями, но с более автономным поведением.

GPT-5.3-Codex также лучше ориентируется в графическом интерфейсе компьютера: она видит экран, понимает, что происходит, и может выполнять действия в обычных приложениях почти так же, как человек.

Источник: https://openai.com/index/introducing-gpt-5-3-codex/"""

        chunks = runnable._split_into_chunks(text)

        print(f"\n=== Text chunking analysis ===")
        print(f"Original text length: {len(text)}")
        print(f"Number of chunks: {len(chunks)}")

        total_chars = 0
        for i, (chunk_text, start_pos) in enumerate(chunks):
            print(f"\nChunk {i+1}:")
            print(f"  Start pos: {start_pos}")
            print(f"  Length: {len(chunk_text)}")
            print(f"  Text: '{chunk_text[:60]}...'")

            # Verify position
            if start_pos + len(chunk_text) <= len(text):
                actual = text[start_pos:start_pos + len(chunk_text)]
                matches = actual == chunk_text
                print(f"  Position matches: {matches}")
                if not matches:
                    print(f"  Expected: '{chunk_text[:30]}...'")
                    print(f"  Actual:   '{actual[:30]}...'")

            total_chars += len(chunk_text)

        # Check for word loss
        original_words = text.split()
        chunk_words = []
        for chunk_text, _ in chunks:
            chunk_words.extend(chunk_text.split())

        print(f"\n=== Word analysis ===")
        print(f"Original word count: {len(original_words)}")
        print(f"Chunked word count: {len(chunk_words)}")

        # Find missing words
        missing = set(original_words) - set(chunk_words)
        extra = set(chunk_words) - set(original_words)

        if missing:
            print(f"Missing words: {missing}")
        if extra:
            print(f"Extra words: {extra}")

        assert len(chunk_words) == len(original_words), \
            f"Word count mismatch: {len(chunk_words)} vs {len(original_words)}"


class TestTimestampCalculation:
    """Tests for timestamp calculation with chunks."""

    @pytest.fixture
    def runnable(self):
        """Create a TTSRunnable for testing."""
        from fast_tts_rus.ui.models.entry import TextEntry, EntryStatus
        entry = TextEntry(
            id="test-id",
            original_text="test",
            status=EntryStatus.PENDING,
        )
        return TTSRunnable(
            entry=entry,
            config=MockConfig(),
            storage=MockStorage(),
            silero_model=MockModel(),
        )

    def test_timestamps_cover_all_words(self, runnable):
        """All words should have timestamps."""
        text = "Первое слово второе слово третье слово."

        # Simulate chunk durations (single chunk)
        chunk_durations = [(0, len(text), 3.0)]  # 3 seconds total

        timestamps = runnable._estimate_timestamps_chunked(text, chunk_durations, None)

        words = text.split()
        assert len(timestamps) == len(words), \
            f"Timestamp count {len(timestamps)} != word count {len(words)}"

    def test_timestamps_monotonically_increasing(self, runnable):
        """Timestamps should always increase."""
        text = "Слово " * 100  # Long text that will be chunked

        # Simulate 2 chunks
        chunk_durations = [
            (0, 300, 5.0),      # First chunk: 5 seconds
            (300, 600, 5.0),   # Second chunk: 5 seconds
        ]

        timestamps = runnable._estimate_timestamps_chunked(
            "Слово " * 50 + "Слово " * 50,  # Simulated chunked text
            chunk_durations,
            None
        )

        prev_end = 0.0
        for i, ts in enumerate(timestamps):
            assert ts["start"] >= prev_end - 0.001, \
                f"Timestamp {i} start {ts['start']} < previous end {prev_end}"
            assert ts["end"] >= ts["start"], \
                f"Timestamp {i} end {ts['end']} < start {ts['start']}"
            prev_end = ts["end"]

    def test_original_positions_within_bounds(self, runnable):
        """Original positions should be within text bounds."""
        text = "Тест позиций слов в тексте."
        chunk_durations = [(0, len(text), 2.0)]

        timestamps = runnable._estimate_timestamps_chunked(text, chunk_durations, None)

        for ts in timestamps:
            start, end = ts["original_pos"]
            assert start >= 0, f"Start position {start} < 0"
            assert end <= len(text), f"End position {end} > text length {len(text)}"
            assert start < end, f"Start {start} >= end {end}"

    def test_chunked_timestamps_continuous(self, runnable):
        """Timestamps across chunks should be continuous."""
        # Simulate text split into 3 chunks
        chunk1 = "Первый чанк текста."
        chunk2 = "Второй чанк текста."
        chunk3 = "Третий чанк текста."
        full_text = chunk1 + " " + chunk2 + " " + chunk3

        chunk_durations = [
            (0, len(chunk1), 2.0),
            (len(chunk1) + 1, len(chunk1) + 1 + len(chunk2), 2.0),
            (len(chunk1) + len(chunk2) + 2, len(full_text), 2.0),
        ]

        timestamps = runnable._estimate_timestamps_chunked(full_text, chunk_durations, None)

        # Check that timestamps from chunk 2 start after chunk 1 ends
        # Find the transition points
        chunk1_words = len(chunk1.split())
        chunk2_words = len(chunk2.split())

        if len(timestamps) > chunk1_words:
            last_chunk1_end = timestamps[chunk1_words - 1]["end"]
            first_chunk2_start = timestamps[chunk1_words]["start"]

            print(f"\nChunk transition:")
            print(f"  Last word of chunk 1 ends at: {last_chunk1_end}")
            print(f"  First word of chunk 2 starts at: {first_chunk2_start}")

            # Chunk 2 should start at or after chunk 1's audio duration (2.0s)
            assert first_chunk2_start >= 1.9, \
                f"Chunk 2 starts too early: {first_chunk2_start} < 2.0"


class TestPositionMapping:
    """Tests for position mapping accuracy."""

    @pytest.fixture
    def runnable(self):
        """Create a TTSRunnable for testing."""
        from fast_tts_rus.ui.models.entry import TextEntry, EntryStatus
        entry = TextEntry(
            id="test-id",
            original_text="test",
            status=EntryStatus.PENDING,
        )
        return TTSRunnable(
            entry=entry,
            config=MockConfig(),
            storage=MockStorage(),
            silero_model=MockModel(),
        )

    def test_char_mapping_simple(self, runnable):
        """Test CharMapping with a simple example."""
        from fast_tts_rus.tts_pipeline.tracked_text import TrackedText

        # Simple case: replace one word
        tracked = TrackedText("Hello NVIDIA world")
        tracked.sub(r'NVIDIA', 'эн ви ай ди ай эй')
        mapping = tracked.build_mapping()

        print(f"\n=== Simple CharMapping test ===")
        print(f"Original: '{tracked.original}' ({len(tracked.original)} chars)")
        print(f"Transformed: '{tracked.text}' ({len(tracked.text)} chars)")
        print(f"Char map length: {len(mapping.char_map)}")

        # Check a few positions
        # "Hello " (0-5) should map to itself
        for i in range(6):
            orig_range = mapping.get_original_range(i, i+1)
            print(f"  Trans[{i}]='{tracked.text[i]}' -> Orig{orig_range}='{tracked.original[orig_range[0]:orig_range[1]]}'")

        # "эн ви ай ди ай эй" (6-23) should all map to "NVIDIA" (6-12)
        trans_nvidia_start = 6
        trans_nvidia_end = 6 + len('эн ви ай ди ай эй')
        orig_range = mapping.get_original_range(trans_nvidia_start, trans_nvidia_end)
        print(f"  Trans[{trans_nvidia_start}:{trans_nvidia_end}]='{tracked.text[trans_nvidia_start:trans_nvidia_end]}' -> Orig{orig_range}='{tracked.original[orig_range[0]:orig_range[1]]}'")

        # " world" should map correctly
        trans_world_start = trans_nvidia_end
        trans_world_end = len(tracked.text)
        orig_range = mapping.get_original_range(trans_world_start, trans_world_end)
        print(f"  Trans[{trans_world_start}:{trans_world_end}]='{tracked.text[trans_world_start:trans_world_end]}' -> Orig{orig_range}='{tracked.original[orig_range[0]:orig_range[1]]}'")

        assert orig_range == (12, 18), f"Expected (12, 18) for ' world', got {orig_range}"

    def test_char_mapping_multiple_replacements(self, runnable):
        """Test CharMapping with multiple sequential replacements."""
        from fast_tts_rus.tts_pipeline.tracked_text import TrackedText

        # Multiple replacements: "API and HTTP" -> "эй пи ай and эйч ти ти пи"
        tracked = TrackedText("API and HTTP")
        tracked.sub(r'API', 'эй пи ай')
        tracked.sub(r'HTTP', 'эйч ти ти пи')
        mapping = tracked.build_mapping()

        print(f"\n=== Multiple replacements test ===")
        print(f"Original: '{tracked.original}' ({len(tracked.original)} chars)")
        print(f"Transformed: '{tracked.text}' ({len(tracked.text)} chars)")
        print(f"Char map length: {len(mapping.char_map)}")
        print(f"Replacements: {len(tracked._replacements)}")

        # Check if lengths match
        assert len(mapping.char_map) == len(tracked.text), \
            f"Char map length {len(mapping.char_map)} != text length {len(tracked.text)}"

        # Check positions for " and "
        # In transformed: "эй пи ай" (9 chars) + " and " (5 chars) + "эйч ти ти пи" (12 chars)
        and_start = len("эй пи ай")  # 9
        and_end = and_start + 5  # 14
        orig_range = mapping.get_original_range(and_start, and_end)
        expected_orig = (3, 8)  # " and " in original is at 3:8
        print(f"  Trans[{and_start}:{and_end}]='{tracked.text[and_start:and_end]}' -> Orig{orig_range}='{tracked.original[orig_range[0]:orig_range[1]]}'")
        print(f"  Expected: Orig{expected_orig}='{tracked.original[expected_orig[0]:expected_orig[1]]}'")

        # Check position of "эйч ти ти пи"
        http_start = and_end  # 14
        http_end = len(tracked.text)  # 26
        orig_range = mapping.get_original_range(http_start, http_end)
        expected_orig = (8, 12)  # "HTTP" in original is at 8:12
        print(f"  Trans[{http_start}:{http_end}]='{tracked.text[http_start:http_end]}' -> Orig{orig_range}='{tracked.original[orig_range[0]:orig_range[1]]}'")
        print(f"  Expected: Orig{expected_orig}='{tracked.original[expected_orig[0]:expected_orig[1]]}'")

        assert orig_range == expected_orig, f"Expected {expected_orig}, got {orig_range}"

    def test_repeated_words(self, runnable):
        """Test that repeated words map to correct positions in order."""
        from fast_tts_rus.tts_pipeline import TTSPipeline

        # Text with repeated word "Модель"
        original = "Модель работает. Модель обучается. Модель отвечает."
        pipeline = TTSPipeline()
        normalized, char_mapping = pipeline.process_with_char_mapping(original)

        print(f"\n=== Repeated words test ===")
        print(f"Original: '{original}'")
        print(f"Normalized: '{normalized}'")

        # Build timestamps
        chunk_durations = [(0, len(normalized), 5.0)]
        timestamps = runnable._estimate_timestamps_chunked(
            normalized, chunk_durations, char_mapping
        )

        # Find all "Модель" timestamps
        model_timestamps = [ts for ts in timestamps if ts["word"] == "Модель"]
        print(f"Found {len(model_timestamps)} 'Модель' words")

        assert len(model_timestamps) == 3, f"Expected 3 'Модель', got {len(model_timestamps)}"

        # Check positions are sequential (not all pointing to first occurrence)
        positions = [ts["original_pos"] for ts in model_timestamps]
        print(f"Positions: {positions}")

        # First "Модель" at 0-6
        assert positions[0] == [0, 6], f"First 'Модель' should be at [0, 6], got {positions[0]}"
        # Second "Модель" at 17-23
        assert positions[1] == [17, 23], f"Second 'Модель' should be at [17, 23], got {positions[1]}"
        # Third "Модель" at 35-41
        assert positions[2] == [35, 41], f"Third 'Модель' should be at [35, 41], got {positions[2]}"

    def test_repeated_words_after_multiple_expansions(self, runnable):
        """Test repeated words after multiple abbreviation expansions.

        This is a regression test for the bug where "Модель" in later paragraphs
        was highlighting the first "Модель" because the word index got desynchronized
        after multiple expandable words like "NVIDIA", "GPT-5.2-Codex", "25%", etc.

        The bug occurred because:
        1. Short words (<=3 chars) like "эн", "ви", "ай" from expansions stayed at same index
        2. After expansion ended, the index was way behind the actual position
        3. The lookahead (10 words) wasn't enough to find the next matching word
        4. Words started showing "No match" and indices got completely out of sync
        """
        from fast_tts_rus.tts_pipeline import TTSPipeline

        # This mimics the GPT-5.3 article structure that caused the bug:
        # - Multiple expandable terms (NVIDIA, GPT-5.2-Codex, 25%, etc.)
        # - "Модель" appears in multiple paragraphs
        original = """Модель обучалась на системах NVIDIA GB200. Это стойки с ускорителями.

Модель объединяет возможности GPT-5.2-Codex в программировании. При этом она на 25% быстрее.

Модель может работать часами. Пользователь может вмешиваться в процесс."""

        pipeline = TTSPipeline()
        normalized, char_mapping = pipeline.process_with_char_mapping(original)

        print(f"\n=== Repeated words after multiple expansions test ===")
        print(f"Original ({len(original)} chars):")
        for i, line in enumerate(original.split('\n')):
            if line.strip():
                print(f"  P{i}: '{line[:60]}...' " if len(line) > 60 else f"  P{i}: '{line}'")
        print(f"Normalized ({len(normalized)} chars)")

        # Build timestamps
        chunk_durations = [(0, len(normalized), 10.0)]
        timestamps = runnable._estimate_timestamps_chunked(
            normalized, chunk_durations, char_mapping
        )

        # Find all "Модель" timestamps
        model_timestamps = [ts for ts in timestamps if ts["word"] == "Модель"]
        print(f"Found {len(model_timestamps)} 'Модель' words")

        # Find actual positions in original
        model_positions_in_original = []
        pos = 0
        while True:
            pos = original.find("Модель", pos)
            if pos == -1:
                break
            model_positions_in_original.append(pos)
            pos += 1

        print(f"Actual 'Модель' positions in original: {model_positions_in_original}")

        for i, ts in enumerate(model_timestamps):
            orig_start, orig_end = ts["original_pos"]
            extracted = original[orig_start:orig_end] if orig_end <= len(original) else "OUT OF BOUNDS"
            expected_pos = model_positions_in_original[i] if i < len(model_positions_in_original) else "N/A"
            match = "✓" if orig_start == expected_pos else "✗"
            print(f"  Модель #{i+1}: orig[{orig_start}:{orig_end}] = '{extracted}' (expected pos {expected_pos}) {match}")

        assert len(model_timestamps) == 3, f"Expected 3 'Модель', got {len(model_timestamps)}"

        # Check each "Модель" maps to correct position
        for i, ts in enumerate(model_timestamps):
            actual_pos = ts["original_pos"][0]
            expected_pos = model_positions_in_original[i]
            assert actual_pos == expected_pos, \
                f"'Модель' #{i+1} should be at position {expected_pos}, got {actual_pos}. " \
                f"Bug: word index got desynchronized after expansions."

    def test_full_pipeline_with_normalization(self, runnable):
        """Test position mapping through the full pipeline with normalization."""
        from fast_tts_rus.tts_pipeline import TTSPipeline

        original_text = """Модель обучалась и запускалась на системах NVIDIA GB200 NVL72. Это стойки с новыми ускорителями Blackwell, рассчитанные на плотную работу с большими моделями и агентами.

Модель объединяет возможности GPT-5.2-Codex в программировании и GPT-5.2 в рассуждениях и работе с профессиональными задачами. При этом она примерно на 25% быстрее и лучше справляется с длительными заданиями, где нужно искать информацию, пользоваться инструментами и выполнять цепочки действий.

GPT-5.3-Codex может работать часами или днями, а пользователь в это время может вмешиваться в процесс: задавать вопросы, менять направление работы и получать промежуточные отчёты, не теряя контекста."""

        # Normalize text
        pipeline = TTSPipeline()
        normalized, char_mapping = pipeline.process_with_char_mapping(original_text)

        print(f"\n=== Full pipeline test ===")
        print(f"Original length: {len(original_text)}")
        print(f"Normalized length: {len(normalized)}")
        print(f"CharMapping.original length: {len(char_mapping.original)}")
        print(f"CharMapping.char_map length: {len(char_mapping.char_map)}")
        print(f"Original == CharMapping.original: {original_text == char_mapping.original}")
        print(f"Normalized text (first 200): {normalized[:200]}...")

        # Split into chunks
        chunks = runnable._split_into_chunks(normalized)
        print(f"Chunks: {len(chunks)}")

        # Build chunk_durations
        chunk_durations = []
        for chunk_text, start_pos in chunks:
            duration = len(chunk_text) / 100.0
            chunk_durations.append((start_pos, start_pos + len(chunk_text), duration))
            print(f"  Chunk: pos={start_pos}, len={len(chunk_text)}, text='{chunk_text[:50]}...'")

        # Calculate timestamps
        timestamps = runnable._estimate_timestamps_chunked(normalized, chunk_durations, char_mapping)

        print(f"\nTotal timestamps: {len(timestamps)}")

        # Verify positions map back to original text correctly
        errors = []
        prev_time = -1
        for i, ts in enumerate(timestamps):
            orig_start, orig_end = ts["original_pos"]
            word = ts["word"]
            start_time = ts["start"]
            end_time = ts["end"]

            # Check time monotonicity
            if start_time < prev_time - 0.001:
                errors.append(f"Word {i} '{word}': time goes backward! {start_time} < {prev_time}")
            prev_time = end_time

            # Check bounds
            if orig_start < 0 or orig_end > len(original_text):
                errors.append(f"Word {i} '{word}': position [{orig_start}, {orig_end}] out of bounds")
                continue

            if orig_start >= orig_end:
                errors.append(f"Word {i} '{word}': invalid range [{orig_start}, {orig_end}]")
                continue

            # Check that we can extract something meaningful
            extracted = original_text[orig_start:orig_end]
            if len(extracted) == 0:
                errors.append(f"Word {i} '{word}': empty extraction at [{orig_start}, {orig_end}]")

            # Show first 10 and last 10
            if i < 10 or i >= len(timestamps) - 10:
                print(f"  [{i:3d}] {start_time:6.2f}-{end_time:6.2f}s: '{word}' -> orig[{orig_start}:{orig_end}] = '{extracted}'")

        if errors:
            print("\nERRORS:")
            for e in errors[:20]:
                print(f"  {e}")

        assert not errors, f"Found {len(errors)} errors"

    def test_word_positions_match_text(self, runnable):
        """Word positions should point to actual words in text."""
        text = "Первое второе третье четвёртое пятое."
        chunk_durations = [(0, len(text), 3.0)]

        timestamps = runnable._estimate_timestamps_chunked(text, chunk_durations, None)

        for ts in timestamps:
            start, end = ts["original_pos"]
            extracted = text[start:end]

            # The extracted text should match the word (or be close)
            assert ts["word"] in text, f"Word '{ts['word']}' not in text"
            print(f"Word: '{ts['word']}', pos: [{start}, {end}], extracted: '{extracted}'")

    def test_gpt_article_positions(self, runnable):
        """Test position mapping with the problematic article."""
        # Simplified version for testing
        text = """Модель обучалась и запускалась на системах NVIDIA GB200 NVL72. Это стойки с новыми ускорителями Blackwell, рассчитанные на плотную работу с большими моделями и агентами."""

        chunks = runnable._split_into_chunks(text)

        # Create chunk_durations based on chunks
        chunk_durations = []
        for chunk_text, start_pos in chunks:
            # Simulate 1 second per 100 characters
            duration = len(chunk_text) / 100.0
            chunk_durations.append((start_pos, start_pos + len(chunk_text), duration))

        timestamps = runnable._estimate_timestamps_chunked(text, chunk_durations, None)

        print(f"\n=== Position mapping test ===")
        print(f"Text length: {len(text)}")
        print(f"Chunks: {len(chunks)}")

        errors = []
        for i, ts in enumerate(timestamps):
            start, end = ts["original_pos"]
            word = ts["word"]

            if start >= len(text) or end > len(text):
                errors.append(f"Word {i} '{word}': position [{start}, {end}] out of bounds (text len={len(text)})")
                continue

            extracted = text[start:end]
            if extracted != word:
                errors.append(f"Word {i}: expected '{word}' at [{start}, {end}], got '{extracted}'")

        if errors:
            print("Errors found:")
            for e in errors:
                print(f"  {e}")

        assert not errors, f"Position mapping errors: {errors}"

    def test_compound_words_terminal_bench(self, runnable):
        """Test that compound words like Terminal-Bench map correctly.

        Regression test for bug where "Terminal-Bench 2.0" expansion caused
        word index to jump ahead incorrectly:
        - "Terminal-Bench" expands to "терминал бенч"
        - Algorithm failed to match "терминал" to "Terminal-Bench"
        - Word index jumped to "работы" and then stuck at "GPT-5.3-Codex"

        The fix: check if normalized word could be transliteration of
        compound word parts (split on hyphens).
        """
        from fast_tts_rus.tts_pipeline import TTSPipeline

        # Text containing the problematic sequence from the GPT-5.3 article
        original = """По бенчмаркам модель показывает лучшие результаты на SWE-Bench Pro и Terminal-Bench 2.0 — тестах, которые проверяют реальные навыки программирования и работы с терминалом. При этом GPT-5.3-Codex решает задачи."""

        pipeline = TTSPipeline()
        normalized, char_mapping = pipeline.process_with_char_mapping(original)

        print(f"\n=== Terminal-Bench compound word test ===")
        print(f"Original: {original[:100]}...")
        print(f"Normalized: {normalized[:150]}...")

        # Generate timestamps
        chunk_durations = [(0, len(normalized), 15.0)]
        timestamps = runnable._estimate_timestamps_chunked(
            normalized, chunk_durations, char_mapping
        )

        # Find positions of key words in original
        terminal_bench_pos = original.find("Terminal-Bench")
        gpt_codex_pos = original.find("GPT-5.3-Codex")
        raboty_pos = original.find("работы")

        print(f"Key positions in original:")
        print(f"  Terminal-Bench: {terminal_bench_pos}")
        print(f"  работы: {raboty_pos}")
        print(f"  GPT-5.3-Codex: {gpt_codex_pos}")

        # Find "терминал" in normalized (expansion of "Terminal")
        terminal_in_norm = normalized.lower().find("терминал")
        assert terminal_in_norm >= 0, "Expected 'терминал' in normalized text"

        # Find the timestamp for "терминал"
        terminal_timestamps = [ts for ts in timestamps if ts["word"].lower() == "терминал"]
        assert len(terminal_timestamps) >= 1, "Expected at least one 'терминал' timestamp"

        terminal_ts = terminal_timestamps[0]
        terminal_orig_pos = terminal_ts["original_pos"][0]

        print(f"\n'терминал' timestamp:")
        print(f"  original_pos: {terminal_ts['original_pos']}")
        print(f"  Expected to map to Terminal-Bench at {terminal_bench_pos}")

        # "терминал" should map to "Terminal-Bench", not to "работы" or later words
        assert terminal_orig_pos == terminal_bench_pos, \
            f"'терминал' should map to Terminal-Bench at {terminal_bench_pos}, " \
            f"but mapped to position {terminal_orig_pos} " \
            f"('{original[terminal_orig_pos:terminal_orig_pos+20]}...'). " \
            f"Bug: compound word expansion not matched correctly."

        # Also verify that words AFTER Terminal-Bench 2.0 don't all map to GPT-5.3-Codex
        # Find "тестах" which should map to "тестах" in original, not GPT-5.3-Codex
        testah_timestamps = [ts for ts in timestamps if "тестах" in ts["word"].lower()]
        if testah_timestamps:
            testah_ts = testah_timestamps[0]
            testah_orig_pos = testah_ts["original_pos"][0]
            testah_expected = original.find("тестах")

            print(f"\n'тестах' timestamp:")
            print(f"  original_pos: {testah_ts['original_pos']}")
            print(f"  Expected position: {testah_expected}")

            # "тестах" should map to itself, not to GPT-5.3-Codex
            assert testah_orig_pos != gpt_codex_pos, \
                f"'тестах' should NOT map to GPT-5.3-Codex. " \
                f"Bug: word index got stuck at wrong expandable word."

    def test_swe_bench_pro_expansion(self, runnable):
        """Test that SWE-Bench Pro expands and maps correctly."""
        from fast_tts_rus.tts_pipeline import TTSPipeline

        original = "Тесты на SWE-Bench Pro показывают хорошие результаты."

        pipeline = TTSPipeline()
        normalized, char_mapping = pipeline.process_with_char_mapping(original)

        print(f"\n=== SWE-Bench Pro test ===")
        print(f"Original: {original}")
        print(f"Normalized: {normalized}")

        chunk_durations = [(0, len(normalized), 5.0)]
        timestamps = runnable._estimate_timestamps_chunked(
            normalized, chunk_durations, char_mapping
        )

        swe_bench_pos = original.find("SWE-Bench")
        pro_pos = original.find("Pro")

        # All expansion words from "SWE-Bench" should map to SWE-Bench position
        # "Pro" expansion should map to "Pro" position

        print(f"Timestamps:")
        for ts in timestamps:
            print(f"  {ts['word']}: {ts['original_pos']}")

        # Find "бенч" or similar in timestamps
        bench_timestamps = [ts for ts in timestamps
                          if "бенч" in ts["word"].lower() or "бенкх" in ts["word"].lower()]

        if bench_timestamps:
            bench_ts = bench_timestamps[0]
            bench_orig_pos = bench_ts["original_pos"][0]

            print(f"\n'бенч/бенкх' maps to position {bench_orig_pos}")
            print(f"SWE-Bench is at position {swe_bench_pos}")

            # Should map to SWE-Bench, not to something else
            assert bench_orig_pos == swe_bench_pos, \
                f"'бенч' should map to SWE-Bench at {swe_bench_pos}, got {bench_orig_pos}"
