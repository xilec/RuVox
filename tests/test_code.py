"""Tests for code normalizers.

Coverage: camelCase, snake_case, kebab-case, code blocks.
"""

import pytest


class TestCamelCase:
    """Tests for camelCase identifier normalization."""

    @pytest.mark.parametrize(
        "identifier,expected",
        [
            # Simple camelCase
            ("getUserData", "гет юзер дата"),
            ("myVariable", "май вэриабл"),
            ("isValid", "из вэлид"),
            ("hasValue", "хэз вэлью"),
            ("onClick", "он клик"),
            ("onChange", "он чейндж"),
            ("handleSubmit", "хендл сабмит"),
            ("fetchData", "фетч дата"),
            ("parseJSON", "парс джейсон"),
            ("toString", "ту стринг"),
            # With multiple words
            ("getUserDataFromServer", "гет юзер дата фром сервер"),
            ("calculateTotalPrice", "калькулейт тотал прайс"),
            ("isUserAuthenticated", "из юзер аутентикейтед"),
            # With abbreviations
            ("parseHTMLContent", "парс эйч ти эм эл контент"),
            ("getAPIResponse", "гет эй пи ай респонс"),
            ("loadJSONData", "лоуд джейсон дата"),
            ("createURLPath", "криейт ю ар эл пас"),
            # With numbers
            ("getUser2Data", "гет юзер два дата"),
            ("item1Name", "айтем один нейм"),
        ],
    )
    def test_camel_case_split(self, code_normalizer, identifier, expected):
        """camelCase should split on capitals and transliterate."""
        result = code_normalizer.normalize_camel_case(identifier)
        assert result == expected


class TestPascalCase:
    """Tests for PascalCase identifier normalization."""

    @pytest.mark.parametrize(
        "identifier,expected",
        [
            ("UserService", "юзер сервис"),
            ("DataRepository", "дата репозитори"),
            ("HttpClient", "эйч ти ти пи клиент"),
            ("ApiController", "эй пи ай контроллер"),
            ("DatabaseConnection", "датабейз коннекшн"),
            ("EventHandler", "ивент хендлер"),
            ("FileManager", "файл менеджер"),
            ("ConfigLoader", "конфиг лоудер"),
        ],
    )
    def test_pascal_case_split(self, code_normalizer, identifier, expected):
        """PascalCase should be handled like camelCase."""
        result = code_normalizer.normalize_camel_case(identifier)
        assert result == expected


class TestSnakeCase:
    """Tests for snake_case identifier normalization."""

    @pytest.mark.parametrize(
        "identifier,expected",
        [
            # Simple snake_case
            ("get_user_data", "гет юзер дата"),
            ("my_variable", "май вэриабл"),
            ("is_valid", "из вэлид"),
            ("has_value", "хэз вэлью"),
            ("on_click", "он клик"),
            ("handle_submit", "хендл сабмит"),
            ("fetch_data", "фетч дата"),
            ("parse_json", "парс джейсон"),
            # Multiple underscores
            ("get_user_data_from_server", "гет юзер дата фром сервер"),
            ("calculate_total_price", "калькулейт тотал прайс"),
            # With numbers
            ("user_2_data", "юзер два дата"),
            ("item_1_name", "айтем один нейм"),
            # Python dunder methods
            ("__init__", "инит"),
            ("__str__", "стр"),
            ("__repr__", "репр"),
            ("__len__", "лен"),
            # Private/protected
            ("_private_method", "прайвит метод"),
            ("__private_attr", "прайвит аттр"),
        ],
    )
    def test_snake_case_split(self, code_normalizer, identifier, expected):
        """snake_case should split on underscores and transliterate."""
        result = code_normalizer.normalize_snake_case(identifier)
        assert result == expected


class TestKebabCase:
    """Tests for kebab-case identifier normalization."""

    @pytest.mark.parametrize(
        "identifier,expected",
        [
            # CSS classes / HTML attributes
            ("my-component", "май компонент"),
            ("button-primary", "баттон праймари"),
            ("nav-bar", "нав бар"),
            ("side-menu", "сайд меню"),
            ("header-logo", "хедер лого"),
            ("footer-links", "футер линкс"),
            # CLI arguments
            ("output-dir", "аутпут дир"),
            ("config-file", "конфиг файл"),
            ("no-cache", "ноу кэш"),
            ("dry-run", "драй ран"),
            # Package names
            ("my-awesome-package", "май авесом пакет"),
            ("react-dom", "риакт дом"),
            ("vue-router", "вью роутер"),
        ],
    )
    def test_kebab_case_split(self, code_normalizer, identifier, expected):
        """kebab-case should split on hyphens and transliterate."""
        result = code_normalizer.normalize_kebab_case(identifier)
        assert result == expected


class TestCodeBlockBriefMode:
    """Tests for code block handling in brief mode."""

    @pytest.mark.parametrize(
        "language,expected",
        [
            ("python", "далее следует пример кода на пайтон"),
            ("javascript", "далее следует пример кода на джаваскрипт"),
            ("typescript", "далее следует пример кода на тайпскрипт"),
            ("bash", "далее следует пример кода на баш"),
            ("sql", "далее следует пример кода на эс кью эл"),
            ("json", "далее следует пример кода на джейсон"),
            ("yaml", "далее следует пример кода на ямл"),
            ("html", "далее следует пример кода на эйч ти эм эл"),
            ("css", "далее следует пример кода на си эс эс"),
            ("go", "далее следует пример кода на го"),
            ("rust", "далее следует пример кода на раст"),
            ("java", "далее следует пример кода на джава"),
            (None, "далее следует блок кода"),
            ("", "далее следует блок кода"),
        ],
    )
    def test_brief_mode_with_language(self, code_block_handler, language, expected):
        """Brief mode should describe the code block without reading it."""
        code_block_handler.set_mode("brief")
        result = code_block_handler.process("print('hello')", language)
        assert result == expected


class TestCodeBlockFullMode:
    """Tests for code block handling in full mode."""

    @pytest.mark.parametrize(
        "code,expected_contains",
        [
            # Python
            (
                "def hello():\n    print('world')",
                ["деф", "хелло", "принт", "ворлд"],
            ),
            # JavaScript
            (
                "const x = 42;",
                ["конст", "икс", "равно", "сорок два"],
            ),
            # Function call
            (
                "getUserData(userId)",
                ["гет юзер дата", "юзер ай ди"],
            ),
        ],
    )
    def test_full_mode_reads_code(self, code_block_handler, code, expected_contains):
        """Full mode should read and normalize the code."""
        code_block_handler.set_mode("full")
        result = code_block_handler.process(code, "python")
        for expected in expected_contains:
            assert expected in result.lower() or expected in result


class TestModeSwitch:
    """Tests for mode switching."""

    def test_switch_to_brief(self, code_block_handler):
        """Should switch to brief mode."""
        code_block_handler.set_mode("brief")
        assert code_block_handler.mode == "brief"

    def test_switch_to_full(self, code_block_handler):
        """Should switch to full mode."""
        code_block_handler.set_mode("full")
        assert code_block_handler.mode == "full"

    def test_invalid_mode_ignored(self, code_block_handler):
        """Invalid mode should be ignored."""
        original_mode = code_block_handler.mode
        code_block_handler.set_mode("invalid")
        assert code_block_handler.mode == original_mode

    def test_default_mode_is_full(self):
        """Default mode should be 'full'."""
        from fast_tts_rus.normalizers import CodeBlockHandler

        handler = CodeBlockHandler()
        assert handler.mode == "full"


class TestLanguageNames:
    """Tests for programming language name pronunciation."""

    @pytest.mark.parametrize(
        "lang_code,expected_name",
        [
            ("py", "пайтон"),
            ("python", "пайтон"),
            ("js", "джаваскрипт"),
            ("javascript", "джаваскрипт"),
            ("ts", "тайпскрипт"),
            ("typescript", "тайпскрипт"),
            ("sh", "шелл"),
            ("bash", "баш"),
            ("shell", "шелл"),
            ("yml", "ямл"),
            ("yaml", "ямл"),
            ("md", "маркдаун"),
            ("markdown", "маркдаун"),
            ("cpp", "си плюс плюс"),
            ("c++", "си плюс плюс"),
            ("cs", "си шарп"),
            ("csharp", "си шарп"),
            ("dockerfile", "докерфайл"),
            ("makefile", "мейкфайл"),
        ],
    )
    def test_language_code_to_name(self, code_block_handler, lang_code, expected_name):
        """Language codes should map to proper Russian names."""
        code_block_handler.set_mode("brief")
        result = code_block_handler.process("code", lang_code)
        assert expected_name in result


class TestMixedIdentifiers:
    """Tests for identifiers with mixed patterns."""

    @pytest.mark.parametrize(
        "identifier,expected",
        [
            # Constants (SCREAMING_SNAKE_CASE)
            ("MAX_VALUE", "макс вэлью"),
            ("DEFAULT_TIMEOUT", "дефолт таймаут"),
            ("API_BASE_URL", "эй пи ай бейз ю ар эл"),
            # Mixed with numbers
            ("sha256Hash", "ша два пять шесть хэш"),
            ("base64Encode", "бейз шестьдесят четыре энкоуд"),
            ("utf8String", "ю ти эф восемь стринг"),
        ],
    )
    def test_mixed_identifiers(self, code_normalizer, identifier, expected):
        """Mixed pattern identifiers should be handled appropriately."""
        # These may need special handling
        pass  # placeholder for complex cases
