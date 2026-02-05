"""Main TTS text preprocessing pipeline."""

import re
from fast_tts_rus.tts_pipeline.config import PipelineConfig
from fast_tts_rus.tts_pipeline.normalizers import (
    NumberNormalizer,
    EnglishNormalizer,
    AbbreviationNormalizer,
    SymbolNormalizer,
    URLPathNormalizer,
    CodeIdentifierNormalizer,
    CodeBlockHandler,
)


class TTSPipeline:
    """Main pipeline for text preprocessing before TTS."""

    def __init__(self, config: PipelineConfig | None = None):
        self.config = config or PipelineConfig()

        # Initialize normalizers
        self.number_normalizer = NumberNormalizer()
        self.english_normalizer = EnglishNormalizer()
        self.abbrev_normalizer = AbbreviationNormalizer()
        self.symbol_normalizer = SymbolNormalizer()
        self.url_normalizer = URLPathNormalizer()
        self.code_normalizer = CodeIdentifierNormalizer()
        self.code_block_handler = CodeBlockHandler(mode=self.config.code_block_mode)

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

        # Process markdown code blocks first (extract and replace)
        text = self._process_code_blocks(text)

        # Process inline code
        text = self._process_inline_code(text)

        # Process markdown structure
        text = self._process_markdown(text)

        # Process URLs
        text = self._process_urls(text)

        # Process emails
        text = self._process_emails(text)

        # Process IP addresses
        text = self._process_ips(text)

        # Process file paths
        text = self._process_paths(text)

        # Process size units (e.g., 100MB, 50ms)
        text = self._process_sizes(text)

        # Process versions
        text = self._process_versions(text)

        # Process ranges (e.g., 10-20)
        text = self._process_ranges(text)

        # Process percentages
        text = self._process_percentages(text)

        # Process operators
        if self.config.read_operators:
            text = self._process_operators(text)

        # Process special symbols (Greek letters, etc.)
        text = self._process_symbols(text)

        # Process code identifiers (camelCase, snake_case, kebab-case)
        text = self._process_code_identifiers(text)

        # Process English words and IT terms
        text = self._process_english_words(text)

        # Process numbers
        text = self._process_numbers(text)

        # Clean up
        text = self._postprocess(text)

        return text

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

    def _process_code_blocks(self, text: str) -> str:
        """Process markdown code blocks."""
        # Pattern for fenced code blocks: ```language\ncode\n```
        pattern = r'```(\w*)\n(.*?)```'

        def replace_block(match):
            language = match.group(1) or None
            code = match.group(2)
            self.code_block_handler.set_mode(self.config.code_block_mode)
            return self.code_block_handler.process(code.strip(), language)

        text = re.sub(pattern, replace_block, text, flags=re.DOTALL)
        return text

    def _process_inline_code(self, text: str) -> str:
        """Process inline code in backticks."""
        # Pattern for inline code: `code`
        pattern = r'`([^`]+)`'

        def replace_inline(match):
            code = match.group(1)
            original_code = code

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

            # Clean up multiple spaces
            code = ' '.join(code.split())

            # If we had Greek letters or special symbols, process words individually
            if has_greek_or_special:
                words = code.split()
                result = []
                for word in words:
                    word_lower = word.lower()
                    if word_lower in self.code_normalizer.CODE_WORDS:
                        result.append(self.code_normalizer.CODE_WORDS[word_lower])
                    elif word_lower in self.english_normalizer.IT_TERMS:
                        result.append(self.english_normalizer.IT_TERMS[word_lower])
                    else:
                        # Keep already converted words (альфа, стрелка, etc.)
                        result.append(word)
                return ' '.join(result)

            # Process the inline code through code normalizer
            # Detect pattern
            if '_' in code:
                return self.code_normalizer.normalize_snake_case(code)
            elif '-' in code and not code.startswith('-'):
                return self.code_normalizer.normalize_kebab_case(code)
            elif any(c.isupper() for c in code[1:]) and any(c.islower() for c in code):
                return self.code_normalizer.normalize_camel_case(code)
            else:
                # Process words individually
                words = code.split()
                result = []
                for word in words:
                    word_lower = word.lower()
                    if word_lower in self.code_normalizer.CODE_WORDS:
                        result.append(self.code_normalizer.CODE_WORDS[word_lower])
                    elif word_lower in self.english_normalizer.IT_TERMS:
                        result.append(self.english_normalizer.IT_TERMS[word_lower])
                    else:
                        result.append(word)
                return ' '.join(result)

        text = re.sub(pattern, replace_inline, text)
        return text

    def _process_markdown(self, text: str) -> str:
        """Process markdown structural elements."""
        # Remove heading markers but keep text
        text = re.sub(r'^#{1,6}\s+', '', text, flags=re.MULTILINE)

        # Process links: [text](url)
        def replace_link(match):
            link_text = match.group(1)
            url = match.group(2)
            # Include both text and expanded URL
            expanded_url = self.url_normalizer.normalize_url(url)
            return f"{link_text} {expanded_url}"

        text = re.sub(r'\[([^\]]+)\]\(([^)]+)\)', replace_link, text)

        # Process numbered lists: convert numbers to ordinals
        def replace_list_number(match):
            num = int(match.group(1))
            # Use Russian ordinal approximation
            ordinals = {
                1: 'первое',
                2: 'второе',
                3: 'третье',
                4: 'четвёртое',
                5: 'пятое',
                6: 'шестое',
                7: 'седьмое',
                8: 'восьмое',
                9: 'девятое',
                10: 'десятое',
            }
            ordinal = ordinals.get(num, self.number_normalizer.normalize_number(str(num)))
            return ordinal + ':'

        text = re.sub(r'^(\d+)\.\s+', replace_list_number, text, flags=re.MULTILINE)

        return text

    def _process_urls(self, text: str) -> str:
        """Process URLs in text."""
        # URL pattern
        url_pattern = r'https?://[^\s<>"\'\)]+|ftp://[^\s<>"\'\)]+|ssh://[^\s<>"\'\)]+|git://[^\s<>"\'\)]+'

        def replace_url(match):
            url = match.group(0)
            return self.url_normalizer.normalize_url(url)

        text = re.sub(url_pattern, replace_url, text)
        return text

    def _process_emails(self, text: str) -> str:
        """Process email addresses in text."""
        # Email pattern
        email_pattern = r'[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}'

        def replace_email(match):
            email = match.group(0)
            return self.url_normalizer.normalize_email(email)

        text = re.sub(email_pattern, replace_email, text)
        return text

    def _process_ips(self, text: str) -> str:
        """Process IP addresses in text."""
        # IP pattern (basic IPv4)
        ip_pattern = r'\b(\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3})\b'

        def replace_ip(match):
            ip = match.group(0)
            return self.url_normalizer.normalize_ip(ip)

        text = re.sub(ip_pattern, replace_ip, text)
        return text

    def _process_paths(self, text: str) -> str:
        """Process file paths in text."""
        # Unix path pattern (starting with / or ~)
        unix_path_pattern = r'(?<![a-zA-Z0-9])([~/][a-zA-Z0-9_./\-]+\.[a-zA-Z0-9]+|[~/][a-zA-Z0-9_./\-]+)'

        def replace_path(match):
            path = match.group(1)
            # Only process if it looks like a real path
            if '/' in path and (path.startswith('/') or path.startswith('~')):
                return self.url_normalizer.normalize_filepath(path)
            return path

        text = re.sub(unix_path_pattern, replace_path, text)
        return text

    def _process_sizes(self, text: str) -> str:
        """Process size units (e.g., 100MB, 50ms)."""
        # Size pattern
        size_pattern = r'\b(\d+(?:\.\d+)?)\s*(KB|MB|GB|TB|ms|sec|min|hr|px|em|rem|vh|vw|кб|мб|гб|тб)\b'

        def replace_size(match):
            size_str = match.group(0)
            return self.number_normalizer.normalize_size(size_str)

        text = re.sub(size_pattern, replace_size, text, flags=re.IGNORECASE)
        return text

    def _process_versions(self, text: str) -> str:
        """Process version numbers."""
        # Version pattern: v1.2.3 or just 1.2.3 in version context
        version_pattern = r'\bv?(\d+\.\d+(?:\.\d+)?(?:-[a-zA-Z]+\d*)?)\b'

        def replace_version(match):
            version = match.group(0)
            # Only process if it has dots (to avoid matching plain numbers)
            if '.' in version:
                return self.number_normalizer.normalize_version(version)
            return version

        text = re.sub(version_pattern, replace_version, text)
        return text

    def _process_ranges(self, text: str) -> str:
        """Process number ranges."""
        # Range pattern: 10-20, 100-200
        range_pattern = r'\b(\d+)\s*-\s*(\d+)\b'

        def replace_range(match):
            full_match = match.group(0)
            # Check if it looks like a range (not a negative number)
            start = match.group(1)
            end = match.group(2)
            return self.number_normalizer.normalize_range(full_match)

        text = re.sub(range_pattern, replace_range, text)
        return text

    def _process_percentages(self, text: str) -> str:
        """Process percentages."""
        # Percentage pattern
        pct_pattern = r'\b(\d+(?:\.\d+)?)\s*%'

        def replace_pct(match):
            pct = match.group(0)
            return self.number_normalizer.normalize_percentage(pct)

        text = re.sub(pct_pattern, replace_pct, text)
        return text

    def _process_operators(self, text: str) -> str:
        """Process operators to speakable text."""
        # Multi-char operators first (order matters)
        operators = [
            ('===', 'строго равно'),
            ('!==', 'строго не равно'),
            ('->', 'стрелка'),
            ('=>', 'толстая стрелка'),
            ('>=', 'больше или равно'),
            ('<=', 'меньше или равно'),
            ('!=', 'не равно'),
            ('==', 'равно равно'),
            ('&&', 'и'),
            ('||', 'или'),
        ]

        for op, replacement in operators:
            text = text.replace(op, f' {replacement} ')

        return text

    def _process_symbols(self, text: str) -> str:
        """Process special symbols (Greek letters, etc.) to speakable text."""
        # Only process symbols that are standalone (not letters, digits, or common punct)
        # Focus on Greek letters and special Unicode symbols
        greek_and_special = {
            # Greek letters (lowercase)
            'α': 'альфа', 'β': 'бета', 'γ': 'гамма', 'δ': 'дельта',
            'ε': 'эпсилон', 'ζ': 'дзета', 'η': 'эта', 'θ': 'тета',
            'ι': 'йота', 'κ': 'каппа', 'λ': 'лямбда', 'μ': 'мю',
            'ν': 'ню', 'ξ': 'кси', 'π': 'пи', 'ρ': 'ро',
            'σ': 'сигма', 'τ': 'тау', 'υ': 'ипсилон', 'φ': 'фи',
            'χ': 'хи', 'ψ': 'пси', 'ω': 'омега',
            # Greek letters (uppercase)
            'Α': 'альфа', 'Β': 'бета', 'Γ': 'гамма', 'Δ': 'дельта',
            'Ε': 'эпсилон', 'Ζ': 'дзета', 'Η': 'эта', 'Θ': 'тета',
            'Ι': 'йота', 'Κ': 'каппа', 'Λ': 'лямбда', 'Μ': 'мю',
            'Ν': 'ню', 'Ξ': 'кси', 'Π': 'пи', 'Ρ': 'ро',
            'Σ': 'сигма', 'Τ': 'тау', 'Υ': 'ипсилон', 'Φ': 'фи',
            'Χ': 'хи', 'Ψ': 'пси', 'Ω': 'омега',
            # Math symbols
            '∞': 'бесконечность', '∈': 'принадлежит', '∉': 'не принадлежит',
            '∀': 'для всех', '∃': 'существует', '∅': 'пустое множество',
            '∩': 'пересечение', '∪': 'объединение', '⊂': 'подмножество',
            '≠': 'не равно', '≈': 'приблизительно равно', '≤': 'меньше или равно',
            '≥': 'больше или равно', '×': 'умножить', '÷': 'разделить',
            '√': 'корень', '∑': 'сумма', '∏': 'произведение',
            # Arrows
            '→': 'стрелка', '←': 'стрелка влево', '↔': 'двунаправленная стрелка',
            '⇒': 'следует', '⇐': 'следует из', '⇔': 'эквивалентно',
        }

        for symbol, replacement in greek_and_special.items():
            if symbol in text:
                text = text.replace(symbol, f' {replacement} ')

        return text

    def _process_code_identifiers(self, text: str) -> str:
        """Process code identifiers (camelCase, snake_case, kebab-case)."""
        # CamelCase pattern (at least 2 words)
        camel_pattern = r'\b([a-z]+(?:[A-Z][a-z]*)+)\b'

        def replace_camel(match):
            identifier = match.group(0)
            return self.code_normalizer.normalize_camel_case(identifier)

        text = re.sub(camel_pattern, replace_camel, text)

        # PascalCase pattern
        pascal_pattern = r'\b([A-Z][a-z]+(?:[A-Z][a-z]+)+)\b'

        def replace_pascal(match):
            identifier = match.group(0)
            return self.code_normalizer.normalize_camel_case(identifier)

        text = re.sub(pascal_pattern, replace_pascal, text)

        # Snake_case pattern (with underscores)
        snake_pattern = r'\b([a-zA-Z_][a-zA-Z0-9]*(?:_[a-zA-Z0-9]+)+)\b'

        def replace_snake(match):
            identifier = match.group(0)
            return self.code_normalizer.normalize_snake_case(identifier)

        text = re.sub(snake_pattern, replace_snake, text)

        # Kebab-case pattern
        kebab_pattern = r'\b([a-zA-Z][a-zA-Z0-9]*(?:-[a-zA-Z0-9]+)+)\b'

        def replace_kebab(match):
            identifier = match.group(0)
            return self.code_normalizer.normalize_kebab_case(identifier)

        text = re.sub(kebab_pattern, replace_kebab, text)

        return text

    def _process_english_words(self, text: str) -> str:
        """Process English words and IT terms."""
        # First handle special cases like C++, C#, F#
        # These need special handling because + and # are not word characters
        special_terms = {
            'C++': 'си плюс плюс',
            'c++': 'си плюс плюс',
            'C#': 'си шарп',
            'c#': 'си шарп',
            'F#': 'эф шарп',
            'f#': 'эф шарп',
        }
        for term, replacement in special_terms.items():
            text = text.replace(term, replacement)

        # Pattern for regular English words
        english_word_pattern = r'\b([A-Za-z][A-Za-z]+)\b'

        def replace_english(match):
            word = match.group(0)
            word_lower = word.lower()

            # Check if it's a known IT term
            if word_lower in self.english_normalizer.IT_TERMS:
                return self.english_normalizer.IT_TERMS[word_lower]

            # Check custom terms
            if word_lower in self.english_normalizer.custom_terms:
                return self.english_normalizer.custom_terms[word_lower]

            # Check if it's an abbreviation (all caps)
            if word.isupper() and len(word) >= 2:
                return self.abbrev_normalizer.normalize(word)

            # Check abbreviations dictionary
            if word_lower in self.abbrev_normalizer.AS_WORD:
                return self.abbrev_normalizer.AS_WORD[word_lower]

            # Fallback: transliterate unknown word and track it
            return self.english_normalizer.normalize(word, track_unknown=True)

        text = re.sub(english_word_pattern, replace_english, text)
        return text

    def _process_numbers(self, text: str) -> str:
        """Process standalone numbers."""
        # Number pattern (not part of version, IP, etc.)
        # Match numbers not followed by units or dots
        number_pattern = r'(?<![.\d])(\d+)(?![.\d]|[a-zA-Zа-яА-Я])'

        def replace_number(match):
            num = match.group(0)
            return self.number_normalizer.normalize_number(num)

        text = re.sub(number_pattern, replace_number, text)
        return text
