#!/usr/bin/env python3
"""Интеграционный тест с РЕАЛЬНЫМИ компонентами приложения.

Исправлена проблема с зависанием - Qt multimedia создаётся после app.exec().
"""

import sys
import signal
import tempfile
from pathlib import Path
import shutil

sys.path.insert(0, 'src')

import logging
logging.basicConfig(level=logging.DEBUG, format='%(name)s: %(message)s')
print("=== STARTING TEST ===", flush=True)

from PyQt6.QtWidgets import QApplication
from PyQt6.QtCore import QTimer

import mpv

# Импортируем РЕАЛЬНЫЕ компоненты из приложения
from ruvox.ui.models.config import UIConfig
from ruvox.ui.models.entry import TextEntry, EntryStatus
from ruvox.ui.services.storage import StorageService
from ruvox.ui.services.tts_worker import TTSWorker

print("All imports done", flush=True)


class IntegrationTestRealComponents:
    """Интеграционный тест с реальными компонентами."""

    def __init__(self):
        print("  __init__: Creating QApplication...", flush=True)
        self.app = QApplication(sys.argv)

        print("  __init__: Creating temp dir...", flush=True)
        self.temp_dir = Path(tempfile.mkdtemp(prefix="tts_test_"))

        print("  __init__: Creating UIConfig...", flush=True)
        self.config = UIConfig(cache_dir=self.temp_dir)

        print("  __init__: Creating StorageService...", flush=True)
        self.storage = StorageService(self.config)

        # mpv will be created in _setup_media()
        self.player = None  # mpv.MPV instance

        print("  __init__: Creating TTSWorker...", flush=True)
        self.tts_worker = TTSWorker(self.config, self.storage)

        self.test_text = "Окно теперь показывается (visible: True). Есть предупреждение о медиа-бэкенде, но окно работает."
        self.entry = None
        self.success = False
        self.error_msg = None

        print("  __init__: Creating timeout timer...", flush=True)
        self.timeout_timer = QTimer()
        self.timeout_timer.setSingleShot(True)
        self.timeout_timer.timeout.connect(self._on_timeout)
        print("  __init__: Done", flush=True)

    def run(self) -> bool:
        print("=" * 60, flush=True)
        print("Интеграционный тест: РЕАЛЬНЫЕ КОМПОНЕНТЫ", flush=True)
        print("(UIConfig, StorageService, TTSWorker)", flush=True)
        print("=" * 60, flush=True)
        print(f"Cache dir: {self.temp_dir}", flush=True)

        # Подключаем сигналы TTS worker
        self.tts_worker.model_loaded.connect(self._on_model_loaded)
        self.tts_worker.model_error.connect(self._on_model_error)
        self.tts_worker.completed.connect(self._on_tts_completed)
        self.tts_worker.error.connect(self._on_tts_error)

        self.timeout_timer.start(180000)  # 3 мин

        # Создаём media компоненты через QTimer (после app.exec())
        QTimer.singleShot(100, self._setup_media)

        self.app.exec()

        # Cleanup
        shutil.rmtree(self.temp_dir, ignore_errors=True)

        print("\n" + "=" * 60, flush=True)
        if self.success:
            print("ТЕСТ ПРОЙДЕН!", flush=True)
        else:
            print(f"ТЕСТ ПРОВАЛЕН: {self.error_msg}", flush=True)
        print("=" * 60, flush=True)

        return self.success

    def _setup_media(self):
        """Создание mpv ПОСЛЕ запуска event loop."""
        print("\n[Setup] Создание mpv...", flush=True)
        try:
            self.player = mpv.MPV(video=False, ytdl=False)
            self.player.volume = 0  # Тихий режим
            print("   mpv player ready", flush=True)

            QTimer.singleShot(100, self._step1_create_entry)
        except Exception as e:
            self._fail(f"Setup media: {e}")

    def _fail(self, msg: str):
        self.error_msg = msg
        print(f"\n❌ ОШИБКА: {msg}", flush=True)
        self.app.quit()

    def _on_timeout(self):
        self._fail("Таймаут теста")

    def _step1_create_entry(self):
        print(f"\n[Шаг 1] Создание entry: '{self.test_text[:40]}...'", flush=True)
        try:
            self.entry = self.storage.add_entry(self.test_text)
            print(f"   ✓ Entry создан: {self.entry.id[:8]}...", flush=True)
            QTimer.singleShot(100, self._step2_first_tts)
        except Exception as e:
            self._fail(f"Create entry: {e}")

    def _step2_first_tts(self):
        print("\n[Шаг 2] Первая генерация через TTSWorker...", flush=True)
        self.tts_worker.process(self.entry, play_when_ready=False)

    def _on_model_loaded(self):
        print("   [TTSWorker] Модель загружена", flush=True)

    def _on_model_error(self, error):
        self._fail(f"Model error: {error}")

    def _on_tts_completed(self, entry_id):
        print(f"   ✓ TTS завершён: {entry_id[:8]}...", flush=True)

        # Обновляем entry из storage
        self.entry = self.storage.get_entry(entry_id)

        if self.entry.status == EntryStatus.READY:
            if not hasattr(self, '_regenerated'):
                # Первая генерация - переходим к воспроизведению
                QTimer.singleShot(100, self._step3_play)
            else:
                # Перегенерация успешна!
                print("\n   ✓✓✓ ПЕРЕГЕНЕРАЦИЯ УСПЕШНА! ✓✓✓", flush=True)
                self.success = True
                self.timeout_timer.stop()
                QTimer.singleShot(500, self.app.quit)

    def _on_tts_error(self, entry_id, error):
        self._fail(f"TTS error ({entry_id[:8]}): {error}")

    def _step3_play(self):
        print("\n[Шаг 3] Воспроизведение...", flush=True)

        audio_path = self.storage.get_audio_path(self.entry.id)
        if not audio_path or not audio_path.exists():
            self._fail("Audio file not found")
            return

        print(f"   Audio: {audio_path}", flush=True)
        self.player.loadfile(str(audio_path))
        self.player.wait_until_playing()
        print("   Playing...", flush=True)

        QTimer.singleShot(2000, self._step4_stop)

    def _step4_stop(self):
        print("\n[Шаг 4] Остановка и подготовка к перегенерации...", flush=True)

        self.player.command("stop")
        print("   Плеер остановлен", flush=True)

        # Удаляем аудио как в реальном UI
        self.storage.delete_audio(self.entry.id)
        print("   Аудио удалено", flush=True)

        # Получаем обновлённый entry
        self.entry = self.storage.get_entry(self.entry.id)
        self.entry.was_regenerated = True
        self.storage.update_entry(self.entry)

        QTimer.singleShot(500, self._step5_regenerate)

    def _step5_regenerate(self):
        print("\n[Шаг 5] ПЕРЕГЕНЕРАЦИЯ через TTSWorker...", flush=True)
        print("   >>> КРИТИЧЕСКИЙ МОМЕНТ <<<", flush=True)

        self._regenerated = True
        self.tts_worker.process(self.entry, play_when_ready=False)


def main():
    print("main() started", flush=True)
    signal.signal(signal.SIGINT, signal.SIG_DFL)

    print("Creating test instance...", flush=True)
    test = IntegrationTestRealComponents()
    print("Test instance created, calling run()...", flush=True)
    success = test.run()

    return 0 if success else 1


if __name__ == "__main__":
    print("__main__ starting...", flush=True)
    sys.exit(main())
