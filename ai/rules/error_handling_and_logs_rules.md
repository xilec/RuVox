# Правила логирования и обработки ошибок

## Библиотека

Стандартный модуль `logging` Python.

## Настройка логгера в модулях

```python
import logging
logger = logging.getLogger(__name__)
```

## Формат сообщений

```
[2024-01-15 14:30:22] [LEVEL] [module.name] Message
```

## Вывод логов

- **Файл:** `~/.cache/fast-tts-rus/logs/app.log`
- **Ротация:** 5 файлов по 1MB
- **stderr:** дублирование для отладки

## Уровни логирования

| Уровень | Когда использовать |
|---------|-------------------|
| DEBUG | Детальная отладка |
| INFO | Основные события (старт, завершение операций) |
| WARNING | Некритичные проблемы (можно продолжить работу) |
| ERROR | Ошибки с возможностью восстановления |
| CRITICAL | Фатальные ошибки (приложение не может работать) |

## Перехват ошибок

| Источник | Механизм |
|----------|----------|
| Python исключения | `sys.excepthook` → логирует traceback |
| Qt слоты | Декоратор `@safe_slot` → try-except + лог |
| QRunnable.run() | try-except внутри → лог + сигнал error |
| Segfaults (C++) | `faulthandler.enable()` → traceback в лог |
| Qt messages | `qInstallMessageHandler` → лог (не блокирует) |

## Реализация

**Файл:** `src/fast_tts_rus/ui/services/logging_service.py`

- `setup_logging()` — инициализация всех обработчиков
- `@safe_slot` — декоратор для Qt слотов

**Инициализация в main.py:**

```python
from fast_tts_rus.ui.services.logging_service import setup_logging
setup_logging()  # Вызвать ДО создания QApplication
```
