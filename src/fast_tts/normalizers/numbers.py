"""Numbers normalizer - converts numbers to Russian words."""

import re
from num2words import num2words


class NumberNormalizer:
    """Normalizes numbers, dates, times, percentages, etc."""

    # Single digit words for reading after decimal point
    DIGITS = {
        '0': 'ноль',
        '1': 'один',
        '2': 'два',
        '3': 'три',
        '4': 'четыре',
        '5': 'пять',
        '6': 'шесть',
        '7': 'семь',
        '8': 'восемь',
        '9': 'девять',
    }

    # Size units with declension forms (nominative singular, genitive singular, genitive plural)
    # Fourth element is gender: 'm' = masculine, 'f' = feminine, 'n' = neuter
    SIZE_UNITS = {
        # Bytes (masculine)
        'kb': ('килобайт', 'килобайта', 'килобайт', 'm'),
        'mb': ('мегабайт', 'мегабайта', 'мегабайт', 'm'),
        'gb': ('гигабайт', 'гигабайта', 'гигабайт', 'm'),
        'tb': ('терабайт', 'терабайта', 'терабайт', 'm'),
        'кб': ('килобайт', 'килобайта', 'килобайт', 'm'),
        'мб': ('мегабайт', 'мегабайта', 'мегабайт', 'm'),
        'гб': ('гигабайт', 'гигабайта', 'гигабайт', 'm'),
        'тб': ('терабайт', 'терабайта', 'терабайт', 'm'),
        # Time (feminine for секунда, минута; masculine for час)
        'ms': ('миллисекунда', 'миллисекунды', 'миллисекунд', 'f'),
        'sec': ('секунда', 'секунды', 'секунд', 'f'),
        'min': ('минута', 'минуты', 'минут', 'f'),
        'hr': ('час', 'часа', 'часов', 'm'),
        # CSS (masculine for пиксель)
        'px': ('пиксель', 'пикселя', 'пикселей', 'm'),
        'em': ('эм', 'эм', 'эм', 'm'),
        'rem': ('рем', 'рем', 'рем', 'm'),
        'vh': ('ви эйч', 'ви эйч', 'ви эйч', 'm'),
        'vw': ('ви дабл ю', 'ви дабл ю', 'ви дабл ю', 'm'),
    }

    # Month names in genitive case
    MONTHS_GENITIVE = [
        '', 'января', 'февраля', 'марта', 'апреля', 'мая', 'июня',
        'июля', 'августа', 'сентября', 'октября', 'ноября', 'декабря'
    ]

    # Version suffixes
    VERSION_SUFFIXES = {
        'alpha': 'альфа',
        'beta': 'бета',
        'rc': 'эр си',
        'dev': 'дев',
        'stable': 'стейбл',
        'release': 'релиз',
    }

    def normalize_number(self, num_str: str) -> str:
        """Convert integer to Russian words."""
        try:
            num = int(num_str)
            return num2words(num, lang='ru')
        except (ValueError, TypeError):
            return num_str

    def normalize_float(self, float_str: str) -> str:
        """Convert float to Russian words with digit-by-digit after decimal."""
        # Replace comma with dot
        float_str = float_str.replace(',', '.')

        if '.' not in float_str:
            return self.normalize_number(float_str)

        parts = float_str.split('.')
        if len(parts) != 2:
            return float_str

        integer_part = self.normalize_number(parts[0])
        # Read decimal digits one by one
        decimal_part = ' '.join(self.DIGITS.get(d, d) for d in parts[1])

        return f"{integer_part} точка {decimal_part}"

    def normalize_percentage(self, pct_str: str) -> str:
        """Convert percentage to Russian words with proper declension."""
        # Remove % sign
        num_str = pct_str.rstrip('%').strip()

        # Check if it's a float
        if '.' in num_str or ',' in num_str:
            num_words = self.normalize_float(num_str)
            return f"{num_words} процентов"

        # Integer percentage
        try:
            num = int(num_str)
            num_words = num2words(num, lang='ru')
            suffix = self._get_declension(num, ('процент', 'процента', 'процентов'))
            return f"{num_words} {suffix}"
        except (ValueError, TypeError):
            return pct_str

    def normalize_range(self, range_str: str) -> str:
        """Convert range to 'от X до Y' format with genitive case."""
        # Split by various dash types
        parts = re.split(r'[-–—]', range_str)
        if len(parts) != 2:
            return range_str

        try:
            start = int(parts[0].strip())
            end = int(parts[1].strip())

            # Use genitive case for both numbers
            start_words = num2words(start, lang='ru', to='cardinal')
            end_words = num2words(end, lang='ru', to='cardinal')

            # Convert to genitive case (approximation - num2words doesn't support cases directly)
            start_genitive = self._to_genitive(start, start_words)
            end_genitive = self._to_genitive(end, end_words)

            return f"от {start_genitive} до {end_genitive}"
        except (ValueError, TypeError):
            return range_str

    def _to_genitive(self, num: int, words: str) -> str:
        """Convert number words to genitive case (approximate)."""
        # Common replacements for genitive
        replacements = [
            ('один', 'одного'),
            ('одна', 'одной'),
            ('два', 'двух'),
            ('две', 'двух'),
            ('три', 'трёх'),
            ('четыре', 'четырёх'),
            ('пять', 'пяти'),
            ('шесть', 'шести'),
            ('семь', 'семи'),
            ('восемь', 'восьми'),
            ('девять', 'девяти'),
            ('десять', 'десяти'),
            ('одиннадцать', 'одиннадцати'),
            ('двенадцать', 'двенадцати'),
            ('тринадцать', 'тринадцати'),
            ('четырнадцать', 'четырнадцати'),
            ('пятнадцать', 'пятнадцати'),
            ('шестнадцать', 'шестнадцати'),
            ('семнадцать', 'семнадцати'),
            ('восемнадцать', 'восемнадцати'),
            ('девятнадцать', 'девятнадцати'),
            ('двадцать', 'двадцати'),
            ('тридцать', 'тридцати'),
            ('сорок', 'сорока'),
            ('пятьдесят', 'пятидесяти'),
            ('шестьдесят', 'шестидесяти'),
            ('семьдесят', 'семидесяти'),
            ('восемьдесят', 'восьмидесяти'),
            ('девяносто', 'девяноста'),
            ('сто', 'ста'),
            ('двести', 'двухсот'),
            ('триста', 'трёхсот'),
            ('четыреста', 'четырёхсот'),
            ('пятьсот', 'пятисот'),
            ('шестьсот', 'шестисот'),
            ('семьсот', 'семисот'),
            ('восемьсот', 'восьмисот'),
            ('девятьсот', 'девятисот'),
            ('тысяча', 'тысячи'),
            ('тысячи', 'тысяч'),
            ('миллион', 'миллиона'),
            ('миллиона', 'миллионов'),
        ]

        # For year-like numbers (2020-2024), use ordinal genitive
        if num >= 1000 and num <= 9999:
            return self._year_to_ordinal_genitive(num)

        result = words
        for nom, gen in replacements:
            # Replace whole words only
            result = re.sub(r'\b' + nom + r'\b', gen, result)

        return result

    def _year_to_ordinal_genitive(self, year: int) -> str:
        """Convert year to ordinal genitive form."""
        # This is complex in Russian, using approximation
        try:
            ordinal = num2words(year, lang='ru', to='ordinal')
            # Convert ordinal to genitive
            ordinal = ordinal.replace('ый', 'ого').replace('ий', 'ого')
            return ordinal
        except:
            return num2words(year, lang='ru')

    def normalize_size(self, size_str: str) -> str:
        """Convert size with units to Russian words."""
        # Parse number and unit
        match = re.match(r'^([\d.,]+)\s*([a-zA-Zа-яА-Я]+)$', size_str.strip())
        if not match:
            return size_str

        num_str = match.group(1)
        unit = match.group(2).lower()

        # Check if unit is known
        if unit not in self.SIZE_UNITS:
            return size_str

        unit_data = self.SIZE_UNITS[unit]
        unit_forms = unit_data[:3]
        gender = unit_data[3] if len(unit_data) > 3 else 'm'

        # Check if number is float
        if '.' in num_str or ',' in num_str:
            num_words = self.normalize_float(num_str)
            return f"{num_words} {unit_forms[2]}"  # Use plural for floats

        try:
            num = int(num_str)
            # Use gender-aware number conversion
            num_words = self._number_with_gender(num, gender)
            unit_word = self._get_declension(num, unit_forms)
            return f"{num_words} {unit_word}"
        except (ValueError, TypeError):
            return size_str

    def _number_with_gender(self, num: int, gender: str) -> str:
        """Convert number to words with proper gender for 1 and 2."""
        words = num2words(num, lang='ru')

        if gender == 'f':
            # Feminine: один → одна, два → две
            words = re.sub(r'\bодин\b', 'одна', words)
            words = re.sub(r'\bдва\b', 'две', words)
        elif gender == 'n':
            # Neuter: один → одно, два → два (same)
            words = re.sub(r'\bодин\b', 'одно', words)

        return words

    def _get_declension(self, num: int, forms: tuple) -> str:
        """Get correct Russian declension form based on number.

        forms: (singular, genitive_singular, genitive_plural)
        e.g., ('процент', 'процента', 'процентов')
        """
        # Handle negative numbers
        num = abs(num)

        # Get last two digits for proper declension
        last_two = num % 100
        last_one = num % 10

        if 11 <= last_two <= 19:
            return forms[2]  # genitive plural
        elif last_one == 1:
            return forms[0]  # singular
        elif 2 <= last_one <= 4:
            return forms[1]  # genitive singular
        else:
            return forms[2]  # genitive plural

    def normalize_version(self, ver_str: str) -> str:
        """Convert version to Russian words."""
        # Remove leading 'v' or 'V'
        ver_str = ver_str.lstrip('vV')

        # Split by dots and dashes
        parts = []
        current = ""
        for char in ver_str:
            if char == '.':
                if current:
                    parts.append(('num', current))
                    current = ""
                parts.append(('dot', '.'))
            elif char == '-':
                if current:
                    parts.append(('num', current))
                    current = ""
                parts.append(('dash', '-'))
            else:
                current += char
        if current:
            parts.append(('num', current))

        # Convert parts to words
        result = []
        for part_type, part_value in parts:
            if part_type == 'dot':
                result.append('точка')
            elif part_type == 'dash':
                pass  # Skip dash, suffix follows directly
            elif part_type == 'num':
                # Check if it's a known suffix
                suffix_lower = part_value.lower()

                # Handle combined suffix like "rc1", "beta1"
                suffix_match = re.match(r'^([a-zA-Z]+)(\d*)$', part_value)
                if suffix_match:
                    suffix_name = suffix_match.group(1).lower()
                    suffix_num = suffix_match.group(2)

                    if suffix_name in self.VERSION_SUFFIXES:
                        result.append(self.VERSION_SUFFIXES[suffix_name])
                        if suffix_num:
                            result.append(self.normalize_number(suffix_num))
                    elif part_value.isdigit():
                        result.append(self.normalize_number(part_value))
                    else:
                        result.append(part_value)
                elif part_value.isdigit():
                    result.append(self.normalize_number(part_value))
                else:
                    result.append(part_value)

        return ' '.join(result)

    def normalize_date(self, date_str: str) -> str:
        """Convert date to Russian words."""
        # Try to parse different formats
        # ISO: YYYY-MM-DD or YYYY/MM/DD
        # European: DD.MM.YYYY or DD/MM/YYYY

        parts = re.split(r'[-/.]', date_str)
        if len(parts) != 3:
            return date_str

        try:
            # Determine format by checking which part is year (4 digits)
            if len(parts[0]) == 4:
                # ISO format: YYYY-MM-DD
                year = int(parts[0])
                month = int(parts[1])
                day = int(parts[2])
            else:
                # European format: DD.MM.YYYY or DD/MM/YYYY
                day = int(parts[0])
                month = int(parts[1])
                year = int(parts[2])

            # Validate
            if not (1 <= month <= 12 and 1 <= day <= 31 and year > 0):
                return date_str

            # Build Russian date string
            day_ordinal = num2words(day, lang='ru', to='ordinal')
            # Make neuter gender for date
            day_ordinal = day_ordinal.replace('ый', 'ое').replace('ий', 'ее').replace('ой', 'ое')

            month_name = self.MONTHS_GENITIVE[month]

            # Year in genitive
            year_genitive = self._year_to_genitive(year)

            return f"{day_ordinal} {month_name} {year_genitive} года"

        except (ValueError, TypeError, IndexError):
            return date_str

    def _year_to_genitive(self, year: int) -> str:
        """Convert year to genitive ordinal form."""
        # Special case for 2000
        if year == 2000:
            return "двухтысячного"

        # For years like 2024, we need "две тысячи двадцать четвёртого"
        try:
            ordinal = num2words(year, lang='ru', to='ordinal')
            # Convert to genitive - order matters!
            # First handle special cases like "третий" → "третьего"
            ordinal = ordinal.replace('третий', 'третьего')
            ordinal = ordinal.replace('тий', 'того')  # for other -тий endings
            ordinal = ordinal.replace('ый', 'ого')
            ordinal = ordinal.replace('ий', 'ого')
            ordinal = ordinal.replace('ой', 'ого')
            return ordinal
        except:
            return num2words(year, lang='ru')

    def normalize_time(self, time_str: str) -> str:
        """Convert time to Russian words with proper declension."""
        parts = time_str.split(':')
        if len(parts) < 2:
            return time_str

        try:
            hours = int(parts[0])
            minutes = int(parts[1])
            seconds = int(parts[2]) if len(parts) > 2 else 0

            result_parts = []

            # Hours
            hours_word = num2words(hours, lang='ru')
            hours_suffix = self._get_declension(hours, ('час', 'часа', 'часов'))
            result_parts.append(f"{hours_word} {hours_suffix}")

            # Minutes (only if non-zero or seconds are present)
            if minutes > 0 or (seconds > 0):
                minutes_word = num2words(minutes, lang='ru')
                minutes_suffix = self._get_declension(minutes, ('минута', 'минуты', 'минут'))
                result_parts.append(f"{minutes_word} {minutes_suffix}")

            # Seconds (only if non-zero)
            if seconds > 0:
                seconds_word = num2words(seconds, lang='ru')
                seconds_suffix = self._get_declension(seconds, ('секунда', 'секунды', 'секунд'))
                result_parts.append(f"{seconds_word} {seconds_suffix}")

            return ' '.join(result_parts)

        except (ValueError, TypeError):
            return time_str
