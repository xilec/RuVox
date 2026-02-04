"""English words normalizer - transliteration to Russian phonetics."""


class EnglishNormalizer:
    """Transliterates English words to Russian phonetic spelling."""

    # IT terms with established Russian pronunciation
    IT_TERMS = {
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
        # Unknown word fallbacks
        'hello': 'хеллоу',
        'world': 'ворлд',
        'example': 'экзампл',
        'tutorial': 'тьюториал',
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

    def __init__(self):
        self.custom_terms: dict[str, str] = {}
        # Sort multi-word phrases by length (longest first) for matching
        self._sorted_phrases = sorted(
            self.MULTI_WORD_PHRASES.keys(),
            key=lambda x: len(x),
            reverse=True
        )

    def add_custom_terms(self, terms: dict[str, str]) -> None:
        """Add custom IT terms to the dictionary."""
        self.custom_terms.update({k.lower(): v for k, v in terms.items()})

    def normalize(self, text: str) -> str:
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

        # Fallback: basic transliteration for unknown words
        return self._transliterate(text)

    def _transliterate(self, word: str) -> str:
        """Basic transliteration for unknown English words."""
        # Simple character-based transliteration map
        translit_map = {
            'a': 'а', 'b': 'б', 'c': 'к', 'd': 'д', 'e': 'е',
            'f': 'ф', 'g': 'г', 'h': 'х', 'i': 'и', 'j': 'дж',
            'k': 'к', 'l': 'л', 'm': 'м', 'n': 'н', 'o': 'о',
            'p': 'п', 'q': 'к', 'r': 'р', 's': 'с', 't': 'т',
            'u': 'у', 'v': 'в', 'w': 'в', 'x': 'кс', 'y': 'й',
            'z': 'з',
        }

        result = []
        word_lower = word.lower()
        i = 0
        while i < len(word_lower):
            char = word_lower[i]
            if char in translit_map:
                result.append(translit_map[char])
            else:
                result.append(char)
            i += 1

        return ''.join(result)
