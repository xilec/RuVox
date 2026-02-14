# URLPathNormalizer

Нормализация URL, email, IP-адресов и файловых путей.

## Импорт

```python
from ruvox.tts_pipeline.normalizers import URLPathNormalizer
```

## Использование

```python
normalizer = URLPathNormalizer()

normalizer.normalize_url("https://github.com")
# → "эйч ти ти пи эс двоеточие слэш слэш github точка ком"

normalizer.normalize_email("user@mail.com")
# → "user собака mail точка ком"
```

## API

### normalize_url

```python
def normalize_url(self, url: str) -> str
```

Полное произношение URL.

```python
normalizer.normalize_url("https://github.com/user/repo")
# → "эйч ти ти пи эс двоеточие слэш слэш github точка ком слэш user слэш repo"

normalizer.normalize_url("http://localhost:8080/api")
# → "эйч ти ти пи двоеточие слэш слэш localhost двоеточие восемь тысяч восемьдесят слэш api"
```

**Компоненты:**
- Протокол: `https` → "эйч ти ти пи эс"
- Разделители: `://` → "двоеточие слэш слэш"
- Домен: `github.com` → "github точка ком"
- Порт: `:8080` → "двоеточие восемь тысяч восемьдесят"
- Путь: `/user/repo` → "слэш user слэш repo"

### normalize_email

```python
def normalize_email(self, email: str) -> str
```

Email с "собака" для @.

```python
normalizer.normalize_email("admin@example.com")
# → "admin собака example точка ком"

normalizer.normalize_email("user.name@company.ru")
# → "user точка name собака company точка ру"
```

### normalize_ip

```python
def normalize_ip(self, ip: str) -> str
```

IP-адреса с числами.

```python
normalizer.normalize_ip("192.168.1.1")
# → "сто девяносто два точка сто шестьдесят восемь точка один точка один"

normalizer.normalize_ip("8.8.8.8")
# → "восемь точка восемь точка восемь точка восемь"

normalizer.normalize_ip("127.0.0.1")
# → "сто двадцать семь точка ноль точка ноль точка один"
```

### normalize_filepath

```python
def normalize_filepath(self, path: str) -> str
```

Файловые пути.

```python
# Unix пути
normalizer.normalize_filepath("/home/user/config.yaml")
# → "слэш home слэш user слэш config точка yaml"

normalizer.normalize_filepath("~/Documents/file.txt")
# → "тильда слэш Documents слэш file точка txt"

# Windows пути
normalizer.normalize_filepath("C:\\Users\\Admin\\file.txt")
# → "си двоеточие бэкслэш Users бэкслэш Admin бэкслэш file точка txt"

# Относительные пути
normalizer.normalize_filepath("./src/main.py")
# → "точка слэш src слэш main точка py"
```

## Словари

### TLD (Top-Level Domains)

```python
TLD_MAP = {
    "com": "ком",
    "org": "орг",
    "net": "нет",
    "ru": "ру",
    "io": "ай оу",
    "dev": "дэв",
    "app": "апп",
}
```

### Протоколы

```python
PROTOCOLS = {
    "http": "эйч ти ти пи",
    "https": "эйч ти ти пи эс",
    "ftp": "эф ти пи",
    "ssh": "эс эс эйч",
    "git": "гит",
}
```

### Расширения файлов

```python
EXTENSIONS = {
    "py": "пай",
    "js": "джей эс",
    "ts": "ти эс",
    "json": "джейсон",
    "yaml": "ямл",
    "yml": "ямл",
    "md": "эм ди",
    "txt": "текст",
}
```

## Специальные символы

| Символ | Произношение |
|--------|--------------|
| `/` | слэш |
| `\` | бэкслэш |
| `.` | точка |
| `:` | двоеточие |
| `@` | собака |
| `~` | тильда |
| `?` | вопрос |
| `#` | решётка |
| `&` | амперсанд |

## Тестирование

```bash
uv run pytest tests/tts_pipeline/test_urls.py -v
```

62 теста покрывают:
- URL разных протоколов
- Email адреса
- IPv4 адреса
- Unix и Windows пути
- Относительные пути
- Query параметры и якоря
