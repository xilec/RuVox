"""Audio player widget with playback controls."""

import logging
from pathlib import Path

logger = logging.getLogger(__name__)

from PyQt6.QtCore import Qt, QUrl, pyqtSignal
from PyQt6.QtWidgets import (
    QWidget,
    QVBoxLayout,
    QHBoxLayout,
    QPushButton,
    QSlider,
    QLabel,
    QStyle,
)
from PyQt6.QtMultimedia import QMediaPlayer, QAudioOutput

from fast_tts_rus.ui.models.entry import TextEntry


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
    playback_started = pyqtSignal(str)    # entry_id
    playback_stopped = pyqtSignal()
    next_requested = pyqtSignal()
    prev_requested = pyqtSignal()

    SPEEDS = [0.5, 0.75, 1.0, 1.25, 1.5, 2.0]

    def __init__(self, parent=None):
        super().__init__(parent)

        self.current_entry: TextEntry | None = None
        self._current_speed_index = 2  # 1.0x
        self._duration_ms = 0

        # Media player setup
        self.player = QMediaPlayer()
        self.audio_output = QAudioOutput()
        self.player.setAudioOutput(self.audio_output)

        # Connect player signals
        self.player.positionChanged.connect(self._on_position_changed)
        self.player.durationChanged.connect(self._on_duration_changed)
        self.player.playbackStateChanged.connect(self._on_state_changed)
        self.player.errorOccurred.connect(self._on_error)

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
        self.btn_speed_down = QPushButton("[")
        self.btn_speed_down.setFixedSize(24, 32)
        self.btn_speed_down.setToolTip("Медленнее ([)")
        self.btn_speed_down.clicked.connect(self.speed_down)
        controls_row.addWidget(self.btn_speed_down)

        self.speed_label = QLabel("1.0x")
        self.speed_label.setFixedWidth(40)
        self.speed_label.setAlignment(Qt.AlignmentFlag.AlignCenter)
        controls_row.addWidget(self.speed_label)

        self.btn_speed_up = QPushButton("]")
        self.btn_speed_up.setFixedSize(24, 32)
        self.btn_speed_up.setToolTip("Быстрее (])")
        self.btn_speed_up.clicked.connect(self.speed_up)
        controls_row.addWidget(self.btn_speed_up)

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

        self.current_entry = entry
        self.player.setSource(QUrl.fromLocalFile(str(audio_path)))
        self._update_controls_enabled(True)
        return True

    def play(self) -> None:
        """Start or resume playback."""
        if self.player.playbackState() == QMediaPlayer.PlaybackState.PausedState:
            self.player.play()
        elif self.current_entry:
            self.player.play()
            if self.current_entry:
                self.playback_started.emit(self.current_entry.id)

    def pause(self) -> None:
        """Pause playback."""
        self.player.pause()

    def stop(self) -> None:
        """Stop playback."""
        self.player.stop()
        self.playback_stopped.emit()

    def toggle_play_pause(self) -> None:
        """Toggle between play and pause."""
        if self.player.playbackState() == QMediaPlayer.PlaybackState.PlayingState:
            self.pause()
        else:
            self.play()

    def seek(self, position_sec: float) -> None:
        """Seek to position in seconds."""
        position_ms = int(position_sec * 1000)
        position_ms = max(0, min(position_ms, self._duration_ms))
        self.player.setPosition(position_ms)

    def seek_relative(self, delta_sec: float) -> None:
        """Seek relative to current position."""
        current_ms = self.player.position()
        new_ms = current_ms + int(delta_sec * 1000)
        new_ms = max(0, min(new_ms, self._duration_ms))
        self.player.setPosition(new_ms)

    def set_speed(self, speed: float) -> None:
        """Set playback speed."""
        speed = max(0.5, min(2.0, speed))
        self.player.setPlaybackRate(speed)
        self.speed_label.setText(f"{speed}x")

    def speed_up(self) -> None:
        """Increase playback speed to next preset."""
        if self._current_speed_index < len(self.SPEEDS) - 1:
            self._current_speed_index += 1
            self.set_speed(self.SPEEDS[self._current_speed_index])

    def speed_down(self) -> None:
        """Decrease playback speed to previous preset."""
        if self._current_speed_index > 0:
            self._current_speed_index -= 1
            self.set_speed(self.SPEEDS[self._current_speed_index])

    def get_position_sec(self) -> float:
        """Get current position in seconds."""
        return self.player.position() / 1000.0

    def get_duration_sec(self) -> float:
        """Get total duration in seconds."""
        return self._duration_ms / 1000.0

    def is_playing(self) -> bool:
        """Check if currently playing."""
        return self.player.playbackState() == QMediaPlayer.PlaybackState.PlayingState

    # Private slots

    def _on_position_changed(self, position_ms: int) -> None:
        """Handle position change from player."""
        if not self._slider_pressed:
            self.progress_slider.setValue(position_ms)

        # Update time label
        position_sec = position_ms / 1000.0
        self.time_current.setText(self._format_time(position_sec))

        # Emit for text synchronization
        self.position_changed.emit(position_sec)

    def _on_duration_changed(self, duration_ms: int) -> None:
        """Handle duration change."""
        self._duration_ms = duration_ms
        self.progress_slider.setRange(0, duration_ms)
        self.time_total.setText(self._format_time(duration_ms / 1000.0))

    def _on_state_changed(self, state: QMediaPlayer.PlaybackState) -> None:
        """Handle playback state change."""
        if state == QMediaPlayer.PlaybackState.PlayingState:
            self.btn_play_pause.setIcon(
                self.style().standardIcon(QStyle.StandardPixmap.SP_MediaPause)
            )
        else:
            self.btn_play_pause.setIcon(
                self.style().standardIcon(QStyle.StandardPixmap.SP_MediaPlay)
            )

        if state == QMediaPlayer.PlaybackState.StoppedState:
            self.playback_stopped.emit()

    def _on_error(self, error: QMediaPlayer.Error, error_string: str) -> None:
        """Handle player error."""
        logger.error("Player error: %s (code=%s)", error_string, error)

    def _on_slider_moved(self, position_ms: int) -> None:
        """Handle slider being dragged."""
        # Update time display while dragging
        self.time_current.setText(self._format_time(position_ms / 1000.0))

    _slider_pressed = False

    def _on_slider_pressed(self) -> None:
        """Handle slider press - pause updates."""
        self._slider_pressed = True

    def _on_slider_released(self) -> None:
        """Handle slider release - seek to position."""
        self._slider_pressed = False
        self.player.setPosition(self.progress_slider.value())

    def _on_volume_changed(self, value: int) -> None:
        """Handle volume slider change."""
        self.audio_output.setVolume(value / 100.0)

    @staticmethod
    def _format_time(seconds: float) -> str:
        """Format time as m:ss."""
        minutes = int(seconds // 60)
        secs = int(seconds % 60)
        return f"{minutes}:{secs:02d}"
