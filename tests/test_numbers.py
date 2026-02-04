"""Tests for numbers normalizer.

Coverage: integers, floats, percentages, ranges, sizes, versions, dates, times.
"""

import pytest


class TestIntegers:
    """Tests for integer number normalization."""

    @pytest.mark.parametrize(
        "number,expected",
        [
            ("0", "ноль"),
            ("1", "один"),
            ("5", "пять"),
            ("10", "десять"),
            ("11", "одиннадцать"),
            ("15", "пятнадцать"),
            ("20", "двадцать"),
            ("21", "двадцать один"),
            ("42", "сорок два"),
            ("99", "девяносто девять"),
            ("100", "сто"),
            ("101", "сто один"),
            ("123", "сто двадцать три"),
            ("200", "двести"),
            ("300", "триста"),
            ("500", "пятьсот"),
            ("999", "девятьсот девяносто девять"),
            ("1000", "одна тысяча"),
            ("1001", "одна тысяча один"),
            ("1234", "одна тысяча двести тридцать четыре"),
            ("10000", "десять тысяч"),
            ("100000", "сто тысяч"),
            ("1000000", "один миллион"),
        ],
    )
    def test_integer_to_words(self, number_normalizer, number, expected):
        """Integers should convert to Russian words correctly."""
        result = number_normalizer.normalize_number(number)
        assert result == expected


class TestFloats:
    """Tests for floating point number normalization."""

    @pytest.mark.parametrize(
        "float_str,expected",
        [
            ("3.14", "три точка один четыре"),
            ("0.5", "ноль точка пять"),
            ("2.0", "два точка ноль"),
            ("10.25", "десять точка два пять"),
            ("99.99", "девяносто девять точка девять девять"),
            ("0.001", "ноль точка ноль ноль один"),
            ("1.5", "один точка пять"),
        ],
    )
    def test_float_to_words(self, number_normalizer, float_str, expected):
        """Floats should convert with 'точка' separator."""
        result = number_normalizer.normalize_float(float_str)
        assert result == expected

    @pytest.mark.parametrize(
        "float_str,expected",
        [
            ("3,14", "три точка один четыре"),
            ("0,5", "ноль точка пять"),
            ("10,25", "десять точка два пять"),
        ],
    )
    def test_float_with_comma(self, number_normalizer, float_str, expected):
        """Floats with comma should be handled like dot."""
        result = number_normalizer.normalize_float(float_str)
        assert result == expected


class TestPercentages:
    """Tests for percentage normalization."""

    @pytest.mark.parametrize(
        "pct,expected",
        [
            ("50%", "пятьдесят процентов"),
            ("100%", "сто процентов"),
            ("1%", "один процент"),
            ("2%", "два процента"),
            ("5%", "пять процентов"),
            ("21%", "двадцать один процент"),
            ("22%", "двадцать два процента"),
            ("25%", "двадцать пять процентов"),
            ("99.9%", "девяносто девять точка девять процентов"),
            ("0.5%", "ноль точка пять процентов"),
            ("33.33%", "тридцать три точка три три процентов"),
        ],
    )
    def test_percentage_to_words(self, number_normalizer, pct, expected):
        """Percentages should have proper declension."""
        result = number_normalizer.normalize_percentage(pct)
        assert result == expected


class TestRanges:
    """Tests for number range normalization."""

    @pytest.mark.parametrize(
        "range_str,expected",
        [
            ("1-10", "от одного до десяти"),
            ("10-20", "от десяти до двадцати"),
            ("100-200", "от ста до двухсот"),
            ("2020-2024", "от две тысячи двадцатого до две тысячи двадцать четвёртого"),
            ("5-6", "от пяти до шести"),
            ("1-100", "от одного до ста"),
        ],
    )
    def test_range_to_words(self, number_normalizer, range_str, expected):
        """Ranges should use 'от ... до ...' format."""
        result = number_normalizer.normalize_range(range_str)
        assert result == expected

    @pytest.mark.parametrize(
        "range_str,expected",
        [
            ("10–20", "от десяти до двадцати"),  # en-dash
            ("100—200", "от ста до двухсот"),    # em-dash (may need separate handling)
        ],
    )
    def test_range_with_dashes(self, number_normalizer, range_str, expected):
        """Ranges with different dash types should be handled."""
        result = number_normalizer.normalize_range(range_str)
        assert result == expected


class TestSizeUnits:
    """Tests for size with units normalization."""

    @pytest.mark.parametrize(
        "size,expected",
        [
            # Bytes
            ("100KB", "сто килобайт"),
            ("1MB", "один мегабайт"),
            ("2MB", "два мегабайта"),
            ("5MB", "пять мегабайт"),
            ("16GB", "шестнадцать гигабайт"),
            ("1TB", "один терабайт"),
            ("512GB", "пятьсот двенадцать гигабайт"),
            # With space
            ("100 KB", "сто килобайт"),
            ("16 GB", "шестнадцать гигабайт"),
            # Time units
            ("10ms", "десять миллисекунд"),
            ("1sec", "одна секунда"),
            ("5sec", "пять секунд"),
            ("30min", "тридцать минут"),
            ("2hr", "два часа"),
            # CSS units
            ("16px", "шестнадцать пикселей"),
            ("1.5em", "один точка пять эм"),
            ("100vh", "сто ви эйч"),
        ],
    )
    def test_size_with_units(self, number_normalizer, size, expected):
        """Sizes with units should have proper words and declension."""
        result = number_normalizer.normalize_size(size)
        assert result == expected

    @pytest.mark.parametrize(
        "size,expected",
        [
            # Russian abbreviations
            ("100кб", "сто килобайт"),
            ("1мб", "один мегабайт"),
            ("16гб", "шестнадцать гигабайт"),
        ],
    )
    def test_russian_size_units(self, number_normalizer, size, expected):
        """Russian size abbreviations should work too."""
        result = number_normalizer.normalize_size(size)
        assert result == expected


class TestVersions:
    """Tests for software version normalization."""

    @pytest.mark.parametrize(
        "version,expected",
        [
            ("1.0", "один точка ноль"),
            ("2.0", "два точка ноль"),
            ("1.0.0", "один точка ноль точка ноль"),
            ("2.3.1", "два точка три точка один"),
            ("3.11", "три точка одиннадцать"),
            ("10.15.7", "десять точка пятнадцать точка семь"),
            ("v1.0", "один точка ноль"),
            ("v2.3.1", "два точка три точка один"),
            ("1.0-beta", "один точка ноль бета"),
            ("2.0-alpha", "два точка ноль альфа"),
            ("1.0.0-rc1", "один точка ноль точка ноль эр си один"),
            ("3.11.0-beta.1", "три точка одиннадцать точка ноль бета точка один"),
        ],
    )
    def test_version_to_words(self, number_normalizer, version, expected):
        """Version numbers should use 'точка' as separator."""
        result = number_normalizer.normalize_version(version)
        assert result == expected


class TestDates:
    """Tests for date normalization."""

    @pytest.mark.parametrize(
        "date,expected",
        [
            # ISO format YYYY-MM-DD
            ("2024-01-15", "пятнадцатое января две тысячи двадцать четвёртого года"),
            ("2024-12-31", "тридцать первое декабря две тысячи двадцать четвёртого года"),
            ("2000-01-01", "первое января двухтысячного года"),
            # European format DD.MM.YYYY
            ("15.01.2024", "пятнадцатое января две тысячи двадцать четвёртого года"),
            ("01.12.2023", "первое декабря две тысячи двадцать третьего года"),
            # With slashes
            ("2024/01/15", "пятнадцатое января две тысячи двадцать четвёртого года"),
            ("15/01/2024", "пятнадцатое января две тысячи двадцать четвёртого года"),
        ],
    )
    def test_date_to_words(self, number_normalizer, date, expected):
        """Dates should convert to readable Russian format."""
        result = number_normalizer.normalize_date(date)
        assert result == expected


class TestTimes:
    """Tests for time normalization."""

    @pytest.mark.parametrize(
        "time,expected",
        [
            ("10:00", "десять часов"),
            ("10:30", "десять часов тридцать минут"),
            ("14:15", "четырнадцать часов пятнадцать минут"),
            ("00:00", "ноль часов"),
            ("01:00", "один час"),
            ("02:00", "два часа"),
            ("05:00", "пять часов"),
            ("21:00", "двадцать один час"),
            ("22:30", "двадцать два часа тридцать минут"),
            ("23:59", "двадцать три часа пятьдесят девять минут"),
            # With seconds
            ("10:30:15", "десять часов тридцать минут пятнадцать секунд"),
            ("08:00:00", "восемь часов"),
        ],
    )
    def test_time_to_words(self, number_normalizer, time, expected):
        """Times should have proper hour/minute declension."""
        result = number_normalizer.normalize_time(time)
        assert result == expected


class TestOrdinalNumbers:
    """Tests for ordinal number context detection."""

    @pytest.mark.parametrize(
        "text,expected_contains",
        [
            ("в 1 главе", "первой"),
            ("на 2 этаже", "втором"),
            ("пункт 3", "третий"),
            ("шаг 5", "пятый"),
            ("версия 10", "десятая"),
        ],
    )
    def test_ordinal_context(self, number_normalizer, text, expected_contains):
        """Numbers in ordinal context should use ordinal form."""
        # This test is for future implementation
        # Numbers after certain words should be ordinal
        pass  # placeholder - context detection is complex
