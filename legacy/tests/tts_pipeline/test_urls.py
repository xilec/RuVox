"""Tests for URL/path normalizer.

Coverage: URLs, emails, IP addresses, file paths.
"""

import pytest


class TestURLNormalization:
    """Tests for URL normalization (full mode)."""

    @pytest.mark.parametrize(
        "url,expected",
        [
            # Simple URLs
            (
                "https://example.com",
                "эйч ти ти пи эс двоеточие слэш слэш example точка ком",
            ),
            (
                "http://test.org",
                "эйч ти ти пи двоеточие слэш слэш test точка орг",
            ),
            # With paths
            (
                "https://github.com/user/repo",
                "эйч ти ти пи эс двоеточие слэш слэш github точка ком слэш user слэш repo",
            ),
            (
                "https://docs.python.org/3.11/tutorial",
                "эйч ти ти пи эс двоеточие слэш слэш docs точка python точка орг слэш три точка одиннадцать слэш tutorial",
            ),
            # With file extension
            (
                "https://example.com/file.html",
                "эйч ти ти пи эс двоеточие слэш слэш example точка ком слэш file точка html",
            ),
            # Subdomains
            (
                "https://api.github.com/repos",
                "эйч ти ти пи эс двоеточие слэш слэш api точка github точка ком слэш repos",
            ),
            # With port
            (
                "http://localhost:8080",
                "эйч ти ти пи двоеточие слэш слэш localhost двоеточие восемь тысяч восемьдесят",
            ),
            (
                "http://localhost:3000/api",
                "эйч ти ти пи двоеточие слэш слэш localhost двоеточие три тысячи слэш api",
            ),
        ],
    )
    def test_url_full_mode(self, url_normalizer, url, expected):
        """URLs should be fully expanded in full mode."""
        result = url_normalizer.normalize_url(url)
        assert result == expected


class TestCommonTLDs:
    """Tests for common top-level domains."""

    @pytest.mark.parametrize(
        "url,expected_tld",
        [
            ("https://example.com", "ком"),
            ("https://example.org", "орг"),
            ("https://example.net", "нет"),
            ("https://example.ru", "ру"),
            ("https://example.io", "ай оу"),
            ("https://example.dev", "дев"),
            ("https://example.app", "апп"),
            ("https://example.ai", "эй ай"),
            ("https://example.co", "ко"),
            ("https://example.me", "ми"),
        ],
    )
    def test_tld_pronunciation(self, url_normalizer, url, expected_tld):
        """Common TLDs should have proper pronunciation."""
        result = url_normalizer.normalize_url(url)
        assert expected_tld in result


class TestProtocols:
    """Tests for protocol pronunciation."""

    @pytest.mark.parametrize(
        "url,expected_protocol",
        [
            ("https://example.com", "эйч ти ти пи эс"),
            ("http://example.com", "эйч ти ти пи"),
            ("ftp://files.example.com", "эф ти пи"),
            ("ssh://server.example.com", "эс эс эйч"),
            ("git://github.com/repo.git", "гит"),
            ("file:///home/user/doc.txt", "файл"),
        ],
    )
    def test_protocol_pronunciation(self, url_normalizer, url, expected_protocol):
        """Protocols should have proper pronunciation."""
        result = url_normalizer.normalize_url(url)
        assert result.startswith(expected_protocol)


class TestEmailNormalization:
    """Tests for email address normalization."""

    @pytest.mark.parametrize(
        "email,expected",
        [
            ("user@example.com", "user собака example точка ком"),
            ("test@mail.ru", "test собака mail точка ру"),
            ("john.doe@company.org", "john точка doe собака company точка орг"),
            ("admin@localhost", "admin собака localhost"),
            ("support@sub.domain.com", "support собака sub точка domain точка ком"),
            ("name_123@test.io", "name андерскор сто двадцать три собака test точка ай оу"),
            ("info-team@company.co", "info дефис team собака company точка ко"),
        ],
    )
    def test_email_normalization(self, url_normalizer, email, expected):
        """Emails should use 'собака' for @ and 'точка' for dots."""
        result = url_normalizer.normalize_email(email)
        assert result == expected


class TestIPAddressNormalization:
    """Tests for IP address normalization."""

    @pytest.mark.parametrize(
        "ip,expected",
        [
            # Read as numbers (default mode)
            (
                "192.168.1.1",
                "сто девяносто два точка сто шестьдесят восемь точка один точка один",
            ),
            (
                "127.0.0.1",
                "сто двадцать семь точка ноль точка ноль точка один",
            ),
            (
                "10.0.0.1",
                "десять точка ноль точка ноль точка один",
            ),
            (
                "255.255.255.0",
                "двести пятьдесят пять точка двести пятьдесят пять точка двести пятьдесят пять точка ноль",
            ),
            (
                "8.8.8.8",
                "восемь точка восемь точка восемь точка восемь",
            ),
            (
                "172.16.0.1",
                "сто семьдесят два точка шестнадцать точка ноль точка один",
            ),
        ],
    )
    def test_ip_as_numbers(self, url_normalizer, ip, expected):
        """IP addresses should read octets as numbers by default."""
        result = url_normalizer.normalize_ip(ip)
        assert result == expected


class TestIPAddressDigitMode:
    """Tests for IP address digit-by-digit mode."""

    @pytest.mark.parametrize(
        "ip,expected",
        [
            (
                "192.168.1.1",
                "один девять два точка один шесть восемь точка один точка один",
            ),
            (
                "127.0.0.1",
                "один два семь точка ноль точка ноль точка один",
            ),
        ],
    )
    def test_ip_as_digits(self, url_normalizer, ip, expected):
        """IP addresses can be read digit by digit."""
        # This will need config to switch modes
        pass  # placeholder for digit mode


class TestFilePathNormalization:
    """Tests for file path normalization."""

    @pytest.mark.parametrize(
        "path,expected",
        [
            # Unix paths
            (
                "/home/user/file.txt",
                "слэш home слэш user слэш file точка txt",
            ),
            (
                "/etc/nginx/nginx.conf",
                "слэш etc слэш nginx слэш nginx точка conf",
            ),
            (
                "/var/log/syslog",
                "слэш var слэш log слэш syslog",
            ),
            # Home directory
            (
                "~/Documents/report.pdf",
                "тильда слэш Documents слэш report точка pdf",
            ),
            (
                "~/.config/settings.json",
                "тильда слэш точка config слэш settings точка json",
            ),
            # Relative paths
            (
                "./src/main.py",
                "точка слэш src слэш main точка py",
            ),
            (
                "../config/app.yaml",
                "точка точка слэш config слэш app точка yaml",
            ),
            # Windows paths
            (
                "C:\\Users\\Admin\\file.txt",
                "си двоеточие бэкслэш Users бэкслэш Admin бэкслэш file точка txt",
            ),
            (
                "D:\\Projects\\code\\main.py",
                "ди двоеточие бэкслэш Projects бэкслэш code бэкслэш main точка py",
            ),
        ],
    )
    def test_filepath_normalization(self, url_normalizer, path, expected):
        """File paths should use 'слэш' or 'бэкслэш' appropriately."""
        result = url_normalizer.normalize_filepath(path)
        assert result == expected


class TestFileExtensions:
    """Tests for common file extensions."""

    @pytest.mark.parametrize(
        "filename,expected",
        [
            ("main.py", "main точка py"),
            ("index.js", "index точка js"),
            ("styles.css", "styles точка css"),
            ("config.yaml", "config точка yaml"),
            ("data.json", "data точка json"),
            ("README.md", "README точка md"),
            ("Dockerfile", "Dockerfile"),  # no extension
            ("docker-compose.yml", "docker дефис compose точка yml"),
            (".gitignore", "точка gitignore"),
            (".env", "точка env"),
            ("test.spec.ts", "test точка spec точка ts"),
        ],
    )
    def test_filename_with_extension(self, url_normalizer, filename, expected):
        """File extensions should be pronounced with 'точка'."""
        # Using filepath normalizer for consistency
        result = url_normalizer.normalize_filepath(filename)
        assert result == expected


class TestComplexURLs:
    """Tests for complex URLs with query params and fragments."""

    @pytest.mark.parametrize(
        "url,expected_contains",
        [
            # Query parameters
            (
                "https://example.com/search?q=test",
                ["example", "search", "q", "test"],
            ),
            # Fragments
            (
                "https://docs.example.com/guide#installation",
                ["docs", "guide", "installation"],
            ),
            # Multiple path segments
            (
                "https://api.example.com/v1/users/123/posts",
                ["api", "v1", "users", "posts"],
            ),
        ],
    )
    def test_complex_url_parts(self, url_normalizer, url, expected_contains):
        """Complex URLs should contain all significant parts."""
        result = url_normalizer.normalize_url(url)
        for part in expected_contains:
            assert part in result.lower() or part in result
