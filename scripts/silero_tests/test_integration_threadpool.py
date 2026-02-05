#!/usr/bin/env python3
"""Интеграционный тест: с QThreadPool как в реальном UI."""

import sys
import signal
import tempfile
from pathlib import Path

sys.path.insert(0, 'src')

from PyQt6.QtWidgets import (
    QApplication, QMainWindow, QWidget, QVBoxLayout,
    QPushButton, QLabel, QSystemTrayIcon, QMenu
)
from PyQt6.QtCore import QTimer, QUrl, QObject, pyqtSignal, QRunnable, QThreadPool
from PyQt6.QtGui import QIcon, QAction
from PyQt6.QtMultimedia import QMediaPlayer, QAudioOutput
from PyQt6.QtDBus import QDBusConnection

import torch
from fast_tts_rus.tts_pipeline import TTSPipeline


# Глобальная модель (как в реальном UI)
g_model = None


class ModelLoadSignals(QObject):
    loaded = pyqtSignal(object)
    error = pyqtSignal(str)


class ModelLoadRunnable(QRunnable):
    """Загрузка модели в отдельном потоке (как в UI)."""

    def __init__(self):
        super().__init__()
        self.signals = ModelLoadSignals()

    def run(self):
        global g_model
        try:
            print("   [Thread] Загрузка модели V5...")
            g_model, _ = torch.hub.load(
                repo_or_dir='snakers4/silero-models',
                model='silero_tts',
                language='ru',
                speaker='v5_ru'
            )
            print(f"   [Thread] Модель загружена: {g_model.speakers}")
            self.signals.loaded.emit(g_model)
        except Exception as e:
            print(f"   [Thread] ОШИБКА: {e}")
            self.signals.error.emit(str(e))


class TTSSignals(QObject):
    completed = pyqtSignal(str)  # audio_path
    error = pyqtSignal(str)


class TTSRunnable(QRunnable):
    """TTS в отдельном потоке (как в UI)."""

    def __init__(self, text: str, output_path: str):
        super().__init__()
        self.text = text
        self.output_path = output_path
        self.signals = TTSSignals()

    def run(self):
        global g_model
        try:
            print(f"   [Thread] TTS: '{self.text[:30]}...'")

            pipeline = TTSPipeline()
            normalized = pipeline.process(self.text)
            print(f"   [Thread] Нормализовано: '{normalized[:30]}...'")

            print(f"   [Thread] Model speakers: {g_model.speakers}")
            print("   [Thread] Вызов apply_tts...")

            with torch.no_grad():
                audio = g_model.apply_tts(
                    text=normalized,
                    speaker='xenia',
                    sample_rate=48000
                )

            if isinstance(audio, torch.Tensor):
                audio_np = audio.numpy()
            else:
                audio_np = audio

            from scipy.io import wavfile
            wavfile.write(self.output_path, 48000, audio_np)

            duration = len(audio_np) / 48000
            print(f"   [Thread] ✓ Готово: {duration:.2f}s")
            self.signals.completed.emit(self.output_path)

        except Exception as e:
            import traceback
            traceback.print_exc()
            print(f"   [Thread] ОШИБКА: {e}")
            self.signals.error.emit(str(e))


class TestMainWindow(QMainWindow):
    def __init__(self):
        super().__init__()
        self.setWindowTitle("Integration Test - ThreadPool")
        self.setMinimumSize(400, 300)

        central = QWidget()
        self.setCentralWidget(central)
        layout = QVBoxLayout(central)

        self.status_label = QLabel("Инициализация...")
        layout.addWidget(self.status_label)


class IntegrationTestThreadPool:
    """Интеграционный тест с QThreadPool."""

    def __init__(self):
        self.app = QApplication(sys.argv)
        self.temp_dir = tempfile.mkdtemp(prefix="tts_test_")
        # Точно такой же текст как в реальном UI
        self.test_text = "Окно теперь показывается (visible: True). Есть предупреждение о медиа-бэкенде, но окно работает."
        self.audio_path = f"{self.temp_dir}/test.wav"
        self.success = False
        self.error_msg = None

        # UI
        self.main_window = None
        self.tray_icon = None
        self.player = None
        self.audio_output = None

        # ThreadPool (как в реальном UI)
        self.thread_pool = QThreadPool()

        # Timeout
        self.timeout_timer = QTimer()
        self.timeout_timer.setSingleShot(True)
        self.timeout_timer.timeout.connect(self._on_timeout)

    def run(self) -> bool:
        print("=" * 60)
        print("Интеграционный тест: С QThreadPool")
        print("(Модель и TTS в отдельных потоках, как в реальном UI)")
        print("=" * 60)
        print(f"Temp dir: {self.temp_dir}")
        print(f"ThreadPool max threads: {self.thread_pool.maxThreadCount()}")

        self.timeout_timer.start(120000)  # 2 мин

        QTimer.singleShot(100, self._step1_setup_ui)

        self.app.exec()

        # Cleanup
        if Path(self.audio_path).exists():
            Path(self.audio_path).unlink()
        try:
            Path(self.temp_dir).rmdir()
        except:
            pass

        print("\n" + "=" * 60)
        if self.success:
            print("ТЕСТ ПРОЙДЕН!")
        else:
            print(f"ТЕСТ ПРОВАЛЕН: {self.error_msg}")
        print("=" * 60)

        return self.success

    def _fail(self, msg: str):
        self.error_msg = msg
        print(f"\n❌ ОШИБКА: {msg}")
        self.app.quit()

    def _on_timeout(self):
        self._fail("Таймаут теста")

    def _step1_setup_ui(self):
        print("\n[Шаг 1] Создание UI...")
        try:
            self.main_window = TestMainWindow()
            self.main_window.show()

            self.tray_icon = QSystemTrayIcon()
            self.tray_icon.setIcon(QIcon.fromTheme("audio-volume-high"))
            self.tray_icon.show()

            self.player = QMediaPlayer()
            self.audio_output = QAudioOutput()
            self.audio_output.setVolume(0.0)  # Тихий режим для тестов
            self.player.setAudioOutput(self.audio_output)
            self.player.mediaStatusChanged.connect(self._on_media_status)

            print("   ✓ UI создан")
            QTimer.singleShot(100, self._step2_load_model)
        except Exception as e:
            self._fail(f"UI: {e}")

    def _on_media_status(self, status):
        print(f"   [Player] {status}")

    def _step2_load_model(self):
        print("\n[Шаг 2] Загрузка модели в QThreadPool...")
        self.main_window.status_label.setText("Загрузка модели...")

        runnable = ModelLoadRunnable()
        runnable.signals.loaded.connect(self._on_model_loaded)
        runnable.signals.error.connect(lambda e: self._fail(f"Model: {e}"))
        self.thread_pool.start(runnable)

    def _on_model_loaded(self, model):
        print("   ✓ Модель загружена")
        QTimer.singleShot(100, self._step3_first_tts)

    def _step3_first_tts(self):
        print(f"\n[Шаг 3] Первая генерация в QThreadPool...")
        self.main_window.status_label.setText("Первая генерация...")

        runnable = TTSRunnable(self.test_text, self.audio_path)
        runnable.signals.completed.connect(self._on_first_tts_done)
        runnable.signals.error.connect(lambda e: self._fail(f"TTS1: {e}"))
        self.thread_pool.start(runnable)

    def _on_first_tts_done(self, path):
        print(f"   ✓ Первая генерация готова: {path}")
        QTimer.singleShot(100, self._step4_play)

    def _step4_play(self):
        print("\n[Шаг 4] Воспроизведение...")
        self.main_window.status_label.setText("Воспроизведение...")

        self.player.setSource(QUrl.fromLocalFile(self.audio_path))
        self.player.play()

        QTimer.singleShot(2000, self._step5_stop)

    def _step5_stop(self):
        print("\n[Шаг 5] Остановка...")
        self.main_window.status_label.setText("Остановка...")

        self.player.stop()
        if Path(self.audio_path).exists():
            Path(self.audio_path).unlink()
        print("   ✓ Остановлен, файл удалён")

        QTimer.singleShot(500, self._step6_regenerate)

    def _step6_regenerate(self):
        print(f"\n[Шаг 6] ПЕРЕГЕНЕРАЦИЯ в QThreadPool...")
        print("   >>> КРИТИЧЕСКИЙ МОМЕНТ <<<")
        self.main_window.status_label.setText("ПЕРЕГЕНЕРАЦИЯ...")

        runnable = TTSRunnable(self.test_text, self.audio_path)
        runnable.signals.completed.connect(self._on_regen_done)
        runnable.signals.error.connect(lambda e: self._fail(f"Regen: {e}"))
        self.thread_pool.start(runnable)

    def _on_regen_done(self, path):
        print(f"   ✓ Перегенерация успешна: {path}")

        self.main_window.status_label.setText("УСПЕХ!")
        self.success = True
        self.timeout_timer.stop()

        QTimer.singleShot(1000, self.app.quit)


def main():
    signal.signal(signal.SIGINT, signal.SIG_DFL)

    test = IntegrationTestThreadPool()
    success = test.run()

    return 0 if success else 1


if __name__ == "__main__":
    sys.exit(main())
