"""Main TTS text preprocessing pipeline."""

import re
from fast_tts_rus.tts_pipeline.config import PipelineConfig
from fast_tts_rus.tts_pipeline.constants import GREEK_LETTERS, MATH_SYMBOLS, ARROW_SYMBOLS
from fast_tts_rus.tts_pipeline.normalizers import (
    NumberNormalizer,
    EnglishNormalizer,
    AbbreviationNormalizer,
    SymbolNormalizer,
    URLPathNormalizer,
    CodeIdentifierNormalizer,
    CodeBlockHandler,
)
from fast_tts_rus.tts_pipeline.word_mapping import WordMapping, build_word_mapping
from fast_tts_rus.tts_pipeline.tracked_text import TrackedText, CharMapping


class TTSPipeline:
    """Main pipeline for text preprocessing before TTS."""

    def __init__(self, config: PipelineConfig | None = None):
        self.config = config or PipelineConfig()

        # Initialize normalizers
        self.number_normalizer = NumberNormalizer()
        self.english_normalizer = EnglishNormalizer()
        self.abbrev_normalizer = AbbreviationNormalizer()
        self.symbol_normalizer = SymbolNormalizer()
        self.url_normalizer = URLPathNormalizer(
            english_normalizer=self.english_normalizer,
            number_normalizer=self.number_normalizer,
        )
        self.code_normalizer = CodeIdentifierNormalizer(
            number_normalizer=self.number_normalizer,
            abbrev_normalizer=self.abbrev_normalizer,
        )
        self.code_block_handler = CodeBlockHandler(
            mode=self.config.code_block_mode,
            code_normalizer=self.code_normalizer,
            number_normalizer=self.number_normalizer,
        )

        # Add custom terms if provided
        if self.config.custom_it_terms:
            self.english_normalizer.add_custom_terms(self.config.custom_it_terms)

    def process(self, text: str) -> str:
        """Process text for TTS. Returns normalized text."""
        if not text:
            return ""

        # Clear unknown words tracking for new processing
        self.english_normalizer.clear_unknown_words()

        # Preprocessing
        text = self._preprocess(text)

        if not text.strip():
            return ""

        # Process markdown code blocks first (extract and replace) - using tracked version
        tracked = TrackedText(text)
        self._process_code_blocks_tracked(tracked)

        # Process inline code - using tracked version
        self._process_inline_code_tracked(tracked)

        # Process markdown structure - using tracked version
        self._process_markdown_tracked(tracked)

        # Process URLs - using tracked version
        self._process_urls_tracked(tracked)

        # Process emails - using tracked version
        self._process_emails_tracked(tracked)

        # Process IP addresses - using tracked version
        self._process_ips_tracked(tracked)

        # Process file paths - using tracked version
        self._process_paths_tracked(tracked)

        # Create new TrackedText to allow nested transformations (e.g., transliterate words inside paths)
        text = tracked.text
        tracked = TrackedText(text)

        # Process size units (e.g., 100MB, 50ms) - using tracked version
        self._process_sizes_tracked(tracked)

        # Process versions - using tracked version
        self._process_versions_tracked(tracked)

        # Process ranges (e.g., 10-20) - using tracked version
        self._process_ranges_tracked(tracked)

        # Process percentages - using tracked version
        self._process_percentages_tracked(tracked)

        # Process operators - using tracked version
        if self.config.read_operators:
            self._process_operators_tracked(tracked)

        # Process special symbols (Greek letters, etc.) - using tracked version
        self._process_symbols_tracked(tracked)

        # Process code identifiers (camelCase, snake_case, kebab-case) - using tracked version
        self._process_code_identifiers_tracked(tracked)

        # Process English words and IT terms (using tracked version)
        self._process_english_words_tracked(tracked)

        # Process numbers (using tracked version)
        self._process_numbers_tracked(tracked)
        text = tracked.text

        # Clean up
        text = self._postprocess(text)

        return text

    def process_with_mapping(self, text: str) -> tuple[str, WordMapping]:
        """Process text for TTS and return mapping to original.

        This method processes the text and builds a word-level mapping
        that allows mapping positions in the transformed text back to
        positions in the original text.

        Args:
            text: Original text to process

        Returns:
            Tuple of (transformed_text, WordMapping)
        """
        original = text
        transformed = self.process(text)
        mapping = build_word_mapping(original, transformed, self)
        return transformed, mapping

    def process_with_char_mapping(self, text: str) -> tuple[str, CharMapping]:
        """Process text for TTS with precise character-level mapping.

        This method tracks all transformations precisely, allowing
        exact mapping from any position in the transformed text back
        to the corresponding position in the original text.

        Args:
            text: Original text to process

        Returns:
            Tuple of (transformed_text, CharMapping)
        """
        if not text:
            return "", CharMapping(original="", transformed="", char_map=[])

        # Clear unknown words tracking for new processing
        self.english_normalizer.clear_unknown_words()

        # Create tracked text with ORIGINAL text - all changes will be tracked
        tracked = TrackedText(text)

        # Preprocessing - now tracked!
        # Remove BOM at start
        if tracked.text.startswith('\ufeff'):
            tracked.sub(r'^\ufeff', '')

        # Process code blocks FIRST — before space/dash normalization which
        # creates replacement entries inside blocks, preventing the code block
        # regex from matching (TrackedText skips matches that touch replacements)
        self._process_code_blocks_tracked(tracked)

        # Normalize quotes (tracked)
        tracked.replace('«', '"')
        tracked.replace('»', '"')
        tracked.replace('"', '"')
        tracked.replace('"', '"')
        tracked.replace(''', "'")
        tracked.replace(''', "'")

        # Normalize dashes (tracked)
        tracked.replace('—', '-')
        tracked.replace('–', '-')

        # Collapse multiple newlines (tracked)
        tracked.sub(r'\n{3,}', '\n\n')

        # Collapse multiple spaces/tabs (tracked)
        tracked.sub(r'[ \t]+', ' ')

        if not tracked.text.strip():
            return "", CharMapping(original=text, transformed="", char_map=[])

        # Process inline code
        self._process_inline_code_tracked(tracked)

        # Process markdown
        self._process_markdown_tracked(tracked)

        # Process URLs
        self._process_urls_tracked(tracked)

        # Process emails
        self._process_emails_tracked(tracked)

        # Process IP addresses
        self._process_ips_tracked(tracked)

        # Process file paths
        self._process_paths_tracked(tracked)

        # Process sizes
        self._process_sizes_tracked(tracked)

        # Process versions
        self._process_versions_tracked(tracked)

        # Process ranges
        self._process_ranges_tracked(tracked)

        # Process percentages
        self._process_percentages_tracked(tracked)

        # Process operators
        if self.config.read_operators:
            self._process_operators_tracked(tracked)

        # Process symbols
        self._process_symbols_tracked(tracked)

        # Process code identifiers
        self._process_code_identifiers_tracked(tracked)

        # Process English words
        self._process_english_words_tracked(tracked)

        # Process numbers
        self._process_numbers_tracked(tracked)

        # Postprocess using tracked text to maintain mapping
        tracked.sub(r' +', ' ')  # Multiple spaces -> single space
        tracked.sub(r' +([.,!?;:])', r'\1')  # Space before punctuation
        tracked.sub(r'\n +', '\n')  # Space after newline
        tracked.sub(r' +\n', '\n')  # Space before newline

        # Build mapping after all transformations
        mapping = tracked.build_mapping()

        # Strip leading/trailing whitespace (adjust mapping manually)
        result = mapping.transformed.strip()
        if result != mapping.transformed:
            # Rebuild char_map accounting for strip
            leading_spaces = len(mapping.transformed) - len(mapping.transformed.lstrip())
            trailing_spaces = len(mapping.transformed) - len(mapping.transformed.rstrip())
            if leading_spaces > 0 or trailing_spaces > 0:
                end_idx = len(mapping.char_map) - trailing_spaces if trailing_spaces > 0 else len(mapping.char_map)
                new_char_map = mapping.char_map[leading_spaces:end_idx]
                mapping = CharMapping(
                    original=mapping.original,
                    transformed=result,
                    char_map=new_char_map,
                )

        return result, mapping

    def set_code_mode(self, mode: str) -> None:
        """Switch code block handling mode."""
        if mode in ("full", "brief"):
            self.config.code_block_mode = mode
            self.code_block_handler.set_mode(mode)

    def get_unknown_words(self) -> dict[str, str]:
        """Get dictionary of unknown words that were transliterated via fallback.

        Returns:
            Dict mapping original word to its transliteration.
        """
        return self.english_normalizer.get_unknown_words()

    def get_warnings(self) -> list[str]:
        """Get list of warning messages about unknown words.

        Returns:
            List of warning strings.
        """
        unknown = self.get_unknown_words()
        if not unknown:
            return []

        warnings = ["Следующие слова были транслитерированы автоматически:"]
        for original, transliterated in sorted(unknown.items()):
            warnings.append(f"  {original} → {transliterated}")
        warnings.append("Добавьте их в словарь IT_TERMS для точного произношения.")
        return warnings

    def print_warnings(self) -> None:
        """Print warnings about unknown words to stderr."""
        import sys
        warnings = self.get_warnings()
        if warnings:
            for line in warnings:
                print(line, file=sys.stderr)

    def _preprocess(self, text: str) -> str:
        """Preprocess text before normalization."""
        # Remove BOM
        text = text.lstrip('\ufeff')

        # Normalize quotes
        text = text.replace('«', '"').replace('»', '"')
        text = text.replace('"', '"').replace('"', '"')
        text = text.replace(''', "'").replace(''', "'")

        # Normalize dashes (but preserve for ranges)
        # Em-dash to hyphen for ranges
        text = text.replace('—', '-')
        # En-dash to hyphen
        text = text.replace('–', '-')

        # Normalize multiple newlines
        text = re.sub(r'\n{3,}', '\n\n', text)

        # Normalize multiple spaces (but keep newlines)
        text = re.sub(r'[ \t]+', ' ', text)

        return text

    def _postprocess(self, text: str) -> str:
        """Clean up after normalization."""
        # Remove multiple spaces
        text = re.sub(r' +', ' ', text)

        # Clean up around punctuation
        text = re.sub(r' +([.,!?;:])', r'\1', text)

        # Clean up newlines
        text = re.sub(r'\n +', '\n', text)
        text = re.sub(r' +\n', '\n', text)

        return text.strip()

    # =========================================================================
    # Tracked versions of processing methods for process_with_char_mapping
    # =========================================================================

    def _process_code_blocks_tracked(self, tracked: TrackedText) -> None:
        """Process markdown code blocks with tracking."""
        pattern = r'```(\w*)\n(.*?)```'

        def replace_block(match):
            language = match.group(1) or None
            code = match.group(2)

            # Mermaid diagrams: not readable text, replace with brief marker
            if language and language.lower() == 'mermaid':
                return 'Тут мермэйд диаграмма'

            self.code_block_handler.set_mode(self.config.code_block_mode)
            return self.code_block_handler.process(code.strip(), language)

        tracked.sub(pattern, replace_block, flags=re.DOTALL)

    def _normalize_code_words(self, code: str) -> str:
        """Normalize space-separated code words using dictionaries and English fallback."""
        words = code.split()
        result = []
        for word in words:
            word_lower = word.lower()
            if word_lower in self.code_normalizer.CODE_WORDS:
                result.append(self.code_normalizer.CODE_WORDS[word_lower])
            elif word_lower in self.english_normalizer.IT_TERMS:
                result.append(self.english_normalizer.IT_TERMS[word_lower])
            elif word.isascii() and any(c.isalpha() for c in word):
                result.append(self.english_normalizer.normalize(word))
            else:
                result.append(word)
        return ' '.join(result)

    def _process_inline_code_tracked(self, tracked: TrackedText) -> None:
        """Process inline code with tracking."""
        pattern = r'`([^`\n]+)`'

        def replace_inline(match):
            code = match.group(1)

            # Pre-process Greek letters and special symbols
            has_greek_or_special = False
            for char, replacement in self.code_block_handler.GREEK_LETTERS.items():
                if char in code:
                    code = code.replace(char, f' {replacement} ')
                    has_greek_or_special = True
            for char, replacement in self.code_block_handler.SPECIAL_SYMBOLS.items():
                if char in code:
                    code = code.replace(char, f' {replacement} ')
                    has_greek_or_special = True

            code = ' '.join(code.split())

            if has_greek_or_special:
                return self._normalize_code_words(code)

            if '_' in code:
                return self.code_normalizer.normalize_snake_case(code)
            elif '-' in code and not code.startswith('-'):
                return self.code_normalizer.normalize_kebab_case(code)
            elif any(c.isupper() for c in code[1:]) and any(c.islower() for c in code):
                return self.code_normalizer.normalize_camel_case(code)
            else:
                return self._normalize_code_words(code)

        tracked.sub(pattern, replace_inline)

    def _process_markdown_tracked(self, tracked: TrackedText) -> None:
        """Process markdown structural elements with tracking."""
        # Remove heading markers
        tracked.sub(r'^#{1,6}\s+', '', flags=re.MULTILINE)

        # Process markdown links: [text](url) → text
        # Remove [ and ](url) separately so link text stays as original
        # characters and can be further normalized (e.g. English transliteration)
        tracked.sub(r'\[(?=[^\]]+\]\([^)]+\))', '')  # remove [ before link
        tracked.sub(r'\]\([^)]+\)', '')               # remove ](url)

        # Process numbered lists
        def replace_list_number(match):
            num = int(match.group(1))
            ordinals = {
                1: 'первое', 2: 'второе', 3: 'третье', 4: 'четвёртое',
                5: 'пятое', 6: 'шестое', 7: 'седьмое', 8: 'восьмое',
                9: 'девятое', 10: 'десятое',
            }
            ordinal = ordinals.get(num, self.number_normalizer.normalize_number(str(num)))
            return ordinal + ':'

        tracked.sub(r'^(\d+)\.\s+', replace_list_number, flags=re.MULTILINE)

    def _process_urls_tracked(self, tracked: TrackedText) -> None:
        """Process URLs with tracking."""
        def replace_url(match):
            return self.url_normalizer.normalize_url(match.group(0))

        tracked.sub(self._RE_URL, replace_url)

    def _process_emails_tracked(self, tracked: TrackedText) -> None:
        """Process emails with tracking."""
        def replace_email(match):
            return self.url_normalizer.normalize_email(match.group(0))

        tracked.sub(self._RE_EMAIL, replace_email)

    def _process_ips_tracked(self, tracked: TrackedText) -> None:
        """Process IP addresses with tracking."""
        def replace_ip(match):
            return self.url_normalizer.normalize_ip(match.group(0))

        tracked.sub(self._RE_IP, replace_ip)

    def _process_paths_tracked(self, tracked: TrackedText) -> None:
        """Process file paths with tracking."""
        def replace_path(match):
            path = match.group(1)
            if '/' in path and (path.startswith('/') or path.startswith('~')):
                return self.url_normalizer.normalize_filepath(path)
            return path

        tracked.sub(self._RE_PATH, replace_path)

    def _process_sizes_tracked(self, tracked: TrackedText) -> None:
        """Process size units with tracking."""
        def replace_size(match):
            return self.number_normalizer.normalize_size(match.group(0))

        tracked.sub(self._RE_SIZE, replace_size)

    def _process_versions_tracked(self, tracked: TrackedText) -> None:
        """Process version numbers with tracking."""
        def replace_version(match):
            version = match.group(0)
            if '.' in version:
                return self.number_normalizer.normalize_version(version)
            return version

        tracked.sub(self._RE_VERSION, replace_version)

    def _process_ranges_tracked(self, tracked: TrackedText) -> None:
        """Process number ranges with tracking."""
        def replace_range(match):
            return self.number_normalizer.normalize_range(match.group(0))

        tracked.sub(self._RE_RANGE, replace_range)

    def _process_percentages_tracked(self, tracked: TrackedText) -> None:
        """Process percentages with tracking."""
        def replace_pct(match):
            return self.number_normalizer.normalize_percentage(match.group(0))

        tracked.sub(self._RE_PERCENTAGE, replace_pct)

    # Multi-char operators to replace in tracked mode (longest first for correct matching)
    _TRACKED_OPERATOR_KEYS = ['===', '!==', '->', '=>', '>=', '<=', '!=', '==', '&&', '||']

    _RE_URL = re.compile(r'https?://[^\s<>"\'\)]+|ftp://[^\s<>"\'\)]+|ssh://[^\s<>"\'\)]+|git://[^\s<>"\'\)]+')
    _RE_EMAIL = re.compile(r'[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}')
    _RE_IP = re.compile(r'\b(\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3})\b')
    _RE_PATH = re.compile(r'(?<![a-zA-Z0-9])([~/][a-zA-Z0-9_./\-]+\.[a-zA-Z0-9]+|[~/][a-zA-Z0-9_./\-]+)')
    _RE_SIZE = re.compile(r'\b(\d+(?:\.\d+)?)\s*(KB|MB|GB|TB|ms|sec|min|hr|px|em|rem|vh|vw|кб|мб|гб|тб)\b', re.IGNORECASE)
    _RE_VERSION = re.compile(r'\bv?(\d+\.\d+(?:\.\d+)?(?:-(?:alpha|beta|rc|dev|stable|release)\d*)?)\b', re.IGNORECASE)
    _RE_RANGE = re.compile(r'\b(\d+)\s*-\s*(\d+)\b')
    _RE_PERCENTAGE = re.compile(r'\b(\d+(?:\.\d+)?)\s*%')

    def _process_operators_tracked(self, tracked: TrackedText) -> None:
        """Process operators with tracking."""
        symbols = SymbolNormalizer.SYMBOLS
        for op in self._TRACKED_OPERATOR_KEYS:
            tracked.replace(op, f' {symbols[op]} ')

    def _process_symbols_tracked(self, tracked: TrackedText) -> None:
        """Process special symbols with tracking."""
        for symbol, replacement in (*GREEK_LETTERS.items(), *MATH_SYMBOLS.items(), *ARROW_SYMBOLS.items()):
            tracked.replace(symbol, f' {replacement} ')

    def _process_code_identifiers_tracked(self, tracked: TrackedText) -> None:
        """Process code identifiers with tracking."""
        # CamelCase
        tracked.sub(
            r'\b([a-z]+(?:[A-Z][a-z]*)+)\b',
            lambda m: self.code_normalizer.normalize_camel_case(m.group(0))
        )

        # PascalCase
        tracked.sub(
            r'\b([A-Z][a-z]+(?:[A-Z][a-z]+)+)\b',
            lambda m: self.code_normalizer.normalize_camel_case(m.group(0))
        )

        # Snake_case
        tracked.sub(
            r'\b([a-zA-Z_][a-zA-Z0-9]*(?:_[a-zA-Z0-9]+)+)\b',
            lambda m: self.code_normalizer.normalize_snake_case(m.group(0))
        )

        # Kebab-case
        tracked.sub(
            r'\b([a-zA-Z][a-zA-Z0-9]*(?:-[a-zA-Z0-9]+)+)\b',
            lambda m: self.code_normalizer.normalize_kebab_case(m.group(0))
        )

    def _process_english_words_tracked(self, tracked: TrackedText) -> None:
        """Process English words with tracking."""
        # Special terms first
        special_terms = {
            'C++': 'си плюс плюс', 'c++': 'си плюс плюс',
            'C#': 'си шарп', 'c#': 'си шарп',
            'F#': 'эф шарп', 'f#': 'эф шарп',
        }
        for term, replacement in special_terms.items():
            tracked.replace(term, replacement)

        # English words pattern
        def replace_english(match):
            word = match.group(0)
            word_lower = word.lower()

            if word_lower in self.english_normalizer.IT_TERMS:
                return self.english_normalizer.IT_TERMS[word_lower]

            if word_lower in self.english_normalizer.custom_terms:
                return self.english_normalizer.custom_terms[word_lower]

            if word.isupper() and len(word) >= 2:
                return self.abbrev_normalizer.normalize(word)

            if word_lower in self.abbrev_normalizer.AS_WORD:
                return self.abbrev_normalizer.AS_WORD[word_lower]

            return self.english_normalizer.normalize(word, track_unknown=True)

        tracked.sub(r'\b([A-Za-z][A-Za-z]+)\b', replace_english)

    def _process_numbers_tracked(self, tracked: TrackedText) -> None:
        """Process numbers with tracking."""
        number_pattern = r'(?<![.\d])(\d+)(?![.\d]|[a-zA-Zа-яА-Я])'

        def replace_number(match):
            return self.number_normalizer.normalize_number(match.group(0))

        tracked.sub(number_pattern, replace_number)
