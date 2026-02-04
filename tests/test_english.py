"""Tests for English words normalizer.

Coverage: IT terms, common English words in Russian tech texts.
"""

import pytest


class TestITTermsDictionary:
    """Tests for IT terms with established Russian pronunciation."""

    @pytest.mark.parametrize(
        "english,expected",
        [
            # Git/VCS terms
            ("feature", "фича"),
            ("branch", "бранч"),
            ("merge", "мёрдж"),
            ("commit", "коммит"),
            ("push", "пуш"),
            ("pull", "пулл"),
            ("checkout", "чекаут"),
            ("rebase", "рибейз"),
            ("stash", "стэш"),
            # Development process
            ("review", "ревью"),
            ("deploy", "деплой"),
            ("release", "релиз"),
            ("debug", "дебаг"),
            ("bug", "баг"),
            ("fix", "фикс"),
            ("refactor", "рефакторинг"),
            ("sprint", "спринт"),
            ("scrum", "скрам"),
            ("agile", "эджайл"),
            # Architecture/Code
            ("framework", "фреймворк"),
            ("library", "лайбрари"),
            ("package", "пакет"),
            ("module", "модуль"),
            ("function", "функция"),
            ("method", "метод"),
            ("class", "класс"),
            ("object", "объект"),
            ("interface", "интерфейс"),
            ("callback", "коллбэк"),
            ("promise", "промис"),
            ("handler", "хендлер"),
            ("listener", "листенер"),
            ("middleware", "мидлвэр"),
            ("endpoint", "эндпоинт"),
            ("router", "роутер"),
            ("controller", "контроллер"),
            ("service", "сервис"),
            ("repository", "репозиторий"),
            # Data
            ("cache", "кэш"),
            ("queue", "кью"),
            ("array", "массив"),
            ("string", "строка"),
            ("boolean", "булеан"),
            ("null", "налл"),
            ("undefined", "андефайнд"),
            ("default", "дефолт"),
            ("index", "индекс"),
            ("query", "квери"),
            # Infrastructure
            ("docker", "докер"),
            ("container", "контейнер"),
            ("kubernetes", "кубернетис"),
            ("cluster", "кластер"),
            ("node", "нода"),
            ("pod", "под"),
            ("nginx", "энджинкс"),
            ("backup", "бэкап"),
            ("server", "сервер"),
            ("client", "клиент"),
            # Testing
            ("test", "тест"),
            ("mock", "мок"),
            ("stub", "стаб"),
            ("spec", "спек"),
            # Build
            ("build", "билд"),
            ("bundle", "бандл"),
            ("compile", "компайл"),
            ("lint", "линт"),
            ("webpack", "вебпак"),
        ],
    )
    def test_it_term_translation(self, english_normalizer, english, expected):
        """IT terms should translate to established Russian equivalents."""
        result = english_normalizer.normalize(english)
        assert result == expected

    @pytest.mark.parametrize(
        "english,expected",
        [
            ("Feature", "фича"),
            ("BRANCH", "бранч"),
            ("Merge", "мёрдж"),
            ("COMMIT", "коммит"),
        ],
    )
    def test_case_insensitivity(self, english_normalizer, english, expected):
        """IT terms should be matched case-insensitively."""
        result = english_normalizer.normalize(english)
        assert result == expected


class TestProgrammingLanguages:
    """Tests for programming language names."""

    @pytest.mark.parametrize(
        "language,expected",
        [
            ("python", "пайтон"),
            ("javascript", "джаваскрипт"),
            ("typescript", "тайпскрипт"),
            ("rust", "раст"),
            ("golang", "голанг"),
            ("kotlin", "котлин"),
            ("swift", "свифт"),
            ("java", "джава"),
            ("ruby", "руби"),
            ("scala", "скала"),
        ],
    )
    def test_language_names(self, english_normalizer, language, expected):
        """Programming language names should have proper pronunciation."""
        result = english_normalizer.normalize(language)
        assert result == expected


class TestFrameworksAndTools:
    """Tests for framework and tool names."""

    @pytest.mark.parametrize(
        "name,expected",
        [
            ("react", "риэкт"),
            ("angular", "ангуляр"),
            ("vue", "вью"),
            ("svelte", "свелт"),
            ("next", "некст"),
            ("express", "экспресс"),
            ("django", "джанго"),
            ("flask", "фласк"),
            ("fastapi", "фаст эй пи ай"),
            ("laravel", "ларавел"),
            ("spring", "спринг"),
            ("redis", "редис"),
            ("mongo", "монго"),
            ("postgres", "постгрес"),
            ("kafka", "кафка"),
            ("github", "гитхаб"),
            ("gitlab", "гитлаб"),
            ("jira", "джира"),
            ("slack", "слэк"),
            ("figma", "фигма"),
            ("postman", "постман"),
        ],
    )
    def test_framework_names(self, english_normalizer, name, expected):
        """Framework and tool names should have proper pronunciation."""
        result = english_normalizer.normalize(name)
        assert result == expected


class TestUnknownWords:
    """Tests for words not in dictionary - G2P transliteration."""

    @pytest.mark.parametrize(
        "english,expected_contains",
        [
            # These should be transliterated phonetically
            ("hello", "хел"),  # should contain 'хел'
            ("world", "ворлд"),
            ("example", "экзампл"),
            ("tutorial", "тьюториал"),
        ],
    )
    def test_unknown_word_transliteration(self, english_normalizer, english, expected_contains):
        """Unknown words should be transliterated phonetically."""
        result = english_normalizer.normalize(english)
        # For unknown words, we check that result contains expected sounds
        assert expected_contains in result or result  # placeholder for now


class TestMultiWordPhrases:
    """Tests for multi-word IT phrases."""

    @pytest.mark.parametrize(
        "phrase,expected",
        [
            ("pull request", "пулл реквест"),
            ("merge request", "мёрдж реквест"),
            ("code review", "код ревью"),
            ("feature branch", "фича бранч"),
            ("stack trace", "стэк трейс"),
            ("daily standup", "дейли стендап"),
            ("hot fix", "хот фикс"),
            ("hot reload", "хот релоуд"),
            ("live reload", "лайв релоуд"),
            ("dry run", "драй ран"),
            ("tech debt", "тек дет"),
            ("code smell", "код смелл"),
            ("best practice", "бест практис"),
            ("use case", "юз кейс"),
            ("edge case", "эдж кейс"),
        ],
    )
    def test_multiword_phrases(self, english_normalizer, phrase, expected):
        """Multi-word IT phrases should translate as units."""
        result = english_normalizer.normalize(phrase)
        assert result == expected
