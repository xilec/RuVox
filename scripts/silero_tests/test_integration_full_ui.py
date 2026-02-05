#!/usr/bin/env python3
"""Интеграционный тест: полный UI с QMainWindow, tray, hotkeys."""

import sys
import signal
import tempfile
from pathlib import Path

sys.path.insert(0, 'src')

from PyQt6.QtWidgets import (
    QApplication, QMainWindow, QWidget, QVBoxLayout,
    QPushButton, QLabel, QSystemTrayIcon, QMenu
)
from PyQt6.QtCore import QTimer, QUrl, QObject, pyqtSignal
from PyQt6.QtGui import QIcon, QAction
from PyQt6.QtMultimedia import QMediaPlayer, QAudioOutput
from PyQt6.QtDBus import QDBusConnection

import torch
from fast_tts_rus.tts_pipeline import TTSPipeline


class TestMainWindow(QMainWindow):
    """Тестовое главное окно, имитирующее реальный UI."""

    def __init__(self):
        super().__init__()
        self.setWindowTitle("Integration Test")
        self.setMinimumSize(400, 300)

        central = QWidget()
        self.setCentralWidget(central)
        layout = QVBoxLayout(central)

        self.status_label = QLabel("Инициализация...")
        layout.addWidget(self.status_label)

        self.btn_test = QPushButton("Запустить тест")
        layout.addWidget(self.btn_test)


class TestTrayIcon(QSystemTrayIcon):
    """Тестовый системный трей."""

    def __init__(self, parent=None):
        super().__init__(parent)
        # Создаём простую иконку
        self.setIcon(QIcon.fromTheme("audio-volume-high"))

        menu = QMenu()
        menu.addAction(QAction("Test Action", menu))
        self.setContextMenu(menu)


class IntegrationTestFullUI:
    """Интеграционный тест с полным UI."""

    def __init__(self):
        self.app = QApplication(sys.argv)
        self.temp_dir = tempfile.mkdtemp(prefix="tts_test_")
        self.model = None
        self.player = None
        self.audio_output = None
        self.test_text = "Тестовый текст для проверки перегенерации."
        self.audio_path = None
        self.success = False
        self.error_msg = None

        # UI компоненты
        self.main_window = None
        self.tray_icon = None
        self.dbus_connection = None

        # Timeout
        self.timeout_timer = QTimer()
        self.timeout_timer.setSingleShot(True)
        self.timeout_timer.timeout.connect(self._on_timeout)

    def run(self) -> bool:
        """Запуск теста."""
        print("=" * 60)
        print("Интеграционный тест: ПОЛНЫЙ UI")
        print("(QMainWindow + SystemTray + DBus + QMediaPlayer)")
        print("=" * 60)
        print(f"Temp dir: {self.temp_dir}")

        self.timeout_timer.start(90000)  # 90 сек

        QTimer.singleShot(100, self._step1_setup_ui)

        self.app.exec()

        # Cleanup
        if self.audio_path and Path(self.audio_path).exists():
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
        self._fail("Таймаут теста (90 сек)")

    def _step1_setup_ui(self):
        """Шаг 1: Создание UI компонентов."""
        print("\n[Шаг 1] Создание UI компонентов...")
        try:
            # QMainWindow
            self.main_window = TestMainWindow()
            self.main_window.show()
            print("   ✓ QMainWindow создан")

            # System Tray
            self.tray_icon = TestTrayIcon()
            self.tray_icon.show()
            print("   ✓ QSystemTrayIcon создан")

            # DBus connection (как в HotkeyService)
            self.dbus_connection = QDBusConnection.sessionBus()
            if self.dbus_connection.isConnected():
                print("   ✓ D-Bus session bus подключен")
            else:
                print("   ⚠ D-Bus session bus НЕ доступен")

            QTimer.singleShot(500, self._step2_load_model)
        except Exception as e:
            import traceback
            traceback.print_exc()
            self._fail(f"Ошибка создания UI: {e}")

    def _step2_load_model(self):
        """Шаг 2: Загрузка модели."""
        print("\n[Шаг 2] Загрузка модели Silero V5...")
        self.main_window.status_label.setText("Загрузка модели...")
        try:
            self.model, _ = torch.hub.load(
                repo_or_dir='snakers4/silero-models',
                model='silero_tts',
                language='ru',
                speaker='v5_ru'
            )
            print(f"   ✓ Модель загружена, speakers: {self.model.speakers}")
            QTimer.singleShot(100, self._step3_first_generation)
        except Exception as e:
            self._fail(f"Не удалось загрузить модель: {e}")

    def _step3_first_generation(self):
        """Шаг 3: Первая генерация."""
        print(f"\n[Шаг 3] Первая генерация: '{self.test_text}'")
        self.main_window.status_label.setText("Первая генерация...")
        try:
            pipeline = TTSPipeline()
            normalized = pipeline.process(self.test_text)
            print(f"   Нормализовано: '{normalized}'")

            with torch.no_grad():
                audio = self.model.apply_tts(
                    text=normalized,
                    speaker='xenia',
                    sample_rate=48000
                )

            if isinstance(audio, torch.Tensor):
                audio_np = audio.numpy()
            else:
                audio_np = audio

            from scipy.io import wavfile
            self.audio_path = f"{self.temp_dir}/test.wav"
            wavfile.write(self.audio_path, 48000, audio_np)

            duration = len(audio_np) / 48000
            print(f"   ✓ Аудио: {duration:.2f}s")

            QTimer.singleShot(100, self._step4_setup_player)
        except Exception as e:
            import traceback
            traceback.print_exc()
            self._fail(f"Первая генерация: {e}")

    def _step4_setup_player(self):
        """Шаг 4: Настройка плеера."""
        print("\n[Шаг 4] Настройка QMediaPlayer...")
        try:
            self.player = QMediaPlayer()
            self.audio_output = QAudioOutput()
            self.audio_output.setVolume(0.0)  # Тихий режим для тестов
            self.player.setAudioOutput(self.audio_output)
            self.player.mediaStatusChanged.connect(self._on_media_status)
            print("   ✓ QMediaPlayer создан")

            QTimer.singleShot(100, self._step5_play)
        except Exception as e:
            self._fail(f"Ошибка плеера: {e}")

    def _on_media_status(self, status):
        print(f"   [Player] {status}")

    def _step5_play(self):
        """Шаг 5: Воспроизведение."""
        print("\n[Шаг 5] Воспроизведение...")
        self.main_window.status_label.setText("Воспроизведение...")
        try:
            self.player.setSource(QUrl.fromLocalFile(self.audio_path))
            self.player.play()
            QTimer.singleShot(2000, self._step6_stop)
        except Exception as e:
            self._fail(f"Воспроизведение: {e}")

    def _step6_stop(self):
        """Шаг 6: Остановка."""
        print("\n[Шаг 6] Остановка плеера...")
        self.main_window.status_label.setText("Остановка...")
        try:
            self.player.stop()
            if Path(self.audio_path).exists():
                Path(self.audio_path).unlink()
            print("   ✓ Плеер остановлен, файл удалён")
            QTimer.singleShot(500, self._step7_regenerate)
        except Exception as e:
            self._fail(f"Остановка: {e}")

    def _step7_regenerate(self):
        """Шаг 7: ПЕРЕГЕНЕРАЦИЯ."""
        print(f"\n[Шаг 7] ПЕРЕГЕНЕРАЦИЯ: '{self.test_text}'")
        print("   >>> КРИТИЧЕСКИЙ МОМЕНТ <<<")
        self.main_window.status_label.setText("ПЕРЕГЕНЕРАЦИЯ...")
        try:
            pipeline = TTSPipeline()
            normalized = pipeline.process(self.test_text)
            print(f"   Нормализовано: '{normalized}'")
            print(f"   Model speakers: {self.model.speakers}")

            print("   Вызов apply_tts...")
            with torch.no_grad():
                audio = self.model.apply_tts(
                    text=normalized,
                    speaker='xenia',
                    sample_rate=48000
                )

            if isinstance(audio, torch.Tensor):
                audio_np = audio.numpy()
            else:
                audio_np = audio

            duration = len(audio_np) / 48000
            print(f"   ✓ Перегенерация успешна: {duration:.2f}s")

            self.main_window.status_label.setText("УСПЕХ!")
            self.success = True
            self.timeout_timer.stop()

            # Закрываем через секунду
            QTimer.singleShot(1000, self.app.quit)

        except Exception as e:
            import traceback
            traceback.print_exc()
            self._fail(f"Перегенерация: {e}")


def main():
    signal.signal(signal.SIGINT, signal.SIG_DFL)

    test = IntegrationTestFullUI()
    success = test.run()

    return 0 if success else 1


if __name__ == "__main__":
    sys.exit(main())
