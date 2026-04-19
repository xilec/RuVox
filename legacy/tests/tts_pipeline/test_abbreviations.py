"""Tests for abbreviations normalizer.

Coverage: IT abbreviations read as words and spelled out.
"""

import pytest


class TestAbbreviationsAsWords:
    """Tests for abbreviations pronounced as words."""

    @pytest.mark.parametrize(
        "abbrev,expected",
        [
            # Data formats
            ("JSON", "джейсон"),
            ("YAML", "ямл"),
            ("TOML", "томл"),
            # Protocols/Standards
            ("REST", "рест"),
            ("AJAX", "эйджакс"),
            ("CRUD", "крад"),
            ("CORS", "корс"),
            ("OAuth", "о ауз"),
            # Image formats
            ("GIF", "гиф"),
            ("JPEG", "джейпег"),
            ("PNG", "пи эн джи"),
            # Memory
            ("RAM", "рам"),
            ("ROM", "ром"),
            # Network
            ("LAN", "лан"),
            ("WAN", "ван"),
            # Architecture
            ("SPA", "спа"),
            ("DOM", "дом"),
            # Other
            ("GUI", "гуи"),
            ("IMAP", "ай мап"),
            ("POP", "поп"),
        ],
    )
    def test_abbreviation_as_word(self, abbrev_normalizer, abbrev, expected):
        """Abbreviations that can be pronounced as words."""
        result = abbrev_normalizer.normalize(abbrev)
        assert result == expected


class TestAbbreviationsSpelledOut:
    """Tests for abbreviations spelled out letter by letter."""

    @pytest.mark.parametrize(
        "abbrev,expected",
        [
            # Web
            ("HTTP", "эйч ти ти пи"),
            ("HTTPS", "эйч ти ти пи эс"),
            ("HTML", "эйч ти эм эл"),
            ("CSS", "си эс эс"),
            ("XML", "экс эм эл"),
            ("URL", "ю ар эл"),
            ("URI", "ю ар ай"),
            # API/SDK
            ("API", "эй пи ай"),
            ("SDK", "эс ди кей"),
            ("CLI", "си эл ай"),
            ("IDE", "ай ди и"),
            # Security
            ("SSL", "эс эс эл"),
            ("TLS", "ти эл эс"),
            ("SSH", "эс эс эйч"),
            ("VPN", "ви пи эн"),
            ("JWT", "джей дабл ю ти"),
            ("XSS", "экс эс эс"),
            ("CSRF", "си эс ар эф"),
            # Network
            ("TCP", "ти си пи"),
            ("UDP", "ю ди пи"),
            ("FTP", "эф ти пи"),
            ("DNS", "ди эн эс"),
            ("SMTP", "эс эм ти пи"),
            ("IP", "ай пи"),
            # Hardware
            ("CPU", "си пи ю"),
            ("GPU", "джи пи ю"),
            ("SSD", "эс эс ди"),
            ("HDD", "эйч ди ди"),
            ("USB", "ю эс би"),
            ("HDMI", "эйч ди эм ай"),
            # UI/UX
            ("UI", "ю ай"),
            ("UX", "ю экс"),
            # DevOps/CI
            ("CI", "си ай"),
            ("CD", "си ди"),
            # AI/ML
            ("AI", "эй ай"),
            ("ML", "эм эл"),
            ("NLP", "эн эл пи"),
            ("CV", "си ви"),
            # Other
            ("SQL", "эс кью эл"),
            ("ORM", "о ар эм"),
            ("MVC", "эм ви си"),
            ("MVP", "эм ви пи"),
            ("IoT", "ай о ти"),
            ("SSR", "эс эс ар"),
            ("SSG", "эс эс джи"),
            ("CSR", "си эс ар"),
            ("PWA", "пи дабл ю эй"),
            ("SVG", "эс ви джи"),
        ],
    )
    def test_abbreviation_spelled_out(self, abbrev_normalizer, abbrev, expected):
        """Abbreviations that are spelled out letter by letter."""
        result = abbrev_normalizer.normalize(abbrev)
        assert result == expected


class TestLetterMap:
    """Tests for individual letter pronunciation."""

    @pytest.mark.parametrize(
        "letter,expected",
        [
            ("A", "эй"),
            ("B", "би"),
            ("C", "си"),
            ("D", "ди"),
            ("E", "и"),
            ("F", "эф"),
            ("G", "джи"),
            ("H", "эйч"),
            ("I", "ай"),
            ("J", "джей"),
            ("K", "кей"),
            ("L", "эл"),
            ("M", "эм"),
            ("N", "эн"),
            ("O", "о"),
            ("P", "пи"),
            ("Q", "кью"),
            ("R", "ар"),
            ("S", "эс"),
            ("T", "ти"),
            ("U", "ю"),
            ("V", "ви"),
            ("W", "дабл ю"),
            ("X", "экс"),
            ("Y", "уай"),
            ("Z", "зед"),
        ],
    )
    def test_letter_pronunciation(self, abbrev_normalizer, letter, expected):
        """Individual letters should have correct pronunciation."""
        result = abbrev_normalizer.normalize(letter)
        assert result == expected


class TestCaseInsensitivity:
    """Tests for case handling."""

    @pytest.mark.parametrize(
        "abbrev,expected",
        [
            ("json", "джейсон"),
            ("Json", "джейсон"),
            ("JSON", "джейсон"),
            ("api", "эй пи ай"),
            ("Api", "эй пи ай"),
            ("API", "эй пи ай"),
        ],
    )
    def test_case_insensitive_abbreviations(self, abbrev_normalizer, abbrev, expected):
        """Abbreviations should be normalized regardless of case."""
        result = abbrev_normalizer.normalize(abbrev)
        assert result == expected


class TestUnknownAbbreviations:
    """Tests for unknown abbreviations - fallback to spelling."""

    @pytest.mark.parametrize(
        "abbrev,expected",
        [
            ("XYZ", "экс уай зед"),
            ("ABC", "эй би си"),
            ("QRS", "кью ар эс"),
            ("WXYZ", "дабл ю экс уай зед"),
        ],
    )
    def test_unknown_abbreviation_spelled_out(self, abbrev_normalizer, abbrev, expected):
        """Unknown abbreviations should be spelled out."""
        result = abbrev_normalizer.normalize(abbrev)
        assert result == expected


class TestMixedCaseAbbreviations:
    """Tests for abbreviations with mixed case (like iOS, macOS)."""

    @pytest.mark.parametrize(
        "abbrev,expected",
        [
            ("iOS", "ай оу эс"),
            ("macOS", "мак оу эс"),
            ("DevOps", "девопс"),
            ("GraphQL", "граф кью эл"),
        ],
    )
    def test_mixed_case_abbreviations(self, abbrev_normalizer, abbrev, expected):
        """Mixed-case abbreviations should be handled correctly."""
        result = abbrev_normalizer.normalize(abbrev)
        assert result == expected
