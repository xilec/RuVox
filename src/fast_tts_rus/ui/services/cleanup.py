"""Background cleanup service for old entries and audio files."""

import logging
from datetime import datetime

from PyQt6.QtCore import QObject, QRunnable, QThreadPool, pyqtSignal

from fast_tts_rus.ui.models.config import UIConfig

logger = logging.getLogger(__name__)


class CleanupSignals(QObject):
    """Signals for cleanup runnable."""
    completed = pyqtSignal(int)  # Number of deleted items


class CleanupRunnable(QRunnable):
    """Runnable for background cleanup."""

    def __init__(self, config: UIConfig, storage):
        super().__init__()
        self.config = config
        self.storage = storage
        self.signals = CleanupSignals()

    def run(self) -> None:
        """Execute cleanup."""
        try:
            deleted_count = 0
            now = datetime.now()

            entries = self.storage.get_all_entries()

            for entry in entries:
                should_delete_text = False
                should_delete_audio = False

                # Rule 1: delete texts older than N days
                age_days = (now - entry.created_at).days
                if age_days > self.config.history_days:
                    should_delete_text = True

                # Rule 3: regenerated audio - keep for N hours only
                if entry.was_regenerated and entry.audio_generated_at:
                    age_hours = (now - entry.audio_generated_at).total_seconds() / 3600
                    if age_hours > self.config.audio_regenerated_hours:
                        should_delete_audio = True

                if should_delete_text:
                    self.storage.delete_entry(entry.id)
                    deleted_count += 1
                elif should_delete_audio:
                    self.storage.delete_audio(entry.id)

            # Rule 2: keep only N most recent audio files
            deleted_count += self._cleanup_old_audio_files()

            self.signals.completed.emit(deleted_count)
        except Exception as e:
            logger.error("Ошибка при очистке: %s", e, exc_info=True)

    def _cleanup_old_audio_files(self) -> int:
        """Delete old audio files, keeping only the most recent N."""
        entries_with_audio = [
            e for e in self.storage.get_all_entries()
            if e.audio_path and e.audio_path.exists() and not e.was_regenerated
        ]

        # Sort by generation time (newest first)
        entries_with_audio.sort(
            key=lambda e: e.audio_generated_at or e.created_at,
            reverse=True
        )

        deleted = 0
        for entry in entries_with_audio[self.config.audio_max_files:]:
            self.storage.delete_audio(entry.id)
            deleted += 1

        return deleted


class CleanupWorker(QObject):
    """Background cleanup worker."""

    cleanup_completed = pyqtSignal(int)  # Number of deleted items

    def __init__(self, config: UIConfig, storage, parent=None):
        super().__init__(parent)
        self.config = config
        self.storage = storage

    def run_cleanup(self) -> None:
        """Start cleanup in background thread."""
        runnable = CleanupRunnable(self.config, self.storage)
        runnable.signals.completed.connect(self._on_completed)
        QThreadPool.globalInstance().start(runnable)

    def _on_completed(self, count: int) -> None:
        """Handle cleanup completion."""
        self.cleanup_completed.emit(count)
