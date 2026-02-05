"""Debug word mapping for specific texts."""
import sys
sys.path.insert(0, "src")

from fast_tts_rus.tts_pipeline import TTSPipeline, tokenize_words


def test_text(original: str):
    """Test mapping for a text using precise char-level tracking."""
    print(f"\n{'='*60}")
    print(f"Original: {original!r}")
    print(f"{'='*60}")

    pipeline = TTSPipeline()

    # Use new precise char-level mapping
    transformed, char_mapping = pipeline.process_with_char_mapping(original)

    print(f"\nTransformed: {transformed!r}")
    print(f"\nChar mapping length: {len(char_mapping.char_map)}")

    # Tokenize transformed to show word-level results
    trans_words = tokenize_words(transformed)
    print(f"\nTransformed words -> Original ranges:")
    for tw in trans_words:
        # Get original range for this word
        orig_range = char_mapping.get_original_range(tw.start, tw.end)
        orig_text = original[orig_range[0]:orig_range[1]]
        print(f"  '{tw.text}' [{tw.start}:{tw.end}] -> orig[{orig_range[0]}:{orig_range[1]}] '{orig_text}'")


if __name__ == "__main__":
    # Test 1: Russian text with technical terms
    test_text("Сейчас приоритетнее разобраться с ошибкой, почему в контекстном меню 3 команды читать сразу и читать отложено. Выдают аж ошибку, что оффер пустой.")

    # Test 2: English text with numbers
    test_text("Test clipboard content 12345")
