"""Text entry model for TTS queue."""

from dataclasses import dataclass, field
from datetime import datetime
from enum import Enum
from pathlib import Path
from typing import Any
import uuid


class EntryStatus(Enum):
    """Status of a text entry in the TTS pipeline."""

    PENDING = "pending"  # Waiting for TTS
    PROCESSING = "processing"  # TTS in progress
    READY = "ready"  # Audio ready
    ERROR = "error"  # TTS failed


@dataclass
class TextEntry:
    """A text entry in the TTS queue.

    Attributes:
        id: Unique identifier (UUID)
        original_text: Original text before normalization
        normalized_text: Text after normalization (what will be spoken)
        status: Current processing status
        created_at: When the entry was created
        audio_generated_at: When audio was generated (if ready)
        audio_path: Path to WAV file (relative to cache_dir/audio/)
        timestamps_path: Path to timestamps JSON (relative to cache_dir/audio/)
        duration_sec: Audio duration in seconds
        was_regenerated: Whether audio was manually regenerated
        error_message: Error message if status is ERROR
    """

    id: str = field(default_factory=lambda: str(uuid.uuid4()))
    original_text: str = ""
    normalized_text: str | None = None
    status: EntryStatus = EntryStatus.PENDING
    created_at: datetime = field(default_factory=datetime.now)
    audio_generated_at: datetime | None = None
    audio_path: Path | None = None
    timestamps_path: Path | None = None
    duration_sec: float | None = None
    was_regenerated: bool = False
    error_message: str | None = None

    def to_dict(self) -> dict[str, Any]:
        """Convert to dictionary for JSON serialization."""
        return {
            "id": self.id,
            "original_text": self.original_text,
            "normalized_text": self.normalized_text,
            "status": self.status.value,
            "created_at": self.created_at.isoformat(),
            "audio_generated_at": (
                self.audio_generated_at.isoformat()
                if self.audio_generated_at
                else None
            ),
            "audio_path": str(self.audio_path) if self.audio_path else None,
            "timestamps_path": (
                str(self.timestamps_path) if self.timestamps_path else None
            ),
            "duration_sec": self.duration_sec,
            "was_regenerated": self.was_regenerated,
            "error_message": self.error_message,
        }

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "TextEntry":
        """Create from dictionary (JSON deserialization)."""
        return cls(
            id=data["id"],
            original_text=data["original_text"],
            normalized_text=data.get("normalized_text"),
            status=EntryStatus(data["status"]),
            created_at=datetime.fromisoformat(data["created_at"]),
            audio_generated_at=(
                datetime.fromisoformat(data["audio_generated_at"])
                if data.get("audio_generated_at")
                else None
            ),
            audio_path=Path(data["audio_path"]) if data.get("audio_path") else None,
            timestamps_path=(
                Path(data["timestamps_path"]) if data.get("timestamps_path") else None
            ),
            duration_sec=data.get("duration_sec"),
            was_regenerated=data.get("was_regenerated", False),
            error_message=data.get("error_message"),
        )
