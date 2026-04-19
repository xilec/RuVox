"""Code identifiers and code blocks normalizer."""

import re

from ruvox.tts_pipeline.constants import ARROW_SYMBOLS, GREEK_LETTERS, MATH_SYMBOLS

from .abbreviations import AbbreviationNormalizer
from .numbers import NumberNormalizer
from .symbols import SymbolNormalizer


class CodeIdentifierNormalizer:
    """Normalizes code identifiers (camelCase, snake_case, etc.)."""

    # Common code words and their Russian transliterations
    CODE_WORDS = {
        # Common verbs
        "get": "гет",
        "set": "сет",
        "is": "из",
        "has": "хэз",
        "can": "кэн",
        "on": "он",
        "off": "офф",
        "add": "адд",
        "remove": "ремув",
        "delete": "делит",
        "create": "криейт",
        "update": "апдейт",
        "find": "файнд",
        "search": "сёрч",
        "load": "лоуд",
        "save": "сейв",
        "read": "рид",
        "write": "райт",
        "send": "сенд",
        "receive": "ресив",
        "fetch": "фетч",
        "parse": "парс",
        "format": "формат",
        "convert": "конверт",
        "transform": "трансформ",
        "validate": "валидейт",
        "check": "чек",
        "handle": "хендл",
        "process": "процесс",
        "execute": "экзекьют",
        "run": "ран",
        "start": "старт",
        "stop": "стоп",
        "init": "инит",
        "close": "клоуз",
        "open": "оупен",
        "click": "клик",
        "change": "чейндж",
        "submit": "сабмит",
        "reset": "ризет",
        "clear": "клир",
        "show": "шоу",
        "hide": "хайд",
        "toggle": "тоггл",
        "enable": "энейбл",
        "disable": "дизейбл",
        "calculate": "калькулейт",
        "compute": "компьют",
        "render": "рендер",
        "mount": "маунт",
        "unmount": "анмаунт",
        "dispatch": "диспатч",
        "emit": "эмит",
        "listen": "лисен",
        "subscribe": "сабскрайб",
        "unsubscribe": "ансабскрайб",
        "connect": "коннект",
        "disconnect": "дисконнект",
        "encode": "энкоуд",
        "decode": "декоуд",
        # Common nouns
        "user": "юзер",
        "data": "дата",
        "item": "айтем",
        "list": "лист",
        "array": "эррей",
        "object": "обджект",
        "value": "вэлью",
        "key": "кей",
        "name": "нейм",
        "id": "ай ди",
        "type": "тайп",
        "size": "сайз",
        "count": "каунт",
        "index": "индекс",
        "length": "ленгс",
        "status": "статус",
        "state": "стейт",
        "error": "эррор",
        "message": "мессадж",
        "result": "резалт",
        "response": "респонс",
        "request": "реквест",
        "event": "ивент",
        "action": "экшн",
        "handler": "хендлер",
        "callback": "коллбэк",
        "promise": "промис",
        "function": "функшн",
        "method": "метод",
        "class": "класс",
        "instance": "инстанс",
        "module": "модуль",
        "component": "компонент",
        "element": "элемент",
        "node": "ноуд",
        "child": "чайлд",
        "parent": "парент",
        "root": "рут",
        "path": "пас",
        "url": "ю ар эл",
        "file": "файл",
        "folder": "фолдер",
        "directory": "директори",
        "config": "конфиг",
        "settings": "сеттингс",
        "options": "опшнс",
        "params": "парамс",
        "args": "аргс",
        "props": "пропс",
        "attr": "аттр",
        "attribute": "атрибьют",
        "context": "контекст",
        "session": "сешн",
        "token": "токен",
        "cache": "кэш",
        "store": "стор",
        "service": "сервис",
        "client": "клиент",
        "server": "сервер",
        "database": "датабейз",
        "connection": "коннекшн",
        "query": "квери",
        "table": "тейбл",
        "column": "колумн",
        "row": "роу",
        "record": "рекорд",
        "field": "филд",
        "form": "форм",
        "input": "инпут",
        "output": "аутпут",
        "button": "баттон",
        "link": "линк",
        "image": "имадж",
        "text": "текст",
        "content": "контент",
        "body": "боди",
        "header": "хедер",
        "footer": "футер",
        "nav": "нав",
        "menu": "меню",
        "sidebar": "сайдбар",
        "modal": "модал",
        "popup": "попап",
        "tooltip": "тултип",
        "loader": "лоудер",
        "spinner": "спиннер",
        "icon": "айкон",
        "logo": "лого",
        "avatar": "аватар",
        "badge": "бэдж",
        "tag": "тэг",
        "label": "лейбл",
        "title": "тайтл",
        "description": "дескрипшн",
        "info": "инфо",
        "details": "детейлс",
        "summary": "саммари",
        "total": "тотал",
        "price": "прайс",
        "amount": "эмаунт",
        "balance": "бэлэнс",
        "date": "дейт",
        "time": "тайм",
        "timestamp": "таймстэмп",
        "version": "вёршн",
        "hash": "хэш",
        "string": "стринг",
        "number": "намбер",
        "boolean": "булеан",
        "null": "налл",
        "undefined": "андефайнд",
        "true": "тру",
        "false": "фолс",
        "const": "конст",
        "var": "вар",
        "let": "лет",
        "def": "деф",
        "print": "принт",
        "return": "ретёрн",
        "import": "импорт",
        "export": "экспорт",
        "from": "фром",
        "async": "эсинк",
        "await": "эвейт",
        "try": "трай",
        "catch": "кэтч",
        "throw": "сроу",
        "new": "нью",
        "this": "зис",
        "self": "селф",
        "super": "супер",
        "extends": "экстендс",
        "implements": "имплементс",
        "interface": "интерфейс",
        "abstract": "абстракт",
        "static": "статик",
        "public": "паблик",
        "private": "прайвит",
        "protected": "протектед",
        "final": "файнал",
        "override": "оверрайд",
        "virtual": "виртуал",
        # Common adjectives
        "valid": "вэлид",
        "invalid": "инвэлид",
        "active": "эктив",
        "inactive": "инэктив",
        "enabled": "энейблд",
        "disabled": "дизейблд",
        "visible": "визибл",
        "hidden": "хидден",
        "selected": "селектед",
        "focused": "фокусд",
        "loading": "лоудинг",
        "loaded": "лоудед",
        "pending": "пендинг",
        "success": "саксесс",
        "failed": "фейлд",
        "empty": "эмпти",
        "full": "фулл",
        "old": "олд",
        "first": "фёрст",
        "last": "ласт",
        "next": "некст",
        "prev": "прев",
        "previous": "привиас",
        "current": "каррент",
        "default": "дефолт",
        "custom": "кастом",
        "primary": "праймари",
        "secondary": "секондари",
        "main": "мейн",
        "base": "бейз",
        "max": "макс",
        "min": "мин",
        "all": "олл",
        "none": "нан",
        "any": "эни",
        "some": "сам",
        "my": "май",
        "your": "юр",
        "our": "ауэр",
        "to": "ту",
        "by": "бай",
        "with": "виз",
        "for": "фор",
        "of": "оф",
        "in": "ин",
        "out": "аут",
        "up": "ап",
        "down": "даун",
        "no": "ноу",
        "not": "нот",
        "or": "ор",
        "and": "энд",
        "if": "иф",
        "else": "элс",
        "then": "зен",
        "when": "вен",
        "where": "вер",
        "while": "вайл",
        "do": "ду",
        "case": "кейс",
        "switch": "свитч",
        "break": "брейк",
        "continue": "континью",
        # Common patterns
        "authenticated": "аутентикейтед",
        "timeout": "таймаут",
        "repository": "репозитори",
        "controller": "контроллер",
        "manager": "менеджер",
        "factory": "фэктори",
        "builder": "билдер",
        "adapter": "адаптер",
        "wrapper": "врэппер",
        "helper": "хелпер",
        "util": "утил",
        "utils": "утилз",
        "common": "коммон",
        "shared": "шэрд",
        "global": "глобал",
        "local": "локал",
        "links": "линкс",
        "dir": "дир",
        "awesome": "авесом",
        "package": "пакет",
        "dom": "дом",
        "router": "роутер",
        "react": "риакт",
        "vue": "вью",
        "variable": "вэриабл",
        "side": "сайд",
        "dry": "драй",
        "pip": "пип",
        "install": "инсталл",
        # Python specific
        "str": "стр",
        "repr": "репр",
        "len": "лен",
        "dict": "дикт",
        "int": "инт",
        "float": "флоат",
        "bool": "бул",
        # Abbreviations that need special handling
        "api": "эй пи ай",
        "html": "эйч ти эм эл",
        "http": "эйч ти ти пи",
        "sql": "эс кью эл",
        "utf": "ю ти эф",
        "sha": "ша",
        "json": "джейсон",
        # Common words
        "hello": "хелло",
        "world": "ворлд",
        "plus": "плас",
        "foo": "фу",
        "bar": "бар",
        "baz": "баз",
        "test": "тест",
        "example": "экзампл",
        "demo": "демо",
        "sample": "сэмпл",
        "x": "икс",
        "y": "игрек",
        "z": "зет",
        "a": "эй",
        "b": "би",
        "i": "ай",
        "j": "джей",
        "k": "кей",
        "n": "эн",
        "m": "эм",
    }

    def __init__(self, number_normalizer=None, abbrev_normalizer=None):
        self.number_normalizer = number_normalizer or NumberNormalizer()
        self.abbrev_normalizer = abbrev_normalizer or AbbreviationNormalizer()

    def normalize_camel_case(self, identifier: str) -> str:
        """Convert camelCase/PascalCase to speakable text."""
        if not identifier:
            return identifier

        # Split camelCase: look for transitions from lowercase to uppercase
        # Also handle consecutive capitals (like HTML, API)
        parts = self._split_camel_case(identifier)
        return self._transliterate_parts(parts)

    def _split_camel_case(self, identifier: str) -> list[str]:
        """Split camelCase into words."""
        # Regex to split on:
        # - lowercase followed by uppercase (getUserData -> get User Data)
        # - uppercase followed by uppercase then lowercase (HTMLParser -> HTML Parser)
        # - digit boundaries
        parts = re.findall(r"[A-Z]?[a-z]+|[A-Z]+(?=[A-Z][a-z]|\d|$)|[A-Z]+(?![a-z])|\d+", identifier)
        return [p for p in parts if p]

    def normalize_snake_case(self, identifier: str) -> str:
        """Convert snake_case to speakable text."""
        if not identifier:
            return identifier

        # Handle dunder methods (__init__, __str__, etc.)
        # Strip leading/trailing underscores and split
        stripped = identifier.strip("_")
        if not stripped:
            return identifier

        parts = stripped.split("_")
        parts = [p for p in parts if p]  # Remove empty parts

        return self._transliterate_parts(parts)

    def normalize_kebab_case(self, identifier: str) -> str:
        """Convert kebab-case to speakable text."""
        if not identifier:
            return identifier

        parts = identifier.split("-")
        parts = [p for p in parts if p]

        return self._transliterate_parts(parts)

    def _transliterate_parts(self, parts: list[str]) -> str:
        """Transliterate list of parts to Russian."""
        result = []
        for part in parts:
            part_lower = part.lower()

            # Check if it's a number
            if part.isdigit():
                result.append(self.number_normalizer.normalize_number(part))
            # Check CODE_WORDS dictionary
            elif part_lower in self.CODE_WORDS:
                result.append(self.CODE_WORDS[part_lower])
            # Check if it's an abbreviation (all caps, 2+ letters)
            elif part.isupper() and len(part) >= 2:
                result.append(self.abbrev_normalizer.normalize(part))
            else:
                # Fallback: use basic transliteration
                result.append(self._basic_transliterate(part_lower))

        return " ".join(result)

    def _basic_transliterate(self, word: str) -> str:
        """Basic transliteration for unknown words."""
        translit_map = {
            "a": "а",
            "b": "б",
            "c": "к",
            "d": "д",
            "e": "е",
            "f": "ф",
            "g": "г",
            "h": "х",
            "i": "и",
            "j": "дж",
            "k": "к",
            "l": "л",
            "m": "м",
            "n": "н",
            "o": "о",
            "p": "п",
            "q": "к",
            "r": "р",
            "s": "с",
            "t": "т",
            "u": "у",
            "v": "в",
            "w": "в",
            "x": "кс",
            "y": "й",
            "z": "з",
        }

        result = []
        for char in word:
            if char in translit_map:
                result.append(translit_map[char])
            else:
                result.append(char)

        return "".join(result)


class CodeBlockHandler:
    """Handles code blocks with configurable mode."""

    # Language names mapping (code -> Russian pronunciation)
    LANGUAGE_NAMES = {
        "python": "пайтон",
        "py": "пайтон",
        "javascript": "джаваскрипт",
        "js": "джаваскрипт",
        "typescript": "тайпскрипт",
        "ts": "тайпскрипт",
        "bash": "баш",
        "sh": "шелл",
        "shell": "шелл",
        "zsh": "зи шелл",
        "sql": "эс кью эл",
        "json": "джейсон",
        "yaml": "ямл",
        "yml": "ямл",
        "html": "эйч ти эм эл",
        "css": "си эс эс",
        "go": "го",
        "golang": "голанг",
        "rust": "раст",
        "java": "джава",
        "kotlin": "котлин",
        "swift": "свифт",
        "ruby": "руби",
        "php": "пи эйч пи",
        "c": "си",
        "cpp": "си плюс плюс",
        "c++": "си плюс плюс",
        "cs": "си шарп",
        "csharp": "си шарп",
        "c#": "си шарп",
        "markdown": "маркдаун",
        "md": "маркдаун",
        "xml": "икс эм эл",
        "toml": "томл",
        "dockerfile": "докерфайл",
        "makefile": "мейкфайл",
        "graphql": "граф кью эл",
        "scss": "эс си эс эс",
        "sass": "сасс",
        "less": "лесс",
        "vue": "вью",
        "jsx": "джей эс икс",
        "tsx": "ти эс икс",
        "r": "ар",
        "perl": "перл",
        "lua": "луа",
        "elixir": "эликсир",
        "erlang": "эрланг",
        "haskell": "хаскелл",
        "scala": "скала",
        "clojure": "кложур",
        "dart": "дарт",
        "nginx": "энджинкс",
        "apache": "апачи",
        "terraform": "терраформ",
        "powershell": "пауэршелл",
        "mermaid": "мёрмэйд",
    }

    def __init__(self, mode: str = "full", code_normalizer=None, number_normalizer=None):
        self.mode = mode
        self.code_normalizer = code_normalizer or CodeIdentifierNormalizer()
        self.number_normalizer = number_normalizer or NumberNormalizer()

    def set_mode(self, mode: str) -> None:
        """Switch between 'full' and 'brief' modes."""
        if mode in ("full", "brief"):
            self.mode = mode

    def process(self, code: str, language: str | None = None) -> str:
        """Process code block according to current mode."""
        if self.mode == "brief":
            return self._brief_description(language)
        else:
            return self._full_normalize(code, language)

    def _brief_description(self, language: str | None) -> str:
        """Generate brief description of code block."""
        if language:
            lang_lower = language.lower()
            lang_name = self.LANGUAGE_NAMES.get(lang_lower, lang_lower)
            return f"далее следует пример кода на {lang_name}"
        else:
            return "далее следует блок кода"

    def _full_normalize(self, code: str, language: str | None = None) -> str:
        """Normalize code content for reading."""
        # Simple tokenization and normalization
        # This is a simplified version - could be extended with proper parsing
        result = []
        tokens = self._tokenize(code)

        for token in tokens:
            normalized = self._normalize_token(token)
            if normalized:
                result.append(normalized)

        return " ".join(result)

    # Greek letters mapping (for code blocks)
    GREEK_LETTERS = GREEK_LETTERS

    # Special symbols (arrows + subset of math symbols relevant to code)
    SPECIAL_SYMBOLS = {
        **ARROW_SYMBOLS,
        **{
            k: MATH_SYMBOLS[k]
            for k in (
                "∞",
                "∈",
                "∉",
                "∀",
                "∃",
                "≠",
                "≤",
                "≥",
            )
        },
    }

    def _tokenize(self, code: str) -> list[str]:
        """Simple tokenization of code."""
        # Split on whitespace and common delimiters
        # Keep meaningful tokens
        # Build pattern for Greek letters and special symbols
        greek_pattern = "|".join(re.escape(c) for c in self.GREEK_LETTERS.keys())
        special_pattern = "|".join(re.escape(c) for c in self.SPECIAL_SYMBOLS.keys())

        tokens = re.findall(
            r"[a-zA-Z_][a-zA-Z0-9_]*|"  # identifiers
            r"\d+|"  # numbers
            rf"{greek_pattern}|"  # Greek letters
            rf"{special_pattern}|"  # special symbols (arrows, etc.)
            r"[()[\]{}]|"  # brackets
            r"[+\-*/=<>!&|]+|"  # operators
            r"['\"][^'\"]*['\"]|"  # strings
            r"[.,;:]",  # punctuation
            code,
        )
        return tokens

    def _normalize_token(self, token: str) -> str:
        """Normalize a single token."""
        # Skip empty tokens
        if not token:
            return ""

        # Greek letters
        if token in self.GREEK_LETTERS:
            return self.GREEK_LETTERS[token]

        # Special symbols (arrows, etc.)
        if token in self.SPECIAL_SYMBOLS:
            return self.SPECIAL_SYMBOLS[token]

        # String literals - extract content and transliterate
        if token.startswith(("'", '"')) and token.endswith(("'", '"')):
            content = token[1:-1]
            if not content:
                return ""
            # Transliterate the string content
            lower = content.lower()
            if lower in self.code_normalizer.CODE_WORDS:
                return self.code_normalizer.CODE_WORDS[lower]
            return self.code_normalizer._basic_transliterate(lower)

        # Numbers
        if token.isdigit():
            return self.number_normalizer.normalize_number(token)

        # Identifiers - check pattern
        if re.match(r"^[a-zA-Z_][a-zA-Z0-9_]*$", token):
            # Detect case style
            if "_" in token:
                return self.code_normalizer.normalize_snake_case(token)
            elif any(c.isupper() for c in token[1:]):
                return self.code_normalizer.normalize_camel_case(token)
            else:
                # Simple word
                lower = token.lower()
                if lower in self.code_normalizer.CODE_WORDS:
                    return self.code_normalizer.CODE_WORDS[lower]
                return self.code_normalizer._basic_transliterate(lower)

        # Operators — use shared SYMBOLS dictionary
        symbol = SymbolNormalizer.SYMBOLS.get(token)
        if symbol is not None:
            return symbol

        return ""
