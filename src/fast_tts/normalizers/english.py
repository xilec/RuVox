"""English words normalizer - transliteration to Russian phonetics."""


class EnglishNormalizer:
    """Transliterates English words to Russian phonetic spelling."""

    # IT terms with established Russian pronunciation
    IT_TERMS = {
        # Programming languages (including special syntax)
        'c++': 'си плюс плюс',
        'c#': 'си шарп',
        'f#': 'эф шарп',
        'c': 'си',
        'lean': 'лин',
        'haskell': 'хаскелл',
        'ocaml': 'окамл',
        'erlang': 'эрланг',
        'elixir': 'эликсир',
        'clojure': 'кложур',
        'lisp': 'лисп',
        'prolog': 'пролог',
        'fortran': 'фортран',
        'cobol': 'кобол',
        'pascal': 'паскаль',
        'delphi': 'делфи',
        'lua': 'луа',
        'perl': 'перл',
        'php': 'пи эйч пи',
        'sql': 'эс кью эль',
        'html': 'эйч ти эм эль',
        'css': 'си эс эс',
        'xml': 'икс эм эль',
        'json': 'джейсон',
        'yaml': 'ямл',
        'toml': 'томл',
        'js': 'джи эс',
        'ts': 'ти эс',
        # Common type/term names
        'nat': 'нат',
        'int': 'инт',
        'uint': 'юинт',
        'float': 'флоат',
        'double': 'дабл',
        'bool': 'бул',
        'char': 'чар',
        'byte': 'байт',
        'void': 'войд',
        'enum': 'энам',
        'struct': 'стракт',
        'trait': 'трейт',
        'traits': 'трейтс',
        'impl': 'импл',
        'async': 'асинк',
        'await': 'эвейт',
        'const': 'конст',
        'static': 'статик',
        'final': 'файнал',
        'override': 'оверрайд',
        'virtual': 'виртуал',
        'abstract': 'абстракт',
        'public': 'паблик',
        'private': 'прайвит',
        'protected': 'протектед',
        'generic': 'дженерик',
        'template': 'темплейт',
        # Git/VCS terms
        'feature': 'фича',
        'branch': 'бранч',
        'merge': 'мёрдж',
        'commit': 'коммит',
        'push': 'пуш',
        'pull': 'пулл',
        'checkout': 'чекаут',
        'rebase': 'рибейз',
        'stash': 'стэш',
        # Development process
        'review': 'ревью',
        'deploy': 'деплой',
        'release': 'релиз',
        'debug': 'дебаг',
        'bug': 'баг',
        'fix': 'фикс',
        'refactor': 'рефакторинг',
        'sprint': 'спринт',
        'scrum': 'скрам',
        'agile': 'эджайл',
        # Architecture/Code
        'framework': 'фреймворк',
        'library': 'лайбрари',
        'package': 'пакет',
        'module': 'модуль',
        'function': 'функция',
        'method': 'метод',
        'class': 'класс',
        'object': 'объект',
        'interface': 'интерфейс',
        'callback': 'коллбэк',
        'promise': 'промис',
        'handler': 'хендлер',
        'listener': 'листенер',
        'middleware': 'мидлвэр',
        'endpoint': 'эндпоинт',
        'router': 'роутер',
        'controller': 'контроллер',
        'service': 'сервис',
        'repository': 'репозиторий',
        # Data
        'cache': 'кэш',
        'queue': 'кью',
        'array': 'массив',
        'string': 'строка',
        'boolean': 'булеан',
        'null': 'налл',
        'undefined': 'андефайнд',
        'default': 'дефолт',
        'index': 'индекс',
        'query': 'квери',
        # Infrastructure
        'docker': 'докер',
        'container': 'контейнер',
        'kubernetes': 'кубернетис',
        'cluster': 'кластер',
        'node': 'нода',
        'pod': 'под',
        'nginx': 'энджинкс',
        'backup': 'бэкап',
        'server': 'сервер',
        'client': 'клиент',
        # Testing
        'test': 'тест',
        'mock': 'мок',
        'stub': 'стаб',
        'spec': 'спек',
        # Build
        'build': 'билд',
        'bundle': 'бандл',
        'compile': 'компайл',
        'lint': 'линт',
        'webpack': 'вебпак',
        # Programming languages
        'python': 'пайтон',
        'javascript': 'джаваскрипт',
        'typescript': 'тайпскрипт',
        'rust': 'раст',
        'golang': 'голанг',
        'kotlin': 'котлин',
        'swift': 'свифт',
        'java': 'джава',
        'ruby': 'руби',
        'scala': 'скала',
        # Frameworks and tools
        'react': 'риэкт',
        'angular': 'ангуляр',
        'vue': 'вью',
        'svelte': 'свелт',
        'next': 'некст',
        'express': 'экспресс',
        'django': 'джанго',
        'flask': 'фласк',
        'fastapi': 'фаст эй пи ай',
        'laravel': 'ларавел',
        'spring': 'спринг',
        'redis': 'редис',
        'mongo': 'монго',
        'postgres': 'постгрес',
        'kafka': 'кафка',
        'github': 'гитхаб',
        'gitlab': 'гитлаб',
        'jira': 'джира',
        'slack': 'слэк',
        'figma': 'фигма',
        'postman': 'постман',
        # Additional common terms
        'request': 'реквест',
        'trace': 'трейс',
        'stack': 'стэк',
        'daily': 'дейли',
        'standup': 'стендап',
        'hot': 'хот',
        'reload': 'релоуд',
        'live': 'лайв',
        'dry': 'драй',
        'run': 'ран',
        'tech': 'тек',
        'debt': 'дет',
        'code': 'код',
        'smell': 'смелл',
        'best': 'бест',
        'practice': 'практис',
        'use': 'юз',
        'case': 'кейс',
        'edge': 'эдж',
        # Common words in paths/URLs
        'docs': 'докс',
        'home': 'хоум',
        'user': 'юзер',
        'users': 'юзерс',
        'admin': 'админ',
        'support': 'саппорт',
        'config': 'конфиг',
        'data': 'дата',
        'file': 'файл',
        'files': 'файлс',
        'download': 'даунлоад',
        'upload': 'аплоад',
        'report': 'репорт',
        'documents': 'документс',
        'localhost': 'локалхост',
        'api': 'эй пи ай',
        'app': 'апп',
        'web': 'веб',
        'src': 'сорс',
        'lib': 'либ',
        'bin': 'бин',
        'var': 'вар',
        'log': 'лог',
        'tmp': 'темп',
        'etc': 'етс',
        'opt': 'опт',
        # File extensions
        'pdf': 'пдф',
        'doc': 'док',
        'txt': 'тэкст',
        'csv': 'си эс ви',
        'png': 'пнг',
        'jpg': 'джэйпег',
        'gif': 'гиф',
        'svg': 'эс ви джи',
        'mp3': 'эм пэ три',
        'mp4': 'эм пэ четыре',
        # Unknown word fallbacks
        'hello': 'хеллоу',
        'world': 'ворлд',
        'example': 'экзампл',
        'tutorial': 'тьюториал',
        'company': 'компани',
        'repo': 'репо',
    }

    # Multi-word phrases (checked first, longest match)
    MULTI_WORD_PHRASES = {
        'pull request': 'пулл реквест',
        'merge request': 'мёрдж реквест',
        'code review': 'код ревью',
        'feature branch': 'фича бранч',
        'stack trace': 'стэк трейс',
        'daily standup': 'дейли стендап',
        'hot fix': 'хот фикс',
        'hot reload': 'хот релоуд',
        'live reload': 'лайв релоуд',
        'dry run': 'драй ран',
        'tech debt': 'тек дет',
        'code smell': 'код смелл',
        'best practice': 'бест практис',
        'use case': 'юз кейс',
        'edge case': 'эдж кейс',
    }

    # Improved transliteration map with common letter combinations
    TRANSLIT_MAP = {
        # Digraphs and common combinations (order matters - check longer first)
        'sh': 'ш', 'ch': 'ч', 'th': 'з', 'ph': 'ф', 'wh': 'в',
        'ck': 'к', 'gh': 'г', 'ng': 'нг', 'qu': 'кв',
        'ee': 'и', 'oo': 'у', 'ea': 'и', 'ou': 'ау', 'ow': 'оу',
        'ai': 'эй', 'ay': 'эй', 'ey': 'эй', 'ei': 'эй',
        'ie': 'и', 'oa': 'оу', 'oi': 'ой', 'oy': 'ой',
        'au': 'о', 'aw': 'о', 'ew': 'ью',
        # Single letters
        'a': 'а', 'b': 'б', 'c': 'к', 'd': 'д', 'e': 'е',
        'f': 'ф', 'g': 'г', 'h': 'х', 'i': 'и', 'j': 'дж',
        'k': 'к', 'l': 'л', 'm': 'м', 'n': 'н', 'o': 'о',
        'p': 'п', 'q': 'к', 'r': 'р', 's': 'с', 't': 'т',
        'u': 'у', 'v': 'в', 'w': 'в', 'x': 'кс', 'y': 'й',
        'z': 'з',
    }

    # Sorted keys by length (longest first) for proper matching
    _TRANSLIT_KEYS = sorted(TRANSLIT_MAP.keys(), key=len, reverse=True)

    def __init__(self):
        self.custom_terms: dict[str, str] = {}
        # Sort multi-word phrases by length (longest first) for matching
        self._sorted_phrases = sorted(
            self.MULTI_WORD_PHRASES.keys(),
            key=lambda x: len(x),
            reverse=True
        )
        # Track unknown words that were transliterated via fallback
        self._unknown_words: dict[str, str] = {}

    def add_custom_terms(self, terms: dict[str, str]) -> None:
        """Add custom IT terms to the dictionary."""
        self.custom_terms.update({k.lower(): v for k, v in terms.items()})

    def normalize(self, text: str, track_unknown: bool = True) -> str:
        """Convert English word or phrase to Russian phonetic spelling."""
        if not text:
            return text

        text_lower = text.lower()

        # Check multi-word phrases first (longest match)
        for phrase in self._sorted_phrases:
            if text_lower == phrase:
                return self.MULTI_WORD_PHRASES[phrase]

        # Check custom terms
        if text_lower in self.custom_terms:
            return self.custom_terms[text_lower]

        # Check IT terms dictionary
        if text_lower in self.IT_TERMS:
            return self.IT_TERMS[text_lower]

        # Fallback: transliteration for unknown words
        result = self._transliterate(text)

        # Track unknown word
        if track_unknown and text_lower not in self._unknown_words:
            self._unknown_words[text_lower] = result

        return result

    def get_unknown_words(self) -> dict[str, str]:
        """Get dictionary of unknown words that were transliterated."""
        return self._unknown_words.copy()

    def clear_unknown_words(self) -> None:
        """Clear the unknown words tracking."""
        self._unknown_words.clear()

    def _transliterate(self, word: str) -> str:
        """Transliterate unknown English word using improved rules."""
        result = []
        word_lower = word.lower()
        i = 0

        while i < len(word_lower):
            matched = False
            # Try to match longer combinations first
            for key in self._TRANSLIT_KEYS:
                if word_lower[i:i+len(key)] == key:
                    result.append(self.TRANSLIT_MAP[key])
                    i += len(key)
                    matched = True
                    break

            if not matched:
                # Keep character as-is (digits, special chars, etc.)
                result.append(word_lower[i])
                i += 1

        return ''.join(result)
