"""Background TTS processing worker."""

import logging
import re
import threading
from datetime import datetime
from typing import Any

# IMPORTANT: torch must be imported early at module level.
# Importing torch in a worker thread can cause crashes due to memory
# initialization conflicts with other native libraries.
import torch
import numpy as np

from PyQt6.QtCore import QObject, QRunnable, QThreadPool, pyqtSignal

from fast_tts_rus.ui.models.entry import TextEntry, EntryStatus
from fast_tts_rus.ui.models.config import UIConfig

logger = logging.getLogger(__name__)

# Maximum characters per TTS chunk (Silero limit is ~1000-1500)
MAX_CHUNK_SIZE = 900


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
            import socket
            import torch

            # Set timeout for network operations to prevent hanging forever
            old_timeout = socket.getdefaulttimeout()
            socket.setdefaulttimeout(60.0)  # 60 seconds

            try:
                # Load Silero model V5
                # Model will be downloaded on first run
                model, _ = torch.hub.load(
                    repo_or_dir='snakers4/silero-models',
                    model='silero_tts',
                    language='ru',
                    speaker='v5_ru'
                )

                self.signals.loaded.emit(model)
            finally:
                socket.setdefaulttimeout(old_timeout)

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

            # Step 1: Normalize text with precise character mapping
            logger.debug("Нормализация текста...")
            normalized, char_mapping = pipeline.process_with_char_mapping(
                self.entry.original_text
            )
            self.entry.normalized_text = normalized

            if not normalized.strip():
                raise ValueError("Normalized text is empty")

            self.signals.progress.emit(self.entry.id, 0.3)

            # Step 2: Split into chunks and synthesize audio
            chunks = self._split_into_chunks(normalized)
            logger.debug(f"Синтез аудио... Текст ({len(normalized)} символов), {len(chunks)} частей")
            logger.debug(f"Model type: {type(self.silero_model)}, speaker={self.config.speaker}, rate={self.config.sample_rate}")

            audio_parts: list[np.ndarray] = []
            chunk_durations: list[tuple[int, int, float]] = []  # (norm_start, norm_end, duration)

            for i, (chunk_text, chunk_start) in enumerate(chunks):
                logger.debug(f"Синтез части {i+1}/{len(chunks)}: {len(chunk_text)} символов")

                with torch.no_grad():
                    audio = self.silero_model.apply_tts(
                        text=chunk_text,
                        speaker=self.config.speaker,
                        sample_rate=self.config.sample_rate,
                    )

                # Convert to numpy array
                if isinstance(audio, torch.Tensor):
                    audio_np = audio.numpy()
                else:
                    audio_np = audio

                audio_parts.append(audio_np)
                chunk_duration = len(audio_np) / self.config.sample_rate
                chunk_durations.append((chunk_start, chunk_start + len(chunk_text), chunk_duration))

                # Update progress
                progress = 0.3 + 0.4 * (i + 1) / len(chunks)
                self.signals.progress.emit(self.entry.id, progress)

            # Concatenate audio parts
            audio_np = np.concatenate(audio_parts) if len(audio_parts) > 1 else audio_parts[0]

            # Calculate total duration
            duration_sec = len(audio_np) / self.config.sample_rate

            # Step 3: Estimate timestamps with precise mapping to original text
            timestamps = self._estimate_timestamps_chunked(
                normalized, chunk_durations, char_mapping
            )

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

    def _split_into_chunks(self, text: str) -> list[tuple[str, int]]:
        """Split text into chunks for TTS processing.

        Splits on sentence boundaries to maintain natural speech flow.
        Returns list of (chunk_text, start_position) tuples.

        Args:
            text: Normalized text to split

        Returns:
            List of (chunk, start_pos) tuples
        """
        if len(text) <= MAX_CHUNK_SIZE:
            return [(text, 0)]

        chunks: list[tuple[str, int]] = []
        current_pos = 0

        while current_pos < len(text):
            # Calculate end of potential chunk
            chunk_end = min(current_pos + MAX_CHUNK_SIZE, len(text))

            if chunk_end >= len(text):
                # Last chunk
                chunks.append((text[current_pos:], current_pos))
                break

            # Find best split point (sentence boundary)
            chunk_text = text[current_pos:chunk_end]

            # Try to find sentence end (. ! ?)
            best_split = -1
            for match in re.finditer(r'[.!?]\s+', chunk_text):
                best_split = match.end()

            if best_split == -1:
                # No sentence boundary, try comma or semicolon
                for match in re.finditer(r'[,;:]\s+', chunk_text):
                    best_split = match.end()

            if best_split == -1:
                # No punctuation, split on word boundary
                for match in re.finditer(r'\s+', chunk_text):
                    best_split = match.end()

            if best_split == -1 or best_split < len(chunk_text) // 2:
                # Fallback: hard split at max size
                best_split = MAX_CHUNK_SIZE

            # Add chunk
            actual_chunk = text[current_pos:current_pos + best_split].strip()
            if actual_chunk:
                chunks.append((actual_chunk, current_pos))

            current_pos += best_split

        logger.debug(f"Текст разбит на {len(chunks)} частей")
        return chunks

    def _estimate_timestamps_chunked(
        self,
        text: str,
        chunk_durations: list[tuple[int, int, float]],
        char_mapping=None,
    ) -> list[dict[str, Any]]:
        """Estimate word timestamps for chunked audio.

        Uses CharMapping for precise position mapping from normalized to original text.

        Args:
            text: Full normalized text
            chunk_durations: List of (norm_start, norm_end, duration) for each chunk
            char_mapping: CharMapping for position conversion

        Returns:
            List of timestamp dictionaries with word, start, end, original_pos
        """
        timestamps: list[dict[str, Any]] = []
        audio_offset = 0.0

        for chunk_start, chunk_end, chunk_duration in chunk_durations:
            chunk_text = text[chunk_start:chunk_end]

            # Extract words with their positions in the chunk
            chunk_words = self._extract_words_with_positions(chunk_text)
            if not chunk_words:
                audio_offset += chunk_duration
                continue

            total_chars = sum(len(w) for w, _, _ in chunk_words)
            if total_chars == 0:
                audio_offset += chunk_duration
                continue

            current_time = 0.0

            for word, word_start_in_chunk, word_end_in_chunk in chunk_words:
                word_duration = (len(word) / total_chars) * chunk_duration

                # Calculate position in full normalized text
                norm_start = chunk_start + word_start_in_chunk
                norm_end = chunk_start + word_end_in_chunk

                # Map to original text position using CharMapping
                if char_mapping is not None:
                    orig_start, orig_end = char_mapping.get_original_range(
                        norm_start, norm_end
                    )
                else:
                    orig_start, orig_end = norm_start, norm_end

                timestamps.append({
                    "word": word,
                    "start": round(audio_offset + current_time, 3),
                    "end": round(audio_offset + current_time + word_duration, 3),
                    "original_pos": [orig_start, orig_end]
                })

                current_time += word_duration

            audio_offset += chunk_duration

        return timestamps

    def _extract_words_with_positions(self, text: str) -> list[tuple[str, int, int]]:
        """Extract words from text with their positions.

        Returns:
            List of (word, start, end) tuples
        """
        words = []
        i = 0
        while i < len(text):
            # Skip whitespace
            while i < len(text) and text[i].isspace():
                i += 1
            if i >= len(text):
                break
            # Find word
            start = i
            while i < len(text) and not text[i].isspace():
                i += 1
            end = i
            word = text[start:end]
            words.append((word, start, end))
        return words

    def _estimate_timestamps(
        self,
        text: str,
        total_duration: float,
        char_mapping=None,
    ) -> list[dict[str, Any]]:
        """Estimate word timestamps based on word length.

        This is a fallback method when Silero doesn't provide timestamps.
        It estimates timing based on the proportion of each word's length
        to the total text length.

        Args:
            text: Normalized text
            total_duration: Total audio duration in seconds
            char_mapping: CharMapping from TrackedText for precise position mapping

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
        current_pos = 0  # Position in normalized text

        for word in words:
            # Estimate word duration proportionally to character count
            word_duration = (len(word) / total_chars) * total_duration

            # Calculate word position in normalized text
            word_start_norm = current_pos
            word_end_norm = current_pos + len(word)

            # Map to original text position using CharMapping
            if char_mapping is not None:
                orig_start, orig_end = char_mapping.get_original_range(
                    word_start_norm, word_end_norm
                )
            else:
                # Fallback: use normalized positions
                orig_start = word_start_norm
                orig_end = word_end_norm

            timestamps.append({
                "word": word,
                "start": round(current_time, 3),
                "end": round(current_time + word_duration, 3),
                "original_pos": [orig_start, orig_end]
            })

            current_time += word_duration
            current_pos = word_end_norm + 1  # +1 for space

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
        self.thread_pool.setMaxThreadCount(2)  # Limit: one for current, one for prefetch
        self.play_queue: list[str] = []  # entry_ids to play after ready
        self._pending_jobs: list[tuple[TextEntry, bool]] = []
        self._active_runnables: dict[str, TTSRunnable] = {}  # entry_id -> runnable
        self._queue_lock = threading.RLock()  # Protects play_queue, _pending_jobs, _active_runnables

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
        with self._queue_lock:
            pending = list(self._pending_jobs)
            self._pending_jobs.clear()
        for entry, play_when_ready in pending:
            self._start_processing(entry, play_when_ready)

    def _on_model_error(self, error_msg: str) -> None:
        """Handle model loading error."""
        self.model_loading_in_progress = False
        self.model_error.emit(error_msg)

        # Mark pending jobs as error
        with self._queue_lock:
            pending = list(self._pending_jobs)
            self._pending_jobs.clear()
        for entry, _ in pending:
            entry.status = EntryStatus.ERROR
            entry.error_message = f"Model load failed: {error_msg}"
            self.storage.update_entry(entry)
            self.error.emit(entry.id, entry.error_message)

    def process(self, entry: TextEntry, play_when_ready: bool = False) -> None:
        """Queue an entry for TTS processing.

        Args:
            entry: TextEntry to process
            play_when_ready: If True, will emit play_requested when done
        """
        # Update status to processing
        entry.status = EntryStatus.PROCESSING
        self.storage.update_entry(entry)

        with self._queue_lock:
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

        with self._queue_lock:
            self._active_runnables[entry.id] = runnable

        self.thread_pool.start(runnable)

    def _cleanup_runnable(self, entry_id: str) -> None:
        """Remove runnable reference and disconnect signals to prevent memory leaks."""
        with self._queue_lock:
            runnable = self._active_runnables.pop(entry_id, None)
        if runnable is not None:
            try:
                runnable.signals.started.disconnect()
                runnable.signals.progress.disconnect()
                runnable.signals.completed.disconnect()
                runnable.signals.error.disconnect()
            except (TypeError, RuntimeError):
                # Signals may already be disconnected
                pass

    def _on_completed(self, entry_id: str) -> None:
        """Handle TTS completion."""
        self._cleanup_runnable(entry_id)
        self.completed.emit(entry_id)

        # Check if auto-play was requested
        with self._queue_lock:
            should_play = entry_id in self.play_queue
            if should_play:
                self.play_queue.remove(entry_id)
        if should_play:
            self.play_requested.emit(entry_id)

    def _on_error(self, entry_id: str, error_msg: str) -> None:
        """Handle TTS error."""
        self._cleanup_runnable(entry_id)
        self.error.emit(entry_id, error_msg)

        # Remove from play queue if present
        with self._queue_lock:
            if entry_id in self.play_queue:
                self.play_queue.remove(entry_id)

    def cancel_pending(self, entry_id: str) -> None:
        """Cancel a pending job (before processing starts)."""
        with self._queue_lock:
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

    def shutdown(self) -> None:
        """Shutdown TTS worker and wait for pending tasks to complete."""
        self.thread_pool.waitForDone(5000)
        with self._queue_lock:
            self._active_runnables.clear()
            self.play_queue.clear()
            self._pending_jobs.clear()
        self.silero_model = None
