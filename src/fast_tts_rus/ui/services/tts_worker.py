"""Background TTS processing worker."""

import logging
from datetime import datetime
from typing import Any

# IMPORTANT: torch must be imported before QMediaPlayer/QAudioOutput are created.
# If torch is imported in a worker thread after Qt multimedia components exist,
# it causes "double free or corruption" crashes due to memory initialization conflicts.
import torch

from PyQt6.QtCore import QObject, QRunnable, QThreadPool, pyqtSignal

from fast_tts_rus.ui.models.entry import TextEntry, EntryStatus
from fast_tts_rus.ui.models.config import UIConfig

logger = logging.getLogger(__name__)


class TTSSignals(QObject):
    """Signals for TTS runnable."""
    started = pyqtSignal(str)           # entry_id
    progress = pyqtSignal(str, float)   # entry_id, progress 0-1
    completed = pyqtSignal(str)         # entry_id
    error = pyqtSignal(str, str)        # entry_id, error_message


class ModelLoadSignals(QObject):
    """Signals for model loading runnable."""
    loaded = pyqtSignal(object)         # model
    error = pyqtSignal(str)             # error_message


class ModelLoadRunnable(QRunnable):
    """Runnable for loading Silero TTS model in background."""

    def __init__(self, config: UIConfig):
        super().__init__()
        self.config = config
        self.signals = ModelLoadSignals()

    def run(self) -> None:
        """Load the Silero TTS model."""
        try:
            import torch

            # Load Silero model V5
            # Model will be downloaded on first run
            model, _ = torch.hub.load(
                repo_or_dir='snakers4/silero-models',
                model='silero_tts',
                language='ru',
                speaker='v5_ru'
            )

            self.signals.loaded.emit(model)

        except Exception as e:
            self.signals.error.emit(str(e))


class TTSRunnable(QRunnable):
    """Runnable for TTS processing."""

    def __init__(
        self,
        entry: TextEntry,
        config: UIConfig,
        storage,
        silero_model,
    ):
        super().__init__()
        self.entry = entry
        self.config = config
        self.storage = storage
        self.silero_model = silero_model
        self.signals = TTSSignals()

    def run(self) -> None:
        """Run TTS processing for the entry."""
        try:
            logger.info(f"TTS начало: {self.entry.id[:8]}...")
            self.signals.started.emit(self.entry.id)

            # Import here to avoid loading at module level
            from fast_tts_rus.tts_pipeline import TTSPipeline
            import torch

            pipeline = TTSPipeline()

            # Step 1: Normalize text
            logger.debug("Нормализация текста...")
            normalized = pipeline.process(self.entry.original_text)
            self.entry.normalized_text = normalized

            if not normalized.strip():
                raise ValueError("Normalized text is empty")

            self.signals.progress.emit(self.entry.id, 0.3)

            # Step 2: Synthesize audio
            logger.debug(f"Синтез аудио... Текст ({len(normalized)} символов): {normalized[:100]}...")
            logger.debug(f"Model type: {type(self.silero_model)}, speaker={self.config.speaker}, rate={self.config.sample_rate}")

            with torch.no_grad():
                audio = self.silero_model.apply_tts(
                    text=normalized,
                    speaker=self.config.speaker,
                    sample_rate=self.config.sample_rate,
                )

            # Convert to numpy array
            if isinstance(audio, torch.Tensor):
                audio_np = audio.numpy()
            else:
                audio_np = audio

            self.signals.progress.emit(self.entry.id, 0.7)

            # Calculate duration
            duration_sec = len(audio_np) / self.config.sample_rate

            # Step 3: Estimate timestamps (fallback method based on word length)
            timestamps = self._estimate_timestamps(normalized, duration_sec)

            self.signals.progress.emit(self.entry.id, 0.9)

            # Step 4: Save audio and timestamps
            logger.debug("Сохранение аудио...")
            audio_path = self.storage.save_audio(
                self.entry.id,
                audio_np,
                self.config.sample_rate
            )
            timestamps_path = self.storage.save_timestamps(
                self.entry.id,
                timestamps
            )

            # Step 5: Update entry
            self.entry.audio_path = audio_path
            self.entry.timestamps_path = timestamps_path
            self.entry.status = EntryStatus.READY
            self.entry.audio_generated_at = datetime.now()
            self.entry.duration_sec = duration_sec

            self.storage.update_entry(self.entry)

            self.signals.progress.emit(self.entry.id, 1.0)
            self.signals.completed.emit(self.entry.id)
            logger.info(f"TTS завершено: {self.entry.id[:8]}... ({duration_sec:.1f}s)")

        except Exception as e:
            logger.error(f"TTS ошибка: {self.entry.id[:8]}... - {e}", exc_info=True)
            self.entry.status = EntryStatus.ERROR
            self.entry.error_message = str(e)
            self.storage.update_entry(self.entry)
            self.signals.error.emit(self.entry.id, str(e))

    def _estimate_timestamps(
        self,
        text: str,
        total_duration: float
    ) -> list[dict[str, Any]]:
        """Estimate word timestamps based on word length.

        This is a fallback method when Silero doesn't provide timestamps.
        It estimates timing based on the proportion of each word's length
        to the total text length.

        Note: This is approximate and may not align perfectly with speech.
        """
        words = text.split()
        if not words:
            return []

        # Count total characters (excluding spaces)
        total_chars = sum(len(w) for w in words)
        if total_chars == 0:
            return []

        timestamps = []
        current_time = 0.0
        current_pos = 0  # Position in original text

        for word in words:
            # Estimate word duration proportionally to character count
            word_duration = (len(word) / total_chars) * total_duration

            # Find word position in original text
            # Note: This is simplified - in full implementation we'd need
            # proper mapping from normalized to original text
            word_start = current_pos
            word_end = current_pos + len(word)

            timestamps.append({
                "word": word,
                "start": round(current_time, 3),
                "end": round(current_time + word_duration, 3),
                "original_pos": [word_start, word_end]
            })

            current_time += word_duration
            current_pos = word_end + 1  # +1 for space

        return timestamps


class TTSWorker(QObject):
    """Background TTS worker managing synthesis jobs."""

    # Entry status signals
    started = pyqtSignal(str)           # entry_id
    progress = pyqtSignal(str, float)   # entry_id, progress 0-1
    completed = pyqtSignal(str)         # entry_id
    error = pyqtSignal(str, str)        # entry_id, error_message

    # Model loading signals
    model_loading = pyqtSignal()        # Started loading model
    model_loaded = pyqtSignal()         # Model ready
    model_error = pyqtSignal(str)       # Error loading model

    # Playback signal
    play_requested = pyqtSignal(str)    # entry_id to play

    def __init__(self, config: UIConfig, storage, parent=None):
        super().__init__(parent)
        self.config = config
        self.storage = storage
        self.silero_model = None
        self.model_loading_in_progress = False
        self.thread_pool = QThreadPool()
        self.play_queue: list[str] = []  # entry_ids to play after ready
        self._pending_jobs: list[tuple[TextEntry, bool]] = []

    def ensure_model_loaded(self) -> bool:
        """Ensure the Silero model is loaded.

        Returns True if model is ready, False if loading in progress.
        First load will download ~100MB model.
        """
        if self.silero_model is not None:
            return True

        if self.model_loading_in_progress:
            return False

        self.model_loading_in_progress = True
        self.model_loading.emit()

        runnable = ModelLoadRunnable(self.config)
        runnable.signals.loaded.connect(self._on_model_loaded)
        runnable.signals.error.connect(self._on_model_error)
        self.thread_pool.start(runnable)

        return False

    def _on_model_loaded(self, model) -> None:
        """Handle model loaded."""
        self.silero_model = model
        self.model_loading_in_progress = False
        self.model_loaded.emit()

        # Process pending jobs
        for entry, play_when_ready in self._pending_jobs:
            self._start_processing(entry, play_when_ready)
        self._pending_jobs.clear()

    def _on_model_error(self, error_msg: str) -> None:
        """Handle model loading error."""
        self.model_loading_in_progress = False
        self.model_error.emit(error_msg)

        # Mark pending jobs as error
        for entry, _ in self._pending_jobs:
            entry.status = EntryStatus.ERROR
            entry.error_message = f"Model load failed: {error_msg}"
            self.storage.update_entry(entry)
            self.error.emit(entry.id, entry.error_message)
        self._pending_jobs.clear()

    def process(self, entry: TextEntry, play_when_ready: bool = False) -> None:
        """Queue an entry for TTS processing.

        Args:
            entry: TextEntry to process
            play_when_ready: If True, will emit play_requested when done
        """
        # Update status to processing
        entry.status = EntryStatus.PROCESSING
        self.storage.update_entry(entry)

        if play_when_ready:
            self.play_queue.append(entry.id)

        # Ensure model is loaded
        if not self.ensure_model_loaded():
            # Model loading - queue the job
            self._pending_jobs.append((entry, play_when_ready))
            return

        self._start_processing(entry, play_when_ready)

    def _start_processing(self, entry: TextEntry, play_when_ready: bool) -> None:
        """Start processing an entry."""
        runnable = TTSRunnable(
            entry=entry,
            config=self.config,
            storage=self.storage,
            silero_model=self.silero_model,
        )
        runnable.signals.started.connect(self.started.emit)
        runnable.signals.progress.connect(self.progress.emit)
        runnable.signals.completed.connect(self._on_completed)
        runnable.signals.error.connect(self._on_error)

        self.thread_pool.start(runnable)

    def _on_completed(self, entry_id: str) -> None:
        """Handle TTS completion."""
        self.completed.emit(entry_id)

        # Check if auto-play was requested
        if entry_id in self.play_queue:
            self.play_queue.remove(entry_id)
            self.play_requested.emit(entry_id)

    def _on_error(self, entry_id: str, error_msg: str) -> None:
        """Handle TTS error."""
        self.error.emit(entry_id, error_msg)

        # Remove from play queue if present
        if entry_id in self.play_queue:
            self.play_queue.remove(entry_id)

    def cancel_pending(self, entry_id: str) -> None:
        """Cancel a pending job (before processing starts)."""
        # Remove from play queue
        if entry_id in self.play_queue:
            self.play_queue.remove(entry_id)

        # Remove from pending jobs
        self._pending_jobs = [
            (e, p) for e, p in self._pending_jobs
            if e.id != entry_id
        ]

    def is_model_loaded(self) -> bool:
        """Check if the TTS model is loaded."""
        return self.silero_model is not None

    def is_loading(self) -> bool:
        """Check if the model is currently loading."""
        return self.model_loading_in_progress
