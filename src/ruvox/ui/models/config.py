"""UI configuration model."""

import json
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

CONFIG_VERSION = 1


@dataclass
class UIConfig:
    """Configuration for the UI application.

    Attributes:
        cache_dir: Directory for storing cache files
        hotkey_read_now: Global hotkey for "read now" action
        hotkey_read_later: Global hotkey for "read later" action
        speaker: Silero TTS speaker name
        speech_rate: Speech rate multiplier (0.5 - 2.0)
        sample_rate: Audio sample rate (8000, 24000, 48000)
        history_days: Days to keep text history
        audio_max_files: Maximum number of audio files to keep
        audio_regenerated_hours: Hours to keep regenerated audio
        notify_on_ready: Show notification when TTS is ready (deferred mode)
        notify_on_error: Show notification on TTS error
        text_format: Default text format ("markdown" or "plain")
        player_hotkeys: Keyboard shortcuts for player controls
        window_geometry: Saved window geometry (x, y, width, height)
    """

    cache_dir: Path = field(default_factory=lambda: Path.home() / ".cache" / "ruvox")

    # Global hotkeys
    hotkey_read_now: str = "Control+grave"
    hotkey_read_later: str = "Control+Shift+grave"

    # TTS settings
    speaker: str = "xenia"
    speech_rate: float = 1.0
    sample_rate: int = 48000

    # Cleanup settings
    history_days: int = 14
    audio_max_files: int = 5
    audio_regenerated_hours: int = 24

    # Behavior
    notify_on_ready: bool = True
    notify_on_error: bool = True
    text_format: str = "plain"

    # Player hotkeys (local, in window)
    player_hotkeys: dict[str, str] = field(
        default_factory=lambda: {
            "play_pause": "Space",
            "forward_5": "Right",
            "backward_5": "Left",
            "forward_30": "Shift+Right",
            "backward_30": "Shift+Left",
            "speed_up": "]",
            "speed_down": "[",
            "next_entry": "n",
            "prev_entry": "p",
            "repeat_sentence": "r",
        }
    )

    # Appearance
    theme: str = "dark_pro"

    # Window state
    window_geometry: tuple[int, int, int, int] | None = None  # x, y, width, height

    def to_dict(self) -> dict[str, Any]:
        """Convert to dictionary for JSON serialization."""
        return {
            "version": CONFIG_VERSION,
            "cache_dir": str(self.cache_dir),
            "hotkey_read_now": self.hotkey_read_now,
            "hotkey_read_later": self.hotkey_read_later,
            "speaker": self.speaker,
            "speech_rate": self.speech_rate,
            "sample_rate": self.sample_rate,
            "history_days": self.history_days,
            "audio_max_files": self.audio_max_files,
            "audio_regenerated_hours": self.audio_regenerated_hours,
            "notify_on_ready": self.notify_on_ready,
            "notify_on_error": self.notify_on_error,
            "text_format": self.text_format,
            "player_hotkeys": self.player_hotkeys,
            "theme": self.theme,
            "window_geometry": self.window_geometry,
        }

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "UIConfig":
        """Create from dictionary (JSON deserialization)."""
        # Handle version migration if needed
        version = data.get("version", 1)
        if version < CONFIG_VERSION:
            data = cls._migrate_config(data, version)

        return cls(
            cache_dir=Path(data.get("cache_dir", Path.home() / ".cache" / "ruvox")),
            hotkey_read_now=data.get("hotkey_read_now", "Control+grave"),
            hotkey_read_later=data.get("hotkey_read_later", "Control+Shift+grave"),
            speaker=data.get("speaker", "xenia"),
            speech_rate=data.get("speech_rate", 1.0),
            sample_rate=data.get("sample_rate", 48000),
            history_days=data.get("history_days", 14),
            audio_max_files=data.get("audio_max_files", 5),
            audio_regenerated_hours=data.get("audio_regenerated_hours", 24),
            notify_on_ready=data.get("notify_on_ready", True),
            notify_on_error=data.get("notify_on_error", True),
            text_format=data.get("text_format", "markdown"),
            player_hotkeys=data.get("player_hotkeys", cls.__dataclass_fields__["player_hotkeys"].default_factory()),
            theme=data.get("theme", "dark_pro"),
            window_geometry=data.get("window_geometry"),
        )

    @staticmethod
    def _migrate_config(data: dict[str, Any], from_version: int) -> dict[str, Any]:
        """Migrate config from older version."""
        # Add migration logic here when CONFIG_VERSION increases
        return data

    def save(self, path: Path | None = None) -> None:
        """Save config to JSON file."""
        if path is None:
            path = self.cache_dir / "config.json"
        path.parent.mkdir(parents=True, exist_ok=True)
        with open(path, "w", encoding="utf-8") as f:
            json.dump(self.to_dict(), f, indent=2, ensure_ascii=False)

    @classmethod
    def load(cls, path: Path) -> "UIConfig":
        """Load config from JSON file."""
        if not path.exists():
            return cls()
        with open(path, encoding="utf-8") as f:
            data = json.load(f)
        return cls.from_dict(data)
