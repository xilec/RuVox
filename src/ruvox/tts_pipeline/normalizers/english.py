"""English words normalizer - transliteration to Russian phonetics."""


class EnglishNormalizer:
    """Transliterates English words to Russian phonetic spelling."""

    # IT terms with Russian pronunciation that differs from G2P
    # (Words matching G2P output are handled by _transliterate_g2p)
    IT_TERMS = {
        # Programming languages (special syntax or differs from G2P)
        'c++': 'си плюс плюс',
        'c#': 'си шарп',
        'f#': 'эф шарп',
        'haskell': 'хаскелл',
        'ocaml': 'окамл',
        'erlang': 'эрланг',
        'elixir': 'эликсир',
        'clojure': 'кложур',
        'prolog': 'пролог',
        'fortran': 'фортран',
        'cobol': 'кобол',
        'pascal': 'паскаль',
        'delphi': 'делфи',
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
        # English numerals (where G2P differs)
        'zero': 'зиро',
        'seven': 'сэвен',
        'ten': 'тен',
        'eleven': 'илэвен',
        'twelve': 'твелв',
        'thirteen': 'сёртин',
        'seventeen': 'сэвентин',
        'twenty': 'твенти',
        # Common code terms
        'eval': 'эвал',
        'plus': 'плас',
        'succ': 'сакс',
        'synthesize': 'синтесайз',
        'addition': 'эдишн',
        # Common type/term names
        'nat': 'нат',
        'uint': 'юинт',
        'float': 'флоат',
        'double': 'дабл',
        'trait': 'трейт',
        'traits': 'трейтс',
        'impl': 'импл',
        'async': 'асинк',
        'await': 'эвейт',
        'const': 'конст',
        'static': 'статик',
        'override': 'оверрайд',
        'virtual': 'виртуал',
        'abstract': 'абстракт',
        'private': 'прайвит',
        'protected': 'протектед',
        'generic': 'дженерик',
        'template': 'темплейт',
        # Git/VCS terms
        'feature': 'фича',
        'branch': 'бранч',
        'merge': 'мёрдж',
        'commit': 'коммит',
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
        'refactor': 'рефакторинг',
        'agile': 'эджайл',
        'scrum': 'скрам',
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
        'webpack': 'вебпак',
        # Programming languages
        'python': 'пайтон',
        'typescript': 'тайпскрипт',
        'rust': 'раст',
        'golang': 'голанг',
        'kotlin': 'котлин',
        # Frameworks and tools
        'react': 'риакт',
        'angular': 'ангуляр',
        'vue': 'вью',
        'svelte': 'свелт',
        'next': 'некст',
        'express': 'экспресс',
        'django': 'джанго',
        'flask': 'фласк',
        'fastapi': 'фаст эй пи ай',
        'laravel': 'ларавел',
        'redis': 'редис',
        'mongo': 'монго',
        'postgres': 'постгрес',
        'github': 'гитхаб',
        'jira': 'джира',
        'slack': 'слэк',
        'postman': 'постман',
        # Additional common terms
        'request': 'реквест',
        'trace': 'трейс',
        'daily': 'дейли',
        'standup': 'стендап',
        'hot': 'хот',
        'reload': 'релоуд',
        'tech': 'тек',
        'debt': 'дет',
        'code': 'код',
        'smell': 'смелл',
        'best': 'бест',
        'practice': 'практис',
        'use': 'юз',
        'case': 'кейс',
        # Common words in paths/URLs
        'home': 'хоум',
        'docs': 'докс',
        'user': 'юзер',
        'users': 'юзерс',
        'admin': 'админ',
        'support': 'саппорт',
        'config': 'конфиг',
        'data': 'дата',
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
        'svg': 'эс ви джи',
        'mp3': 'эм пэ три',
        'mp4': 'эм пэ четыре',
        # Common words
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

    # ARPAbet to Russian phonetic mapping (for G2P fallback)
    ARPABET_MAP = {
        # Vowels
        'AA': 'а',    # father
        'AE': 'э',    # cat
        'AH': 'а',    # but (schwa) - можно 'э' в безударной
        'AO': 'о',    # dog
        'AW': 'ау',   # cow
        'AY': 'ай',   # my
        'EH': 'э',    # bed
        'ER': 'ер',   # bird
        'EY': 'эй',   # say
        'IH': 'и',    # bit
        'IY': 'и',    # bee
        'OW': 'оу',   # go
        'OY': 'ой',   # boy
        'UH': 'у',    # book
        'UW': 'у',    # too
        # Consonants
        'B': 'б',
        'CH': 'ч',
        'D': 'д',
        'DH': 'з',    # the (voiced th)
        'F': 'ф',
        'G': 'г',
        'HH': 'х',
        'JH': 'дж',
        'K': 'к',
        'L': 'л',
        'M': 'м',
        'N': 'н',
        'NG': 'нг',
        'P': 'п',
        'R': 'р',
        'S': 'с',
        'SH': 'ш',
        'T': 'т',
        'TH': 'с',    # think (unvoiced th)
        'V': 'в',
        'W': 'в',
        'Y': 'й',
        'Z': 'з',
        'ZH': 'ж',
    }

    # Simple transliteration map (fallback when G2P not available)
    TRANSLIT_MAP = {
        # Digraphs and common combinations (order matters - check longer first)
        'sh': 'ш', 'ch': 'ч', 'th': 'с', 'ph': 'ф', 'wh': 'в',
        'ck': 'к', 'gh': 'г', 'ng': 'нг', 'qu': 'кв',
        'tion': 'шн', 'sion': 'жн',  # common suffixes
        'ee': 'и', 'oo': 'у', 'ea': 'и', 'ou': 'ау', 'ow': 'оу',
        'ai': 'эй', 'ay': 'эй', 'ey': 'эй', 'ei': 'эй',
        'ie': 'и', 'oa': 'оу', 'oi': 'ой', 'oy': 'ой',
        'au': 'о', 'aw': 'о', 'ew': 'ью',
        # Single letters
        'a': 'а', 'b': 'б', 'c': 'к', 'd': 'д', 'e': 'е',
        'f': 'ф', 'g': 'г', 'h': 'х', 'i': 'и', 'j': 'дж',
        'k': 'к', 'l': 'л', 'm': 'м', 'n': 'н', 'o': 'о',
        'p': 'п', 'q': 'к', 'r': 'р', 's': 'с', 't': 'т',
        'u': 'у', 'v': 'в', 'w': 'в', 'x': 'кс', 'y': 'и',
        'z': 'з',
    }

    # Sorted keys by length (longest first) for proper matching
    _TRANSLIT_KEYS = sorted(TRANSLIT_MAP.keys(), key=len, reverse=True)

    def __init__(self, use_g2p: bool = True):
        self.custom_terms: dict[str, str] = {}
        # Sort multi-word phrases by length (longest first) for matching
        self._sorted_phrases = sorted(
            self.MULTI_WORD_PHRASES.keys(),
            key=lambda x: len(x),
            reverse=True
        )
        # Track unknown words that were transliterated via fallback
        self._unknown_words: dict[str, str] = {}
        # Cache for transliteration results
        self._transliterate_cache: dict[str, str] = {}

        # Lazy-load G2P
        self._g2p = None
        self._use_g2p = use_g2p
        self._g2p_available = None  # None = not checked yet

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

    def _get_g2p(self):
        """Lazy-load G2P model."""
        if self._g2p_available is None:
            try:
                from g2p_en import G2p
                self._g2p = G2p()
                self._g2p_available = True
            except ImportError:
                self._g2p_available = False
        return self._g2p if self._g2p_available else None

    def _transliterate_g2p(self, word: str) -> str | None:
        """Transliterate using G2P (ARPAbet phonemes)."""
        if not self._use_g2p:
            return None

        g2p = self._get_g2p()
        if g2p is None:
            return None

        try:
            phonemes = g2p(word)
            result = []
            for phoneme in phonemes:
                # Remove stress markers (0, 1, 2)
                phoneme_clean = phoneme.rstrip('012')
                if phoneme_clean in self.ARPABET_MAP:
                    result.append(self.ARPABET_MAP[phoneme_clean])
                elif phoneme_clean:
                    # Unknown phoneme, keep as-is
                    result.append(phoneme_clean.lower())
            return ''.join(result)
        except Exception:
            return None

    def _transliterate_simple(self, word: str) -> str:
        """Simple transliteration using character mapping."""
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

    def _transliterate(self, word: str) -> str:
        """Transliterate unknown English word. Uses G2P if available."""
        key = word.lower()
        if key in self._transliterate_cache:
            return self._transliterate_cache[key]

        # Try G2P first
        result = self._transliterate_g2p(word)
        if not result:
            # Fallback to simple transliteration
            result = self._transliterate_simple(word)

        self._transliterate_cache[key] = result
        return result
