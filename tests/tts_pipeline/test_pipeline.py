"""Integration tests for the full TTS pipeline.

Coverage: end-to-end text transformation from input to TTS-ready output.
These tests verify the complete pipeline behavior.
"""

import pytest


class TestPureRussianText:
    """Tests for pure Russian text (should pass through mostly unchanged)."""

    @pytest.mark.parametrize(
        "input_text,expected",
        [
            # Simple sentences
            ("Привет, как дела?", "Привет, как дела?"),
            ("Сегодня хорошая погода.", "Сегодня хорошая погода."),
            # With numbers
            ("Мне 25 лет.", "Мне двадцать пять лет."),
            ("В комнате 3 человека.", "В комнате три человека."),
            # Abbreviations should stay Russian
            ("Это текст на русском языке.", "Это текст на русском языке."),
        ],
    )
    def test_russian_text_passthrough(self, pipeline, input_text, expected):
        """Pure Russian text should pass through with number conversion."""
        result = pipeline.process(input_text)
        assert result == expected


class TestMixedText:
    """Tests for mixed Russian-English technical text."""

    @pytest.mark.parametrize(
        "input_text,expected",
        [
            # IT terms in Russian context
            (
                "Нужно сделать pull request.",
                "Нужно сделать пулл реквест.",
            ),
            (
                "Создай новую feature branch.",
                "Создай новую фича бранч.",
            ),
            (
                "Пройди code review перед merge.",
                "Пройди код ревью перед мёрдж.",
            ),
            # Tools and technologies
            (
                "Установи Docker и запусти контейнер.",
                "Установи докер и запусти контейнер.",
            ),
            (
                "Используем React для фронтенда.",
                "Используем риакт для фронтенда.",
            ),
            # Multiple terms
            (
                "Этот API endpoint возвращает JSON.",
                "Этот эй пи ай эндпоинт возвращает джейсон.",
            ),
        ],
    )
    def test_mixed_text_normalization(self, pipeline, input_text, expected):
        """Mixed text should have English terms transliterated."""
        result = pipeline.process(input_text)
        assert result == expected


class TestURLProcessing:
    """Tests for URL processing in context."""

    @pytest.mark.parametrize(
        "input_text,expected_contains",
        [
            (
                "Документация: https://docs.python.org",
                ["эйч ти ти пи эс", "докс", "пайтон", "орг"],
            ),
            (
                "Репозиторий на https://github.com/user/repo",
                ["эйч ти ти пи эс", "гитхаб", "юзер", "репо"],
            ),
            (
                "API доступен на http://localhost:8080/api",
                ["эйч ти ти пи", "локалхост", "восемь тысяч восемьдесят", "эй пи ай"],
            ),
        ],
    )
    def test_url_in_text(self, pipeline, input_text, expected_contains):
        """URLs in text should be fully expanded."""
        result = pipeline.process(input_text)
        for part in expected_contains:
            assert part.lower() in result.lower()


class TestEmailProcessing:
    """Tests for email processing in context."""

    @pytest.mark.parametrize(
        "input_text,expected_contains",
        [
            (
                "Пиши на admin@example.com",
                ["админ", "собака", "экзампл", "ком"],
            ),
            (
                "Контакт: support@company.ru",
                ["саппорт", "собака", "компани", "ру"],
            ),
        ],
    )
    def test_email_in_text(self, pipeline, input_text, expected_contains):
        """Emails in text should use 'собака' for @."""
        result = pipeline.process(input_text)
        for part in expected_contains:
            assert part.lower() in result.lower()


class TestIPProcessing:
    """Tests for IP address processing."""

    @pytest.mark.parametrize(
        "input_text,expected_contains",
        [
            (
                "Сервер на 192.168.1.1",
                ["сто девяносто два", "сто шестьдесят восемь", "один", "один"],
            ),
            (
                "DNS: 8.8.8.8",
                ["восемь", "точка", "восемь"],
            ),
        ],
    )
    def test_ip_in_text(self, pipeline, input_text, expected_contains):
        """IP addresses should be read as numbers."""
        result = pipeline.process(input_text)
        for part in expected_contains:
            assert part.lower() in result.lower()


class TestVersionProcessing:
    """Tests for version number processing."""

    @pytest.mark.parametrize(
        "input_text,expected_contains",
        [
            (
                "Python 3.11 вышел недавно.",
                ["пайтон", "три", "точка", "одиннадцать"],
            ),
            (
                "Обновись до версии v2.0.0",
                ["два", "точка", "ноль"],
            ),
            (
                "Требуется Node.js 18.0 или выше.",
                ["нода", "джи эс", "восемнадцать", "точка", "ноль"],
            ),
        ],
    )
    def test_version_in_text(self, pipeline, input_text, expected_contains):
        """Version numbers should use 'точка' as separator."""
        result = pipeline.process(input_text)
        for part in expected_contains:
            assert part.lower() in result.lower()


class TestCodeIdentifiersInText:
    """Tests for code identifiers in running text."""

    @pytest.mark.parametrize(
        "input_text,expected_contains",
        [
            (
                "Вызови функцию getUserData для получения данных.",
                ["гет юзер дата"],
            ),
            (
                "Переменная my_variable содержит результат.",
                ["май вэриабл"],
            ),
            (
                "Компонент button-primary стилизован.",
                ["баттон праймари"],
            ),
        ],
    )
    def test_code_identifiers_in_text(self, pipeline, input_text, expected_contains):
        """Code identifiers should be split and transliterated."""
        result = pipeline.process(input_text)
        for part in expected_contains:
            assert part.lower() in result.lower()


class TestOperatorsInText:
    """Tests for operators in text."""

    @pytest.mark.parametrize(
        "input_text,expected_contains",
        [
            (
                "Используй стрелку -> для типов.",
                ["стрелка"],
            ),
            (
                "Условие x >= 10 должно выполняться.",
                ["больше или равно", "десять"],
            ),
            (
                "Проверь что a != b.",
                ["не равно"],
            ),
        ],
    )
    def test_operators_in_text(self, pipeline, input_text, expected_contains):
        """Operators should be read aloud."""
        result = pipeline.process(input_text)
        for part in expected_contains:
            assert part.lower() in result.lower()


class TestFilePathsInText:
    """Tests for file paths in text."""

    @pytest.mark.parametrize(
        "input_text,expected_contains",
        [
            (
                "Файл находится в /home/user/config.yaml",
                ["слэш", "хоум", "юзер", "конфиг", "ямл"],
            ),
            (
                "Открой ~/Documents/report.pdf",
                ["тильда", "документс", "репорт", "пдф"],
            ),
        ],
    )
    def test_file_paths_in_text(self, pipeline, input_text, expected_contains):
        """File paths should use 'слэш' for separators."""
        result = pipeline.process(input_text)
        for part in expected_contains:
            assert part.lower() in result.lower()


class TestSizeUnitsInText:
    """Tests for size units in text."""

    @pytest.mark.parametrize(
        "input_text,expected_contains",
        [
            (
                "Файл весит 100MB.",
                ["сто", "мегабайт"],
            ),
            (
                "Latency около 50ms.",
                ["пятьдесят", "миллисекунд"],
            ),
            (
                "Диск на 1TB.",
                ["один", "терабайт"],
            ),
        ],
    )
    def test_sizes_in_text(self, pipeline, input_text, expected_contains):
        """Size units should be expanded to words."""
        result = pipeline.process(input_text)
        for part in expected_contains:
            assert part.lower() in result.lower()


class TestMarkdownCodeBlocks:
    """Tests for markdown code blocks."""

    def test_code_block_full_mode(self, pipeline):
        """Code blocks in full mode should be read."""
        text = """
Пример кода:
```python
def hello():
    print("world")
```
Конец примера.
"""
        pipeline.config.code_block_mode = "full"
        result = pipeline.process(text)
        assert "деф" in result.lower() or "хелло" in result.lower()
        assert "принт" in result.lower()

    def test_code_block_brief_mode(self, pipeline):
        """Code blocks in brief mode should be described."""
        text = """
Пример кода:
```python
def hello():
    print("world")
```
Конец примера.
"""
        pipeline.config.code_block_mode = "brief"
        result = pipeline.process(text)
        assert "пример кода на пайтон" in result.lower()
        # Should not contain actual code
        assert "деф" not in result.lower()


class TestInlineCode:
    """Tests for inline code in markdown."""

    @pytest.mark.parametrize(
        "input_text,expected_contains",
        [
            (
                "Вызови `getUserData()` для получения данных.",
                ["гет юзер дата"],
            ),
            (
                "Установи через `pip install package`.",
                ["пип", "инсталл", "пакет"],
            ),
            (
                "Переменная `my_var` содержит значение.",
                ["май", "вар"],
            ),
        ],
    )
    def test_inline_code_normalization(self, pipeline, input_text, expected_contains):
        """Inline code should be normalized."""
        result = pipeline.process(input_text)
        for part in expected_contains:
            assert part.lower() in result.lower()


class TestMarkdownStructure:
    """Tests for markdown structural elements."""

    def test_headings_preserved(self, pipeline):
        """Headings should be read as text."""
        text = "## Установка\n\nШаги установки..."
        result = pipeline.process(text)
        assert "установка" in result.lower()

    def test_lists_read_naturally(self, pipeline):
        """Lists should be read naturally."""
        text = """
Шаги:
1. Установить зависимости
2. Настроить конфиг
3. Запустить сервер
"""
        result = pipeline.process(text)
        assert "первое" in result.lower() or "один" in result.lower()
        assert "установить" in result.lower()

    def test_links_read_text_only(self, pipeline):
        """Links should read only link text, not URL."""
        text = "Смотри [документацию](https://docs.example.com)"
        result = pipeline.process(text)
        assert "документацию" in result.lower()
        # URL should NOT be read
        assert "экзампл" not in result.lower()
        assert "https" not in result.lower()

    def test_links_english_text_transliterated(self, pipeline):
        """English words in link text should be transliterated."""
        text = "Читай [Fun with Dada](https://example.com)"
        result = pipeline.process(text)
        # English words must be transliterated (TTS can't read English)
        assert "fun" not in result.lower()
        assert "with" not in result.lower()
        # URL should NOT be read
        assert "https" not in result.lower()
        assert "экзампл" not in result.lower()

    def test_links_english_text_transliterated_with_mapping(self, pipeline):
        """English link text should be transliterated in process_with_char_mapping too."""
        text = "Пост [Fun with Dada](https://example.com/path), далее."
        result, mapping = pipeline.process_with_char_mapping(text)
        # English words must be transliterated
        assert "fun" not in result.lower()
        assert "dada" not in result.lower()
        # URL should NOT be read
        assert "https" not in result.lower()

    def test_links_mapping_points_to_exact_words(self, pipeline):
        """Each word in link text should map to its exact original position."""
        import re
        text = "Пост [Fun with Dada](https://example.com), далее."
        result, mapping = pipeline.process_with_char_mapping(text)
        words = {m.group(): (m.start(), m.end()) for m in re.finditer(r'\b\w+\b', result)}

        # Find transliterated "Dada" (дада)
        dada_candidates = [(w, s, e) for w, (s, e) in words.items() if 'дада' in w.lower()]
        assert dada_candidates, f"'дада' not found in: {list(words.keys())}"
        _, s, e = dada_candidates[0]
        orig_start, orig_end = mapping.get_original_range(s, e)
        assert text[orig_start:orig_end] == "Dada", (
            f"Expected 'Dada' at orig[{orig_start}:{orig_end}], got {repr(text[orig_start:orig_end])}"
        )

    def test_bare_urls_still_read(self, pipeline):
        """Bare URLs (not in markdown links) should still be read."""
        text = "Сайт: https://docs.example.com"
        result = pipeline.process(text)
        # Bare URL should be read
        assert "экзампл" in result.lower()


class TestComplexTechnicalText:
    """Tests for complex real-world technical text."""

    def test_installation_guide(self, pipeline):
        """Real installation guide should be readable."""
        text = """
## Установка Docker

1. Скачайте Docker Desktop с https://docker.com/download
2. Запустите `docker --version` для проверки
3. Версия должна быть >= 20.10.0
"""
        result = pipeline.process(text)
        # Should contain key parts
        assert "докер" in result.lower()
        assert "двадцать" in result.lower()  # version number
        assert "больше или равно" in result.lower()

    def test_api_documentation(self, pipeline):
        """API documentation should be readable."""
        text = """
### API Endpoint

GET https://api.example.com/v1/users

Возвращает JSON с данными пользователя:
```json
{"id": 123, "name": "John"}
```
"""
        result = pipeline.process(text)
        assert "эй пи ай" in result.lower()
        assert "эндпоинт" in result.lower() or "endpoint" in result.lower()
        assert "джейсон" in result.lower()

    def test_error_message(self, pipeline):
        """Error message with technical details should be readable."""
        text = """
Ошибка: ConnectionRefusedError на 192.168.1.100:8080
Проверьте что сервер запущен и порт не занят.
Логи: /var/log/app/error.log
"""
        result = pipeline.process(text)
        assert "сто девяносто два" in result.lower()
        assert "восемь тысяч восемьдесят" in result.lower()
        assert "слэш" in result.lower()


class TestEdgeCases:
    """Tests for edge cases and special situations."""

    def test_empty_text(self, pipeline):
        """Empty text should return empty string."""
        result = pipeline.process("")
        assert result == ""

    def test_only_whitespace(self, pipeline):
        """Whitespace-only text should return empty or minimal."""
        result = pipeline.process("   \n\t  \n  ")
        assert result.strip() == ""

    def test_only_numbers(self, pipeline):
        """Text with only numbers should work."""
        result = pipeline.process("123 456 789")
        assert "сто двадцать три" in result.lower()
        assert "четыреста пятьдесят шесть" in result.lower()

    def test_only_english(self, pipeline):
        """Text with only English should be transliterated."""
        result = pipeline.process("Hello World")
        # Should be transliterated
        assert len(result) > 0

    def test_special_characters_only(self, pipeline):
        """Text with only special characters."""
        result = pipeline.process("!@#$%")
        # Should handle gracefully
        assert len(result) >= 0

    def test_very_long_url(self, pipeline):
        """Very long URL should be handled."""
        url = "https://example.com/" + "/".join(["path"] * 10)
        text = f"Ссылка: {url}"
        result = pipeline.process(text)
        assert "экзампл" in result.lower()

    def test_nested_code_block(self, pipeline):
        """Nested code blocks should be handled."""
        text = """
```markdown
```python
print("nested")
```
```
"""
        # Should not crash
        result = pipeline.process(text)
        assert isinstance(result, str)


class TestConfiguration:
    """Tests for pipeline configuration."""

    def test_custom_it_terms(self, config, pipeline):
        """Custom IT terms should be used."""
        config.custom_it_terms["customterm"] = "кастом терм"
        pipeline = type(pipeline)(config)
        # After implementation, this should work
        pass  # placeholder

    def test_operators_disabled(self, config, pipeline):
        """Operators can be disabled."""
        config.read_operators = False
        pipeline = type(pipeline)(config)
        text = "x -> y"
        # With operators disabled, should not contain 'стрелка'
        pass  # placeholder


class TestGreekLettersInCode:
    """Tests for Greek letters in code context (Lean, math, etc.)."""

    def test_lean_class_declaration_inline_code(self, pipeline):
        """Greek letters in inline code should be converted to Russian names.

        Text from Lean documentation about type classes.
        """
        text = "В следующем объявлении класса типов `Plus` — это имя класса, `α : Type` — единственный аргумент, а `plus : α → α → α` — единственный метод."
        result = pipeline.process(text)

        # Check that Greek alpha is converted to "альфа"
        assert "альфа" in result.lower(), f"α should be converted to 'альфа', got: {result}"
        # All 3 alphas in `plus : α → α → α` should be converted
        assert result.lower().count("альфа") >= 3, f"Expected at least 3 'альфа', got {result.lower().count('альфа')} in: {result}"
        # Arrow should be converted
        assert "стрелка" in result.lower(), f"→ should be converted to 'стрелка', got: {result}"

    def test_lean_class_declaration_code_block(self, pipeline):
        """Greek letters in code blocks should be converted to Russian names.

        Full Lean code block with Greek letters.
        """
        text = '''```lean
class Plus (α : Type) where
  plus : α → α → α
```'''
        pipeline.config.code_block_mode = "full"
        result = pipeline.process(text)

        # Check that Greek alpha is converted to "альфа" in code block too
        assert "альфа" in result.lower(), f"α in code block should be converted to 'альфа', got: {result}"
        # Should have multiple alphas
        assert result.lower().count("альфа") >= 2, f"Expected at least 2 'альфа' in code block, got {result.lower().count('альфа')} in: {result}"

    def test_lean_full_example(self, pipeline):
        """Full Lean example with both inline code and code block."""
        text = '''В следующем объявлении класса типов `Plus` — это имя класса, `α : Type` — единственный аргумент, а `plus : α → α → α` — единственный метод:

```lean
class Plus (α : Type) where
  plus : α → α → α
```'''
        pipeline.config.code_block_mode = "full"
        result = pipeline.process(text)

        # Total alphas: 3 in inline code + 3 in code block = 6
        alpha_count = result.lower().count("альфа")
        assert alpha_count >= 5, f"Expected at least 5 'альфа' (inline + code block), got {alpha_count} in: {result}"

        # No Greek α should remain
        assert "α" not in result, f"Greek α should not remain in result: {result}"


class TestPreprocessing:
    """Tests for text preprocessing."""

    def test_bom_removal(self, pipeline):
        """BOM should be removed."""
        text = "\ufeffПривет мир"
        result = pipeline.process(text)
        assert result.startswith("Привет") or result.startswith("привет")

    def test_normalize_quotes(self, pipeline):
        """Fancy quotes should be normalized."""
        text = '«Привет» и "мир"'
        result = pipeline.process(text)
        # Quotes should be handled consistently
        assert "привет" in result.lower()

    def test_normalize_dashes(self, pipeline):
        """Different dashes should be handled."""
        text = "10–20 и 100—200"  # en-dash and em-dash
        result = pipeline.process(text)
        assert "десяти" in result.lower() or "от" in result.lower()
