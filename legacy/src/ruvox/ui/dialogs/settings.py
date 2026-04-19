"""Settings dialog."""

import logging

from PyQt6.QtCore import Qt
from PyQt6.QtWidgets import (
    QCheckBox,
    QComboBox,
    QDialog,
    QDialogButtonBox,
    QGridLayout,
    QGroupBox,
    QHBoxLayout,
    QLabel,
    QLineEdit,
    QPushButton,
    QSlider,
    QSpinBox,
    QTabWidget,
    QTextEdit,
    QVBoxLayout,
    QWidget,
)

from ruvox.ui.models.config import UIConfig
from ruvox.ui.services.logging_service import safe_slot

logger = logging.getLogger(__name__)


class SettingsDialog(QDialog):
    """Settings dialog for configuring the application."""

    SPEAKERS = ["aidar", "baya", "kseniya", "xenia", "eugene"]
    SAMPLE_RATES = [8000, 24000, 48000]

    def __init__(self, config: UIConfig, hotkey_service=None, parent=None, on_theme_changed=None):
        super().__init__(parent)
        self.config = config
        self.hotkey_service = hotkey_service
        self._original_theme: str = config.theme
        self._on_theme_changed_callback = on_theme_changed

        self.setWindowTitle("Настройки")
        self.setMinimumWidth(500)

        self._setup_ui()
        self._load_settings()

    def _setup_ui(self) -> None:
        """Setup the dialog UI."""
        layout = QVBoxLayout(self)

        # Tab widget
        tabs = QTabWidget()
        tabs.addTab(self._create_appearance_tab(), "Оформление")
        tabs.addTab(self._create_hotkeys_tab(), "Хоткеи")
        tabs.addTab(self._create_voice_tab(), "Голос")
        tabs.addTab(self._create_storage_tab(), "Хранение")
        layout.addWidget(tabs)

        # Button box
        buttons = QDialogButtonBox(
            QDialogButtonBox.StandardButton.Ok
            | QDialogButtonBox.StandardButton.Cancel
            | QDialogButtonBox.StandardButton.Apply
        )
        buttons.accepted.connect(self._on_ok)
        buttons.rejected.connect(self._on_cancel)
        buttons.button(QDialogButtonBox.StandardButton.Apply).clicked.connect(self._apply_settings)
        layout.addWidget(buttons)

    def _create_appearance_tab(self) -> QWidget:
        """Create appearance settings tab."""
        from ruvox.ui.themes import get_available_themes

        tab = QWidget()
        layout = QVBoxLayout(tab)

        group = QGroupBox("Тема оформления")
        group_layout = QVBoxLayout(group)

        self.theme_combo = QComboBox()
        self._theme_ids: list[str] = []
        for theme_id, theme_name in get_available_themes():
            self.theme_combo.addItem(theme_name)
            self._theme_ids.append(theme_id)
        self.theme_combo.currentIndexChanged.connect(self._on_theme_changed)

        group_layout.addWidget(QLabel("Выберите тему:"))
        group_layout.addWidget(self.theme_combo)

        layout.addWidget(group)
        layout.addStretch()

        return tab

    @safe_slot
    def _on_theme_changed(self, index: int) -> None:
        """Apply theme instantly for preview."""
        if 0 <= index < len(self._theme_ids):
            theme_id = self._theme_ids[index]
            if self._on_theme_changed_callback:
                self._on_theme_changed_callback(theme_id)
            else:
                from ruvox.ui.themes import apply_theme

                apply_theme(theme_id)

    def _create_hotkeys_tab(self) -> QWidget:
        """Create hotkeys settings tab."""
        tab = QWidget()
        layout = QVBoxLayout(tab)

        # Hotkey group
        group = QGroupBox("Глобальные хоткеи")
        grid = QGridLayout(group)

        grid.addWidget(QLabel("Читать сразу:"), 0, 0)
        self.hotkey_read_now = QLineEdit()
        self.hotkey_read_now.setPlaceholderText("Ctrl+T")
        grid.addWidget(self.hotkey_read_now, 0, 1)

        grid.addWidget(QLabel("Читать отложенно:"), 1, 0)
        self.hotkey_read_later = QLineEdit()
        self.hotkey_read_later.setPlaceholderText("Ctrl+Shift+T")
        grid.addWidget(self.hotkey_read_later, 1, 1)

        layout.addWidget(group)

        # Instructions
        instructions_group = QGroupBox("Инструкции по настройке")
        instructions_layout = QVBoxLayout(instructions_group)

        self.instructions_text = QTextEdit()
        self.instructions_text.setReadOnly(True)
        self.instructions_text.setMaximumHeight(200)
        if self.hotkey_service:
            self.instructions_text.setPlainText(self.hotkey_service.get_fallback_instructions())
        else:
            self.instructions_text.setPlainText(
                "Настройте хоткеи вручную в вашем окружении рабочего стола.\n\n"
                "Команды для привязки:\n"
                "  Читать сразу: ruvox --read-now\n"
                "  Читать отложенно: ruvox --read-later"
            )
        instructions_layout.addWidget(self.instructions_text)

        layout.addWidget(instructions_group)
        layout.addStretch()

        return tab

    def _create_voice_tab(self) -> QWidget:
        """Create voice settings tab."""
        tab = QWidget()
        layout = QVBoxLayout(tab)

        # Voice group
        group = QGroupBox("Настройки голоса")
        grid = QGridLayout(group)

        # Speaker
        grid.addWidget(QLabel("Спикер:"), 0, 0)
        self.speaker_combo = QComboBox()
        self.speaker_combo.addItems(self.SPEAKERS)
        grid.addWidget(self.speaker_combo, 0, 1)

        # Speech rate
        grid.addWidget(QLabel("Скорость речи:"), 1, 0)
        rate_layout = QHBoxLayout()
        self.rate_slider = QSlider(Qt.Orientation.Horizontal)
        self.rate_slider.setRange(50, 200)  # 0.5x to 2.0x
        self.rate_slider.setTickInterval(25)
        self.rate_slider.setTickPosition(QSlider.TickPosition.TicksBelow)
        self.rate_slider.valueChanged.connect(self._on_rate_changed)
        rate_layout.addWidget(self.rate_slider)
        self.rate_label = QLabel("1.0x")
        self.rate_label.setFixedWidth(40)
        rate_layout.addWidget(self.rate_label)
        grid.addLayout(rate_layout, 1, 1)

        # Sample rate
        grid.addWidget(QLabel("Частота дискретизации:"), 2, 0)
        self.sample_rate_combo = QComboBox()
        self.sample_rate_combo.addItems([str(r) for r in self.SAMPLE_RATES])
        grid.addWidget(self.sample_rate_combo, 2, 1)

        layout.addWidget(group)
        layout.addStretch()

        return tab

    def _create_storage_tab(self) -> QWidget:
        """Create storage settings tab."""
        tab = QWidget()
        layout = QVBoxLayout(tab)

        # Cleanup group
        group = QGroupBox("Очистка")
        grid = QGridLayout(group)

        # History days
        grid.addWidget(QLabel("Хранить тексты (дней):"), 0, 0)
        self.history_days_spin = QSpinBox()
        self.history_days_spin.setRange(1, 365)
        grid.addWidget(self.history_days_spin, 0, 1)

        # Max audio files
        grid.addWidget(QLabel("Макс. аудио файлов:"), 1, 0)
        self.audio_max_spin = QSpinBox()
        self.audio_max_spin.setRange(1, 100)
        grid.addWidget(self.audio_max_spin, 1, 1)

        # Regenerated hours
        grid.addWidget(QLabel("Хранить перегенерированные (часов):"), 2, 0)
        self.regen_hours_spin = QSpinBox()
        self.regen_hours_spin.setRange(1, 168)  # Up to 1 week
        grid.addWidget(self.regen_hours_spin, 2, 1)

        layout.addWidget(group)

        # Notifications group
        notify_group = QGroupBox("Уведомления")
        notify_layout = QVBoxLayout(notify_group)

        self.notify_ready_check = QCheckBox("Уведомлять при готовности аудио")
        notify_layout.addWidget(self.notify_ready_check)

        self.notify_error_check = QCheckBox("Уведомлять об ошибках")
        notify_layout.addWidget(self.notify_error_check)

        layout.addWidget(notify_group)

        # Cache info
        cache_group = QGroupBox("Кэш")
        cache_layout = QHBoxLayout(cache_group)

        self.cache_size_label = QLabel("Занято: — ")
        cache_layout.addWidget(self.cache_size_label)

        cache_layout.addStretch()

        open_btn = QPushButton("Открыть папку")
        open_btn.clicked.connect(self._open_cache_folder)
        cache_layout.addWidget(open_btn)

        layout.addWidget(cache_group)
        layout.addStretch()

        return tab

    def _load_settings(self) -> None:
        """Load current settings into UI."""
        # Appearance
        if self.config.theme in self._theme_ids:
            self.theme_combo.setCurrentIndex(self._theme_ids.index(self.config.theme))

        # Hotkeys
        self.hotkey_read_now.setText(self.config.hotkey_read_now)
        self.hotkey_read_later.setText(self.config.hotkey_read_later)

        # Voice
        speaker_idx = self.SPEAKERS.index(self.config.speaker) if self.config.speaker in self.SPEAKERS else 3
        self.speaker_combo.setCurrentIndex(speaker_idx)
        self.rate_slider.setValue(int(self.config.speech_rate * 100))
        sample_idx = (
            self.SAMPLE_RATES.index(self.config.sample_rate) if self.config.sample_rate in self.SAMPLE_RATES else 2
        )
        self.sample_rate_combo.setCurrentIndex(sample_idx)

        # Storage
        self.history_days_spin.setValue(self.config.history_days)
        self.audio_max_spin.setValue(self.config.audio_max_files)
        self.regen_hours_spin.setValue(self.config.audio_regenerated_hours)

        # Notifications
        self.notify_ready_check.setChecked(self.config.notify_on_ready)
        self.notify_error_check.setChecked(self.config.notify_on_error)

        # Cache size (would need storage service)
        self.cache_size_label.setText(f"Папка: {self.config.cache_dir}")

    def _apply_settings(self) -> None:
        """Apply settings to config."""
        # Appearance
        idx = self.theme_combo.currentIndex()
        if 0 <= idx < len(self._theme_ids):
            self.config.theme = self._theme_ids[idx]
            self._original_theme = self.config.theme

        # Hotkeys
        self.config.hotkey_read_now = self.hotkey_read_now.text() or "Control+t"
        self.config.hotkey_read_later = self.hotkey_read_later.text() or "Control+Shift+t"

        # Voice
        self.config.speaker = self.SPEAKERS[self.speaker_combo.currentIndex()]
        self.config.speech_rate = self.rate_slider.value() / 100.0
        self.config.sample_rate = self.SAMPLE_RATES[self.sample_rate_combo.currentIndex()]

        # Storage
        self.config.history_days = self.history_days_spin.value()
        self.config.audio_max_files = self.audio_max_spin.value()
        self.config.audio_regenerated_hours = self.regen_hours_spin.value()

        # Notifications
        self.config.notify_on_ready = self.notify_ready_check.isChecked()
        self.config.notify_on_error = self.notify_error_check.isChecked()

        # Save config
        self.config.save()

    @safe_slot
    def _on_cancel(self) -> None:
        """Handle Cancel button — revert theme preview."""
        if self._on_theme_changed_callback:
            self._on_theme_changed_callback(self._original_theme)
        else:
            from ruvox.ui.themes import apply_theme

            apply_theme(self._original_theme)
        self.reject()

    @safe_slot
    def _on_ok(self) -> None:
        """Handle OK button."""
        self._apply_settings()
        self.accept()

    @safe_slot
    def _on_rate_changed(self, value: int) -> None:
        """Handle rate slider change."""
        rate = value / 100.0
        self.rate_label.setText(f"{rate:.1f}x")

    def _open_cache_folder(self) -> None:
        """Open cache folder in file manager."""
        import subprocess

        try:
            subprocess.run(["xdg-open", str(self.config.cache_dir)], check=False)
        except Exception:
            pass
