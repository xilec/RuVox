"""Storage service for managing history and audio files."""

import json
import logging
from datetime import datetime
from pathlib import Path
from typing import Any

import numpy as np

from ruvox.ui.models.entry import TextEntry, EntryStatus
from ruvox.ui.models.config import UIConfig

logger = logging.getLogger(__name__)


HISTORY_VERSION = 1


class StorageService:
    """Service for persisting entries and audio files.

    Manages:
    - history.json: list of TextEntry
    - audio/{uuid}.wav: audio files
    - audio/{uuid}.timestamps.json: word timestamps
    """

    def __init__(self, config: UIConfig):
        self.config = config
        self.cache_dir = config.cache_dir
        self.audio_dir = self.cache_dir / "audio"
        self.history_file = self.cache_dir / "history.json"

        self._entries: dict[str, TextEntry] = {}
        self._ensure_dirs()
        self._load_history()

    def _ensure_dirs(self) -> None:
        """Create necessary directories."""
        self.cache_dir.mkdir(parents=True, exist_ok=True)
        self.audio_dir.mkdir(parents=True, exist_ok=True)

    def _load_history(self) -> None:
        """Load history from JSON file."""
        if not self.history_file.exists():
            return

        try:
            with open(self.history_file, encoding="utf-8") as f:
                data = json.load(f)

            # Handle version migration if needed
            version = data.get("version", 1)
            if version < HISTORY_VERSION:
                data = self._migrate_history(data, version)

            needs_save = False
            for entry_data in data.get("entries", []):
                entry = TextEntry.from_dict(entry_data)
                # Validate entry status against actual audio file existence
                if self._validate_entry_status(entry):
                    needs_save = True
                self._entries[entry.id] = entry

            # Save if any entries were fixed
            if needs_save:
                self._save_history()

        except (json.JSONDecodeError, KeyError, ValueError) as e:
            # Corrupted history file - start fresh but backup old file
            backup_path = self.history_file.with_suffix(".json.bak")
            if self.history_file.exists():
                self.history_file.rename(backup_path)
            self._entries = {}

    def _validate_entry_status(self, entry: TextEntry) -> bool:
        """Validate and fix entry status based on audio file existence.

        Returns:
            True if entry was modified
        """
        modified = False

        if entry.audio_path:
            audio_path = self.audio_dir / entry.audio_path
            if audio_path.exists():
                # Audio file exists - ensure status is READY
                if entry.status != EntryStatus.READY:
                    entry.status = EntryStatus.READY
                    modified = True
            else:
                # Audio file doesn't exist - reset to PENDING
                entry.audio_path = None
                entry.timestamps_path = None
                entry.duration_sec = None
                entry.audio_generated_at = None
                if entry.status == EntryStatus.READY:
                    entry.status = EntryStatus.PENDING
                    modified = True
        elif entry.status == EntryStatus.PROCESSING:
            # Was processing but no audio - reset to PENDING
            entry.status = EntryStatus.PENDING
            modified = True

        return modified

    def _save_history(self) -> None:
        """Save history to JSON file."""
        data = {
            "version": HISTORY_VERSION,
            "entries": [entry.to_dict() for entry in self._entries.values()],
        }
        with open(self.history_file, "w", encoding="utf-8") as f:
            json.dump(data, f, indent=2, ensure_ascii=False)

    @staticmethod
    def _migrate_history(data: dict[str, Any], from_version: int) -> dict[str, Any]:
        """Migrate history from older version."""
        # Add migration logic here when HISTORY_VERSION increases
        return data

    # CRUD operations

    def add_entry(self, text: str) -> TextEntry:
        """Add a new text entry.

        Note: BOM character is stripped from the beginning because Qt's
        QTextDocument removes it automatically in setPlainText(), which
        would cause position mismatch between Python string and Qt widget.
        """
        # Strip BOM to match Qt's behavior
        clean_text = text.lstrip('\ufeff')
        entry = TextEntry(original_text=clean_text)
        self._entries[entry.id] = entry
        self._save_history()
        return entry

    def get_entry(self, entry_id: str) -> TextEntry | None:
        """Get entry by ID."""
        return self._entries.get(entry_id)

    def update_entry(self, entry: TextEntry) -> None:
        """Update an existing entry."""
        self._entries[entry.id] = entry
        self._save_history()

    def delete_entry(self, entry_id: str) -> None:
        """Delete entry and its associated files."""
        entry = self._entries.get(entry_id)
        if entry:
            # Delete audio file
            if entry.audio_path:
                audio_path = self.audio_dir / entry.audio_path
                if audio_path.exists():
                    try:
                        audio_path.unlink()
                    except OSError as e:
                        logger.error("Не удалось удалить аудио файл %s: %s", audio_path, e)

            # Delete timestamps file
            if entry.timestamps_path:
                ts_path = self.audio_dir / entry.timestamps_path
                if ts_path.exists():
                    try:
                        ts_path.unlink()
                    except OSError as e:
                        logger.error("Не удалось удалить файл таймстемпов %s: %s", ts_path, e)

            del self._entries[entry_id]
            self._save_history()

    def delete_audio(self, entry_id: str) -> None:
        """Delete only the audio file for an entry."""
        entry = self._entries.get(entry_id)
        if entry:
            if entry.audio_path:
                audio_path = self.audio_dir / entry.audio_path
                if audio_path.exists():
                    try:
                        audio_path.unlink()
                    except OSError as e:
                        logger.error("Не удалось удалить аудио файл %s: %s", audio_path, e)
                entry.audio_path = None

            if entry.timestamps_path:
                ts_path = self.audio_dir / entry.timestamps_path
                if ts_path.exists():
                    try:
                        ts_path.unlink()
                    except OSError as e:
                        logger.error("Не удалось удалить файл таймстемпов %s: %s", ts_path, e)
                entry.timestamps_path = None

            entry.status = EntryStatus.PENDING
            entry.audio_generated_at = None
            entry.duration_sec = None
            self._save_history()

    def get_all_entries(self) -> list[TextEntry]:
        """Get all entries sorted by creation date (newest first)."""
        return sorted(
            self._entries.values(),
            key=lambda e: e.created_at,
            reverse=True,
        )

    # Audio operations

    def save_audio(
        self,
        entry_id: str,
        audio_data: np.ndarray,
        sample_rate: int,
    ) -> Path:
        """Save audio data to WAV file.

        Returns:
            Relative path to audio file (relative to audio_dir)
        """
        try:
            from scipy.io import wavfile
        except ImportError:
            raise ImportError("scipy is required for audio saving. Install with: pip install scipy")

        filename = f"{entry_id}.wav"
        filepath = self.audio_dir / filename
        wavfile.write(filepath, sample_rate, audio_data)
        return Path(filename)

    def save_timestamps(
        self,
        entry_id: str,
        timestamps: list[dict[str, Any]],
    ) -> Path | None:
        """Save word timestamps to JSON file.

        Args:
            entry_id: Entry ID
            timestamps: List of {"word": str, "start": float, "end": float, "original_pos": [int, int]}

        Returns:
            Relative path to timestamps file, or None on error
        """
        filename = f"{entry_id}.timestamps.json"
        filepath = self.audio_dir / filename
        data = {"words": timestamps}
        try:
            with open(filepath, "w", encoding="utf-8") as f:
                json.dump(data, f, indent=2, ensure_ascii=False)
            return Path(filename)
        except (OSError, TypeError, ValueError) as e:
            logger.error("Не удалось сохранить таймстемпы %s: %s", filepath, e)
            return None

    def load_timestamps(self, entry_id: str) -> list[dict[str, Any]] | None:
        """Load word timestamps from JSON file."""
        entry = self._entries.get(entry_id)
        if not entry or not entry.timestamps_path:
            return None

        filepath = self.audio_dir / entry.timestamps_path
        if not filepath.exists():
            return None

        try:
            with open(filepath, encoding="utf-8") as f:
                data = json.load(f)
            return data.get("words", [])
        except (OSError, json.JSONDecodeError) as e:
            logger.error("Не удалось загрузить таймстемпы %s: %s", filepath, e)
            return None

    def get_audio_path(self, entry_id: str) -> Path | None:
        """Get full path to audio file."""
        entry = self._entries.get(entry_id)
        if not entry or not entry.audio_path:
            return None
        return self.audio_dir / entry.audio_path

    # Stats

    def get_cache_size(self) -> int:
        """Get total cache size in bytes."""
        total = 0
        for path in self.audio_dir.glob("*"):
            if path.is_file():
                total += path.stat().st_size
        return total

    def get_audio_count(self) -> int:
        """Get number of audio files."""
        return len(list(self.audio_dir.glob("*.wav")))
