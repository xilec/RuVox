# TTS Pipeline

Pipeline предобработки текста для Silero TTS. Преобразует технический текст в форму, пригодную для качественного озвучивания.

## Назначение

Silero TTS не умеет корректно произносить:
- Английские слова → молчание или искажение
- Аббревиатуры → неправильное произношение
- Числа, даты, URL → читает посимвольно
- Операторы и спецсимволы → пропускает

Pipeline решает эти проблемы через нормализацию.

## Быстрый старт

```python
from ruvox.tts_pipeline import TTSPipeline

pipeline = TTSPipeline()

# Простое использование
text = "Вызови getUserData() через API"
result = pipeline.process(text)
# → "Вызови гет юзер дата через эй пи ай"

# С маппингом позиций (для подсветки слов)
result, mapping = pipeline.process_with_char_mapping(text)
# mapping позволяет отобразить позицию в result на позицию в text
```

## Что обрабатывается

| Тип | Пример | Результат |
|-----|--------|-----------|
| Английские слова | `feature` | "фича" |
| Аббревиатуры | `API` | "эй пи ай" |
| Числа | `123` | "сто двадцать три" |
| Проценты | `50%` | "пятьдесят процентов" |
| URL | `https://example.com` | "эйч ти ти пи эс..." |
| Email | `user@mail.com` | "user собака mail точка ком" |
| IP-адреса | `192.168.1.1` | "сто девяносто два точка..." |
| Пути | `/home/user` | "слэш home слэш user" |
| Версии | `v2.3.1` | "два точка три точка один" |
| Операторы | `>=` | "больше или равно" |
| camelCase | `getUserData` | "гет юзер дата" |
| snake_case | `get_user` | "гет юзер" |

## Структура модуля

```
tts_pipeline/
├── __init__.py          # Экспорты
├── config.py            # PipelineConfig
├── pipeline.py          # TTSPipeline
├── tracked_text.py      # TrackedText, CharMapping
├── word_mapping.py      # WordMapping (эвристический)
└── normalizers/
    ├── english.py       # EnglishNormalizer
    ├── abbreviations.py # AbbreviationNormalizer
    ├── numbers.py       # NumberNormalizer
    ├── urls.py          # URLPathNormalizer
    ├── symbols.py       # SymbolNormalizer
    └── code.py          # CodeIdentifierNormalizer
```

## Разделы документации

- [Архитектура](architecture.md) — этапы обработки, приоритеты
- [TTSPipeline](pipeline.md) — основной класс
- [TrackedText](tracked-text.md) — отслеживание позиций
- [Нормализаторы](normalizers/index.md) — отдельные компоненты

## Зависимости

```
num2words>=0.5.12    # Числа в слова (русский)
```

Опционально:
```
g2p-en>=2.1.0        # Grapheme-to-phoneme для неизвестных слов
```
