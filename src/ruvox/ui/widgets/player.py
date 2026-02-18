"""Audio player widget with playback controls.

Uses libmpv (python-mpv) for high-quality audio playback with scaletempo2.
"""

import logging
from pathlib import Path

from PyQt6.QtCore import Qt, QTimer, pyqtSignal
from PyQt6.QtGui import QWheelEvent
from PyQt6.QtWidgets import (
    QHBoxLayout,
    QLabel,
    QPushButton,
    QSlider,
    QStyle,
    QVBoxLayout,
    QWidget,
)

from ruvox.ui.models.entry import TextEntry
from ruvox.ui.services.logging_service import safe_slot

logger = logging.getLogger(__name__)

# Try to import mpv; graceful degradation if unavailable
try:
    import mpv

    _MPV_AVAILABLE = True
except (ImportError, OSError) as e:
    _MPV_AVAILABLE = False
    logger.error("mpv (libmpv) недоступен — плеер не будет работать: %s", e)


class SpeedLabel(QLabel):
    """Speed label with mouse wheel and click support."""

    speed_up_requested = pyqtSignal()
    speed_down_requested = pyqtSignal()
    reset_requested = pyqtSignal()

    def wheelEvent(self, event: QWheelEvent) -> None:
        if event.angleDelta().y() > 0:
            self.speed_up_requested.emit()
        elif event.angleDelta().y() < 0:
            self.speed_down_requested.emit()
        event.accept()

    def mousePressEvent(self, event) -> None:
        if event.button() == Qt.MouseButton.LeftButton:
            self.reset_requested.emit()
        else:
            super().mousePressEvent(event)


class PlayerWidget(QWidget):
    """Audio player widget with controls.

    Features:
    - Play/pause/stop
    - Progress bar with seeking
    - Speed control (0.5x - 2.0x)
    - Volume control
    - Navigation (prev/next in queue)
    """

    # Signals
    position_changed = pyqtSignal(float)  # Position in seconds for text sync
    playback_started = pyqtSignal(str)  # entry_id
    playback_stopped = pyqtSignal()
    next_requested = pyqtSignal()
    prev_requested = pyqtSignal()

    def __init__(self, parent: QWidget | None = None):
        super().__init__(parent)

        self.current_entry: TextEntry | None = None
        self._current_speed: float = 1.0
        self._duration_ms = 0
        self._is_playing = False
        self._mpv_available = _MPV_AVAILABLE

        # mpv player setup
        self._player = None
        if self._mpv_available:
            try:
                self._player = mpv.MPV(
                    video=False,
                    ytdl=False,
                    af="scaletempo2",
                )
                self._player.observe_property("duration", self._on_mpv_duration)
                self._player.observe_property("idle-active", self._on_mpv_idle)
            except Exception as e:
                logger.error("Не удалось создать mpv.MPV: %s", e)
                self._mpv_available = False
                self._player = None

        # Position polling timer (5 Hz — enough for word highlight)
        self._position_timer = QTimer(self)
        self._position_timer.setInterval(200)
        self._position_timer.timeout.connect(self._poll_position)

        self._setup_ui()

    def _setup_ui(self) -> None:
        """Setup player UI."""
        layout = QVBoxLayout(self)
        layout.setContentsMargins(8, 8, 8, 8)
        layout.setSpacing(8)

        # Progress bar row
        progress_row = QHBoxLayout()
        progress_row.setSpacing(8)

        self.time_current = QLabel("0:00")
        self.time_current.setFixedWidth(45)
        progress_row.addWidget(self.time_current)

        self.progress_slider = QSlider(Qt.Orientation.Horizontal)
        self.progress_slider.setRange(0, 0)
        self.progress_slider.sliderMoved.connect(self._on_slider_moved)
        self.progress_slider.sliderPressed.connect(self._on_slider_pressed)
        self.progress_slider.sliderReleased.connect(self._on_slider_released)
        progress_row.addWidget(self.progress_slider, 1)

        self.time_total = QLabel("0:00")
        self.time_total.setFixedWidth(45)
        progress_row.addWidget(self.time_total)

        layout.addLayout(progress_row)

        # Controls row
        controls_row = QHBoxLayout()
        controls_row.setSpacing(4)

        # Navigation and playback buttons
        self.btn_prev = QPushButton()
        self.btn_prev.setIcon(self.style().standardIcon(QStyle.StandardPixmap.SP_MediaSkipBackward))
        self.btn_prev.setFixedSize(32, 32)
        self.btn_prev.setToolTip("Предыдущий (P)")
        self.btn_prev.clicked.connect(self.prev_requested.emit)
        controls_row.addWidget(self.btn_prev)

        self.btn_back = QPushButton()
        self.btn_back.setIcon(self.style().standardIcon(QStyle.StandardPixmap.SP_MediaSeekBackward))
        self.btn_back.setFixedSize(32, 32)
        self.btn_back.setToolTip("-5 сек (\u2190)")
        self.btn_back.clicked.connect(lambda: self.seek_relative(-5))
        controls_row.addWidget(self.btn_back)

        self.btn_play_pause = QPushButton()
        self.btn_play_pause.setIcon(self.style().standardIcon(QStyle.StandardPixmap.SP_MediaPlay))
        self.btn_play_pause.setFixedSize(48, 32)
        self.btn_play_pause.setToolTip("Play/Pause (Space)")
        self.btn_play_pause.clicked.connect(self.toggle_play_pause)
        controls_row.addWidget(self.btn_play_pause)

        self.btn_forward = QPushButton()
        self.btn_forward.setIcon(self.style().standardIcon(QStyle.StandardPixmap.SP_MediaSeekForward))
        self.btn_forward.setFixedSize(32, 32)
        self.btn_forward.setToolTip("+5 сек (\u2192)")
        self.btn_forward.clicked.connect(lambda: self.seek_relative(5))
        controls_row.addWidget(self.btn_forward)

        self.btn_next = QPushButton()
        self.btn_next.setIcon(self.style().standardIcon(QStyle.StandardPixmap.SP_MediaSkipForward))
        self.btn_next.setFixedSize(32, 32)
        self.btn_next.setToolTip("Следующий (N)")
        self.btn_next.clicked.connect(self.next_requested.emit)
        controls_row.addWidget(self.btn_next)

        controls_row.addSpacing(16)

        # Speed control
        self.speed_label = SpeedLabel("x1.0")
        self.speed_label.setFixedWidth(36)
        self.speed_label.setAlignment(Qt.AlignmentFlag.AlignCenter)
        self.speed_label.setToolTip("Скорость (скролл / клик = сброс)")
        self.speed_label.speed_up_requested.connect(self.speed_up)
        self.speed_label.speed_down_requested.connect(self.speed_down)
        self.speed_label.reset_requested.connect(self.speed_reset)
        controls_row.addWidget(self.speed_label)

        speed_btn_box = QVBoxLayout()
        speed_btn_box.setSpacing(2)
        speed_btn_box.setContentsMargins(0, 0, 0, 0)
        speed_btn_box.setAlignment(Qt.AlignmentFlag.AlignVCenter)

        _speed_btn_base = "font-size: 7px; border: none;"

        self.btn_speed_up = QPushButton("\u25b2")
        self.btn_speed_up.setFixedSize(24, 13)
        self.btn_speed_up.setStyleSheet(_speed_btn_base + " padding: 0 0 2px 0;")
        self.btn_speed_up.setToolTip("Быстрее (])")
        self.btn_speed_up.setAutoRepeat(True)
        self.btn_speed_up.setAutoRepeatDelay(400)
        self.btn_speed_up.setAutoRepeatInterval(150)
        self.btn_speed_up.clicked.connect(self.speed_up)
        speed_btn_box.addWidget(self.btn_speed_up)

        self.btn_speed_down = QPushButton("\u25bc")
        self.btn_speed_down.setFixedSize(24, 13)
        self.btn_speed_down.setStyleSheet(_speed_btn_base + " padding: 0;")
        self.btn_speed_down.setToolTip("Медленнее ([)")
        self.btn_speed_down.setAutoRepeat(True)
        self.btn_speed_down.setAutoRepeatDelay(400)
        self.btn_speed_down.setAutoRepeatInterval(150)
        self.btn_speed_down.clicked.connect(self.speed_down)
        speed_btn_box.addWidget(self.btn_speed_down)

        controls_row.addLayout(speed_btn_box)

        controls_row.addStretch()

        # Volume control
        self.volume_slider = QSlider(Qt.Orientation.Horizontal)
        self.volume_slider.setRange(0, 100)
        self.volume_slider.setValue(100)
        self.volume_slider.setFixedWidth(80)
        self.volume_slider.setToolTip("Громкость")
        self.volume_slider.valueChanged.connect(self._on_volume_changed)
        controls_row.addWidget(self.volume_slider)

        layout.addLayout(controls_row)

        # Initial state
        self._update_controls_enabled(False)

    def _update_controls_enabled(self, enabled: bool) -> None:
        """Enable/disable playback controls."""
        self.btn_play_pause.setEnabled(enabled)
        self.btn_back.setEnabled(enabled)
        self.btn_forward.setEnabled(enabled)
        self.progress_slider.setEnabled(enabled)

    # Public methods

    def load_entry(self, entry: TextEntry, audio_dir: Path) -> bool:
        """Load audio from entry.

        Args:
            entry: TextEntry with audio_path
            audio_dir: Base directory for audio files

        Returns:
            True if loaded successfully
        """
        if not entry.audio_path:
            return False

        audio_path = audio_dir / entry.audio_path
        if not audio_path.exists():
            return False

        if not self._mpv_available or self._player is None:
            logger.warning("mpv недоступен — воспроизведение невозможно")
            return False

        self.current_entry = entry

        # Reset state for new file
        self._duration_ms = 0
        self.progress_slider.setValue(0)
        self.progress_slider.setRange(0, 0)
        self.time_current.setText("0:00")
        self.time_total.setText("0:00")

        # Set duration from entry (calculated from numpy data during TTS generation)
        if entry.duration_sec is not None:
            self._on_duration_changed(int(entry.duration_sec * 1000))

        self._player.loadfile(str(audio_path))
        self._update_controls_enabled(True)
        return True

    def play(self) -> None:
        """Start or resume playback."""
        if not self._mpv_available or self._player is None:
            return

        self._player.pause = False
        self._set_playing(True)
        self._ensure_duration()

        if self.current_entry:
            self.playback_started.emit(self.current_entry.id)

    def pause(self) -> None:
        """Pause playback."""
        if not self._mpv_available or self._player is None:
            return

        self._player.pause = True
        self._set_playing(False)

    def stop(self) -> None:
        """Stop playback."""
        if not self._mpv_available or self._player is None:
            return

        self._player.command("stop")
        self._set_playing(False)
        self.playback_stopped.emit()

    def toggle_play_pause(self) -> None:
        """Toggle between play and pause."""
        if self._is_playing:
            self.pause()
        else:
            self.play()

    def seek(self, position_sec: float) -> None:
        """Seek to position in seconds."""
        if not self._mpv_available or self._player is None:
            return

        position_sec = max(0.0, min(position_sec, self._duration_ms / 1000.0))
        self._player.seek(position_sec, reference="absolute")
        # Update UI after mpv processes the seek (works on pause too)
        QTimer.singleShot(50, self._sync_position)

    def seek_relative(self, delta_sec: float) -> None:
        """Seek relative to current position."""
        if not self._mpv_available or self._player is None:
            return

        self._player.seek(delta_sec, reference="relative")
        # Update UI after mpv processes the seek (works on pause too)
        QTimer.singleShot(50, self._sync_position)

    def set_speed(self, speed: float) -> None:
        """Set playback speed."""
        speed = max(0.1, min(3.0, round(speed, 1)))
        self._current_speed = speed
        if self._mpv_available and self._player is not None:
            self._player.speed = speed
        self.speed_label.setText(f"x{speed:.1f}")

    def speed_up(self) -> None:
        """Increase playback speed by 0.1."""
        self.set_speed(self._current_speed + 0.1)

    def speed_down(self) -> None:
        """Decrease playback speed by 0.1."""
        self.set_speed(self._current_speed - 0.1)

    def speed_reset(self) -> None:
        """Reset playback speed to 1.0."""
        self.set_speed(1.0)

    def get_position_sec(self) -> float:
        """Get current position in seconds."""
        if not self._mpv_available or self._player is None:
            return 0.0
        pos = self._player.time_pos
        return pos if pos is not None else 0.0

    def get_duration_sec(self) -> float:
        """Get total duration in seconds."""
        return self._duration_ms / 1000.0

    def is_playing(self) -> bool:
        """Check if currently playing."""
        return self._is_playing

    def cleanup(self) -> None:
        """Terminate mpv event thread. Call before application exit."""
        self._position_timer.stop()
        if self._player is not None:
            try:
                self._player.terminate()
            except Exception:
                pass
            self._player = None

    # Private methods

    def _set_playing(self, playing: bool) -> None:
        """Update playing state and icon."""
        self._is_playing = playing
        if playing:
            self.btn_play_pause.setIcon(self.style().standardIcon(QStyle.StandardPixmap.SP_MediaPause))
            self._position_timer.start()
        else:
            self.btn_play_pause.setIcon(self.style().standardIcon(QStyle.StandardPixmap.SP_MediaPlay))
            self._position_timer.stop()

    def _ensure_duration(self) -> None:
        """Read duration directly from mpv if not yet set (fallback)."""
        if self._duration_ms == 0 and self._player is not None:
            dur = self._player.duration
            if dur is not None:
                self._on_duration_changed(int(dur * 1000))

    def _on_mpv_duration(self, _name: str, value) -> None:
        """Handle duration property change from mpv (called from mpv thread)."""
        if value is not None:
            duration_ms = int(value * 1000)
            QTimer.singleShot(0, lambda: self._on_duration_changed(duration_ms))

    def _on_mpv_idle(self, _name: str, value) -> None:
        """Handle idle-active property change (EOF detection, called from mpv thread)."""
        if value is True:
            QTimer.singleShot(0, self._on_playback_ended)

    # Private slots

    @safe_slot
    def _on_duration_changed(self, duration_ms: int) -> None:
        """Handle duration change (Qt thread)."""
        self._duration_ms = duration_ms
        self.progress_slider.setRange(0, duration_ms)
        self.time_total.setText(self._format_time(duration_ms / 1000.0))

    @safe_slot
    def _on_playback_ended(self) -> None:
        """Handle playback ended (Qt thread)."""
        if self._is_playing:
            self._set_playing(False)
            self.playback_stopped.emit()

    @safe_slot
    def _sync_position(self) -> None:
        """Read current position from mpv and update UI.

        Used after seeks (including on pause) to immediately update
        the slider, time label, and word highlight.
        """
        if self._player is None:
            return

        self._ensure_duration()

        pos = self._player.time_pos
        if pos is None:
            return

        position_ms = int(pos * 1000)

        if not self._slider_pressed:
            self.progress_slider.setValue(position_ms)

        self.time_current.setText(self._format_time(pos))
        self.position_changed.emit(pos)

    @safe_slot
    def _poll_position(self) -> None:
        """Poll mpv for current position (called by QTimer during playback)."""
        self._sync_position()

    @safe_slot
    def _on_slider_moved(self, position_ms: int) -> None:
        """Handle slider being dragged."""
        position_sec = position_ms / 1000.0
        self.time_current.setText(self._format_time(position_sec))
        self.position_changed.emit(position_sec)

    _slider_pressed = False

    @safe_slot
    def _on_slider_pressed(self) -> None:
        """Handle slider press - pause updates."""
        self._slider_pressed = True

    @safe_slot
    def _on_slider_released(self) -> None:
        """Handle slider release - seek to position."""
        self._slider_pressed = False
        position_sec = self.progress_slider.value() / 1000.0
        self.seek(position_sec)

    @safe_slot
    def _on_volume_changed(self, value: int) -> None:
        """Handle volume slider change."""
        if self._mpv_available and self._player is not None:
            self._player.volume = value

    @staticmethod
    def _format_time(seconds: float) -> str:
        """Format time as m:ss."""
        minutes = int(seconds // 60)
        secs = int(seconds % 60)
        return f"{minutes}:{secs:02d}"
