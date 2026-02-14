# NumberNormalizer

Преобразование чисел и числовых форматов в русские слова.

## Импорт

```python
from ruvox.tts_pipeline.normalizers import NumberNormalizer
```

## Использование

```python
normalizer = NumberNormalizer()

normalizer.normalize_number("123")      # → "сто двадцать три"
normalizer.normalize_percentage("50%")  # → "пятьдесят процентов"
normalizer.normalize_version("v2.3.1")  # → "два точка три точка один"
```

## API

### normalize_number

```python
def normalize_number(self, num_str: str) -> str
```

Целые числа.

```python
normalizer.normalize_number("0")      # → "ноль"
normalizer.normalize_number("42")     # → "сорок два"
normalizer.normalize_number("1000")   # → "тысяча"
normalizer.normalize_number("12345")  # → "двенадцать тысяч триста сорок пять"
```

### normalize_float

```python
def normalize_float(self, float_str: str) -> str
```

Дробные числа.

```python
normalizer.normalize_float("3.14")    # → "три точка один четыре"
normalizer.normalize_float("0.5")     # → "ноль точка пять"
normalizer.normalize_float("100.00")  # → "сто точка ноль ноль"
```

### normalize_percentage

```python
def normalize_percentage(self, pct_str: str) -> str
```

Проценты с правильным склонением.

```python
normalizer.normalize_percentage("1%")    # → "один процент"
normalizer.normalize_percentage("2%")    # → "два процента"
normalizer.normalize_percentage("5%")    # → "пять процентов"
normalizer.normalize_percentage("21%")   # → "двадцать один процент"
normalizer.normalize_percentage("50.5%") # → "пятьдесят точка пять процентов"
```

### normalize_range

```python
def normalize_range(self, range_str: str) -> str
```

Диапазоны чисел.

```python
normalizer.normalize_range("10-20")   # → "от десяти до двадцати"
normalizer.normalize_range("1-100")   # → "от одного до ста"
normalizer.normalize_range("0-5")     # → "от нуля до пяти"
```

### normalize_size

```python
def normalize_size(self, size_str: str) -> str
```

Размеры с единицами измерения.

```python
normalizer.normalize_size("100MB")    # → "сто мегабайт"
normalizer.normalize_size("1GB")      # → "один гигабайт"
normalizer.normalize_size("50ms")     # → "пятьдесят миллисекунд"
normalizer.normalize_size("10px")     # → "десять пикселей"
```

**Поддерживаемые единицы:**

| Единица | Произношение |
|---------|--------------|
| KB | килобайт |
| MB | мегабайт |
| GB | гигабайт |
| TB | терабайт |
| ms | миллисекунд |
| sec | секунд |
| min | минут |
| px | пикселей |
| em | эм |
| rem | рэм |

### normalize_version

```python
def normalize_version(self, ver_str: str) -> str
```

Версии программ.

```python
normalizer.normalize_version("v1.0")      # → "один точка ноль"
normalizer.normalize_version("2.3.1")     # → "два точка три точка один"
normalizer.normalize_version("v10.15.7")  # → "десять точка пятнадцать точка семь"
```

### normalize_date

```python
def normalize_date(self, date_str: str) -> str
```

Даты.

```python
# ISO формат
normalizer.normalize_date("2024-01-15")
# → "пятнадцатое января две тысячи двадцать четвёртого года"

# Европейский формат
normalizer.normalize_date("15.01.2024")
# → "пятнадцатое января две тысячи двадцать четвёртого года"
```

### normalize_time

```python
def normalize_time(self, time_str: str) -> str
```

Время.

```python
normalizer.normalize_time("14:30")  # → "четырнадцать часов тридцать минут"
normalizer.normalize_time("9:05")   # → "девять часов пять минут"
normalizer.normalize_time("0:00")   # → "ноль часов ноль минут"
```

## Склонение

Правильное склонение для русского языка:

```python
# Проценты
"1 процент", "2 процента", "5 процентов", "21 процент"

# Единицы
"1 мегабайт", "2 мегабайта", "5 мегабайт"
"1 секунда", "2 секунды", "5 секунд"
```

## Зависимости

Используется библиотека `num2words`:

```python
from num2words import num2words

num2words(42, lang='ru')  # → "сорок два"
num2words(42, lang='ru', to='ordinal')  # → "сорок второй"
```

## Тестирование

```bash
uv run pytest tests/tts_pipeline/test_numbers.py -v
```

108 тестов покрывают:
- Целые числа (0 до миллиардов)
- Дробные числа
- Проценты со склонением
- Диапазоны
- Размеры с единицами
- Версии
- Даты и время
