"""Централизованная система логирования и обработки ошибок."""

import faulthandler
import functools
import logging
import sys
from logging.handlers import RotatingFileHandler
from pathlib import Path
from typing import Callable

# Глобальные переменные для хранения handlers
_original_qt_handler = None
_file_handler: RotatingFileHandler | None = None
_faulthandler_file = None


def setup_logging(log_dir: Path | None = None) -> None:
    """Инициализация системы логирования.

    Вызывать ДО создания QApplication.

    Args:
        log_dir: Директория для логов. По умолчанию ~/.cache/ruvox/logs/
    """
    if log_dir is None:
        log_dir = Path.home() / ".cache" / "ruvox" / "logs"

    log_dir.mkdir(parents=True, exist_ok=True)
    log_file = log_dir / "app.log"

    # Настройка корневого логгера
    root_logger = logging.getLogger("ruvox")
    root_logger.setLevel(logging.DEBUG)

    # Сокращаем имя модуля для читаемости
    class ShortNameFormatter(logging.Formatter):
        def format(self, record):
            # ruvox.ui.services.tts_worker -> ui.services.tts_worker
            if record.name.startswith("ruvox."):
                record.name = record.name[len("ruvox.") :]
            return super().format(record)

    short_formatter = ShortNameFormatter(
        "[%(asctime)s] [%(levelname)s] [%(name)s] %(message)s", datefmt="%Y-%m-%d %H:%M:%S"
    )

    global _file_handler

    # Handler для файла с ротацией
    _file_handler = RotatingFileHandler(
        log_file,
        maxBytes=1024 * 1024,  # 1MB
        backupCount=5,
        encoding="utf-8",
    )
    _file_handler.setLevel(logging.DEBUG)
    _file_handler.setFormatter(short_formatter)
    root_logger.addHandler(_file_handler)

    # Handler для stderr
    stderr_handler = logging.StreamHandler(sys.stderr)
    stderr_handler.setLevel(logging.DEBUG)
    stderr_handler.setFormatter(short_formatter)
    root_logger.addHandler(stderr_handler)

    global _faulthandler_file

    # Включаем faulthandler для segfaults
    _faulthandler_file = open(log_file, "a")
    faulthandler.enable(file=_faulthandler_file)

    # Устанавливаем перехватчик необработанных исключений
    sys.excepthook = _exception_hook

    # Логируем старт
    logger = logging.getLogger(__name__)
    logger.info(f"Логирование инициализировано. Файл: {log_file}")


def setup_qt_logging() -> None:
    """Настройка перехвата Qt сообщений.

    Вызывать ПОСЛЕ импорта PyQt6.
    """
    global _original_qt_handler

    try:
        from PyQt6.QtCore import QtMsgType, qInstallMessageHandler

        def qt_message_handler(msg_type, context, message):
            """Обработчик Qt сообщений."""
            logger = logging.getLogger("ruvox.qt")

            level_map = {
                QtMsgType.QtDebugMsg: logging.DEBUG,
                QtMsgType.QtInfoMsg: logging.INFO,
                QtMsgType.QtWarningMsg: logging.WARNING,
                QtMsgType.QtCriticalMsg: logging.ERROR,
                QtMsgType.QtFatalMsg: logging.CRITICAL,
            }

            level = level_map.get(msg_type, logging.WARNING)

            # Форматируем сообщение с контекстом
            location = ""
            if context.file:
                location = f" ({context.file}:{context.line})"

            logger.log(level, f"{message}{location}")

        qInstallMessageHandler(qt_message_handler)

        logger = logging.getLogger(__name__)
        logger.debug("Qt message handler установлен")

    except ImportError:
        pass


def _exception_hook(exc_type, exc_value, exc_traceback):
    """Перехватчик необработанных исключений Python."""
    # Игнорируем KeyboardInterrupt
    if issubclass(exc_type, KeyboardInterrupt):
        sys.__excepthook__(exc_type, exc_value, exc_traceback)
        return

    logger = logging.getLogger("ruvox")
    logger.critical("Необработанное исключение", exc_info=(exc_type, exc_value, exc_traceback))


def safe_slot(func: Callable) -> Callable:
    """Декоратор для безопасного выполнения Qt слотов.

    Оборачивает слот в try-except и логирует ошибки.

    Использование:
        @safe_slot
        def on_button_clicked(self):
            ...
    """

    @functools.wraps(func)
    def wrapper(*args, **kwargs):
        try:
            return func(*args, **kwargs)
        except Exception as e:
            logger = logging.getLogger("ruvox.qt.slots")
            logger.error(f"Ошибка в слоте {func.__qualname__}: {e}", exc_info=True)

    return wrapper


def get_log_file_path() -> Path:
    """Возвращает путь к текущему файлу логов."""
    return Path.home() / ".cache" / "ruvox" / "logs" / "app.log"


def shutdown_logging() -> None:
    """Корректное завершение системы логирования.

    Закрывает все handlers и освобождает ресурсы.
    Вызывать при завершении приложения.
    """
    global _file_handler, _faulthandler_file

    logger = logging.getLogger(__name__)
    logger.info("Завершение системы логирования")

    # Отключаем faulthandler и закрываем его файл
    faulthandler.disable()
    if _faulthandler_file is not None:
        try:
            _faulthandler_file.close()
        except Exception:
            pass
        _faulthandler_file = None

    # Закрываем file handler
    if _file_handler is not None:
        try:
            root_logger = logging.getLogger("ruvox")
            root_logger.removeHandler(_file_handler)
            _file_handler.close()
        except Exception:
            pass
        _file_handler = None
