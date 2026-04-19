"""Abbreviations normalizer."""


class AbbreviationNormalizer:
    """Normalizes abbreviations to speakable text."""

    # Letter pronunciation map (English alphabet in Russian)
    LETTER_MAP = {
        "a": "эй",
        "b": "би",
        "c": "си",
        "d": "ди",
        "e": "и",
        "f": "эф",
        "g": "джи",
        "h": "эйч",
        "i": "ай",
        "j": "джей",
        "k": "кей",
        "l": "эл",
        "m": "эм",
        "n": "эн",
        "o": "о",
        "p": "пи",
        "q": "кью",
        "r": "ар",
        "s": "эс",
        "t": "ти",
        "u": "ю",
        "v": "ви",
        "w": "дабл ю",
        "x": "экс",
        "y": "уай",
        "z": "зед",
    }

    # Abbreviations pronounced as words
    AS_WORD = {
        # Data formats
        "json": "джейсон",
        "yaml": "ямл",
        "toml": "томл",
        # Protocols/Standards
        "rest": "рест",
        "ajax": "эйджакс",
        "crud": "крад",
        "cors": "корс",
        "oauth": "о ауз",
        # Image formats
        "gif": "гиф",
        "jpeg": "джейпег",
        # Memory
        "ram": "рам",
        "rom": "ром",
        # Network
        "lan": "лан",
        "wan": "ван",
        # Architecture
        "spa": "спа",
        "dom": "дом",
        # Other
        "gui": "гуи",
        "imap": "ай мап",
        "pop": "поп",
        # DevOps (special handling)
        "devops": "девопс",
    }

    # Special mixed-case abbreviations with custom handling
    SPECIAL_CASES = {
        "ios": "ай оу эс",
        "macos": "мак оу эс",
        "graphql": "граф кью эл",
        "iot": "ай о ти",  # Internet of Things
    }

    def normalize(self, abbrev: str) -> str:
        """Convert abbreviation to speakable text."""
        if not abbrev:
            return abbrev

        lower = abbrev.lower()

        # Check special cases first
        if lower in self.SPECIAL_CASES:
            return self.SPECIAL_CASES[lower]

        # Check if it's a word-like abbreviation
        if lower in self.AS_WORD:
            return self.AS_WORD[lower]

        # Single letter
        if len(abbrev) == 1:
            return self.LETTER_MAP.get(lower, abbrev)

        # Check if all letters - spell it out
        if abbrev.isalpha():
            return self._spell_out(abbrev)

        # Mixed alphanumeric - try to handle
        return self._handle_mixed(abbrev)

    def _spell_out(self, abbrev: str) -> str:
        """Spell out abbreviation letter by letter."""
        result = []
        for char in abbrev.lower():
            if char in self.LETTER_MAP:
                result.append(self.LETTER_MAP[char])
            else:
                result.append(char)
        return " ".join(result)

    def _handle_mixed(self, abbrev: str) -> str:
        """Handle mixed abbreviations with numbers or special chars."""
        result = []
        for char in abbrev.lower():
            if char in self.LETTER_MAP:
                result.append(self.LETTER_MAP[char])
            elif char.isdigit():
                # Keep digits as-is for now, pipeline will handle
                result.append(char)
            else:
                result.append(char)
        return " ".join(result)
