"""Tests for StorageService - persistence layer for history and audio files."""

import json
import time
from datetime import datetime
from pathlib import Path

import numpy as np
import pytest

from ruvox.ui.models.config import UIConfig
from ruvox.ui.models.entry import EntryStatus
from ruvox.ui.services.storage import HISTORY_VERSION, StorageService


@pytest.fixture
def sample_timestamps() -> list[dict]:
    """Create sample timestamps data."""
    return [
        {"word": "Hello", "start": 0.0, "end": 0.5, "original_pos": [0, 5]},
        {"word": "World", "start": 0.5, "end": 1.0, "original_pos": [6, 11]},
    ]


# =============================================================================
# 1. Инициализация (3 теста)
# =============================================================================


class TestStorageInitialization:
    """Tests for StorageService initialization."""

    def test_init_creates_cache_directory(self, tmp_path: Path):
        """StorageService should create cache_dir if it doesn't exist."""
        cache_dir = tmp_path / "new_cache" / "nested"
        assert not cache_dir.exists()

        config = UIConfig(cache_dir=cache_dir)
        StorageService(config)

        assert cache_dir.exists()
        assert (cache_dir / "audio").exists()

    def test_init_loads_existing_history(self, temp_cache_dir: Path):
        """StorageService should load entries from existing history.json."""
        # Create history file manually
        history_data = {
            "version": HISTORY_VERSION,
            "entries": [
                {
                    "id": "test-id-123",
                    "original_text": "Test text",
                    "normalized_text": None,
                    "status": "pending",
                    "created_at": datetime.now().isoformat(),
                    "audio_generated_at": None,
                    "audio_path": None,
                    "timestamps_path": None,
                    "duration_sec": None,
                    "was_regenerated": False,
                    "error_message": None,
                },
            ],
        }
        history_file = temp_cache_dir / "history.json"
        with open(history_file, "w", encoding="utf-8") as f:
            json.dump(history_data, f)

        config = UIConfig(cache_dir=temp_cache_dir)
        storage = StorageService(config)

        entry = storage.get_entry("test-id-123")
        assert entry is not None
        assert entry.original_text == "Test text"

    def test_init_empty_cache_dir(self, temp_cache_dir: Path):
        """StorageService should work with empty cache directory."""
        config = UIConfig(cache_dir=temp_cache_dir)
        storage = StorageService(config)

        entries = storage.get_all_entries()
        assert entries == []


# =============================================================================
# 2. CRUD: add_entry (5 тестов)
# =============================================================================


class TestAddEntry:
    """Tests for add_entry method."""

    def test_add_entry_creates_unique_id(self, storage: StorageService):
        """Each added entry should have a unique UUID."""
        entry1 = storage.add_entry("Text 1")
        entry2 = storage.add_entry("Text 2")

        assert entry1.id != entry2.id
        assert len(entry1.id) == 36  # UUID format

    def test_add_entry_strips_bom_character(self, storage: StorageService):
        """BOM character at start should be stripped."""
        text_with_bom = "\ufeffHello World"
        entry = storage.add_entry(text_with_bom)

        assert entry.original_text == "Hello World"
        assert not entry.original_text.startswith("\ufeff")

    def test_add_entry_saves_to_history(self, storage: StorageService, config: UIConfig):
        """Added entry should be persisted to history.json."""
        entry = storage.add_entry("Persistent text")

        # Verify history file exists and contains entry
        history_file = config.cache_dir / "history.json"
        assert history_file.exists()

        with open(history_file, encoding="utf-8") as f:
            data = json.load(f)

        entry_ids = [e["id"] for e in data["entries"]]
        assert entry.id in entry_ids

    def test_add_entry_sets_pending_status(self, storage: StorageService):
        """New entry should have PENDING status."""
        entry = storage.add_entry("New text")

        assert entry.status == EntryStatus.PENDING

    def test_add_entry_sets_creation_timestamp(self, storage: StorageService):
        """New entry should have created_at timestamp."""
        before = datetime.now()
        entry = storage.add_entry("Timestamped text")
        after = datetime.now()

        assert before <= entry.created_at <= after


# =============================================================================
# 3. CRUD: get_entry (2 теста)
# =============================================================================


class TestGetEntry:
    """Tests for get_entry method."""

    def test_get_entry_returns_existing_entry(self, storage: StorageService):
        """get_entry should return entry by ID."""
        entry = storage.add_entry("Find me")
        found = storage.get_entry(entry.id)

        assert found is not None
        assert found.id == entry.id
        assert found.original_text == "Find me"

    def test_get_entry_returns_none_for_missing_id(self, storage: StorageService):
        """get_entry should return None for non-existent ID."""
        result = storage.get_entry("non-existent-id")

        assert result is None


# =============================================================================
# 4. CRUD: update_entry (3 теста)
# =============================================================================


class TestUpdateEntry:
    """Tests for update_entry method."""

    def test_update_entry_modifies_data(self, storage: StorageService):
        """update_entry should modify entry data."""
        entry = storage.add_entry("Original")
        entry.normalized_text = "Modified"
        entry.status = EntryStatus.READY

        storage.update_entry(entry)
        updated = storage.get_entry(entry.id)

        assert updated.normalized_text == "Modified"
        assert updated.status == EntryStatus.READY

    def test_update_entry_persists_to_history(self, storage: StorageService, config: UIConfig):
        """Updated entry should be persisted to history.json."""
        entry = storage.add_entry("Original")
        entry.normalized_text = "Updated for persistence"
        storage.update_entry(entry)

        # Reload from file
        with open(config.cache_dir / "history.json", encoding="utf-8") as f:
            data = json.load(f)

        saved_entry = next(e for e in data["entries"] if e["id"] == entry.id)
        assert saved_entry["normalized_text"] == "Updated for persistence"

    def test_update_entry_preserves_id(self, storage: StorageService):
        """update_entry should preserve the entry ID."""
        entry = storage.add_entry("Test")
        original_id = entry.id
        entry.original_text = "Changed"

        storage.update_entry(entry)

        assert entry.id == original_id
        assert storage.get_entry(original_id) is not None


# =============================================================================
# 5. CRUD: delete_entry (5 тестов)
# =============================================================================


class TestDeleteEntry:
    """Tests for delete_entry method."""

    def test_delete_entry_removes_from_cache(self, storage: StorageService):
        """delete_entry should remove entry from internal cache."""
        entry = storage.add_entry("To delete")
        entry_id = entry.id

        storage.delete_entry(entry_id)

        assert storage.get_entry(entry_id) is None

    def test_delete_entry_removes_audio_file(self, storage: StorageService, mock_audio_data: np.ndarray):
        """delete_entry should remove associated audio file."""
        entry = storage.add_entry("With audio")
        audio_path = storage.save_audio(entry.id, mock_audio_data, 48000)
        entry.audio_path = audio_path
        storage.update_entry(entry)

        full_audio_path = storage.audio_dir / audio_path
        assert full_audio_path.exists()

        storage.delete_entry(entry.id)

        assert not full_audio_path.exists()

    def test_delete_entry_removes_timestamps_file(self, storage: StorageService, sample_timestamps: list[dict]):
        """delete_entry should remove associated timestamps file."""
        entry = storage.add_entry("With timestamps")
        ts_path = storage.save_timestamps(entry.id, sample_timestamps)
        entry.timestamps_path = ts_path
        storage.update_entry(entry)

        full_ts_path = storage.audio_dir / ts_path
        assert full_ts_path.exists()

        storage.delete_entry(entry.id)

        assert not full_ts_path.exists()

    def test_delete_entry_persists_to_history(self, storage: StorageService, config: UIConfig):
        """delete_entry should update history.json."""
        entry = storage.add_entry("To persist delete")
        entry_id = entry.id

        storage.delete_entry(entry_id)

        with open(config.cache_dir / "history.json", encoding="utf-8") as f:
            data = json.load(f)

        entry_ids = [e["id"] for e in data["entries"]]
        assert entry_id not in entry_ids

    def test_delete_entry_handles_missing_audio_gracefully(self, storage: StorageService):
        """delete_entry should not fail if audio file is already missing."""
        entry = storage.add_entry("No audio")
        entry.audio_path = Path("missing.wav")
        storage.update_entry(entry)

        # Should not raise exception
        storage.delete_entry(entry.id)

        assert storage.get_entry(entry.id) is None


# =============================================================================
# 6. CRUD: delete_audio (4 теста)
# =============================================================================


class TestDeleteAudio:
    """Tests for delete_audio method."""

    def test_delete_audio_removes_wav_file(self, storage: StorageService, mock_audio_data: np.ndarray):
        """delete_audio should remove the WAV file."""
        entry = storage.add_entry("Audio to delete")
        audio_path = storage.save_audio(entry.id, mock_audio_data, 48000)
        entry.audio_path = audio_path
        entry.status = EntryStatus.READY
        storage.update_entry(entry)

        full_path = storage.audio_dir / audio_path
        assert full_path.exists()

        storage.delete_audio(entry.id)

        assert not full_path.exists()

    def test_delete_audio_removes_timestamps_file(
        self,
        storage: StorageService,
        mock_audio_data: np.ndarray,
        sample_timestamps: list[dict],
    ):
        """delete_audio should remove timestamps file too."""
        entry = storage.add_entry("Audio with timestamps")
        audio_path = storage.save_audio(entry.id, mock_audio_data, 48000)
        ts_path = storage.save_timestamps(entry.id, sample_timestamps)
        entry.audio_path = audio_path
        entry.timestamps_path = ts_path
        entry.status = EntryStatus.READY
        storage.update_entry(entry)

        full_ts_path = storage.audio_dir / ts_path
        assert full_ts_path.exists()

        storage.delete_audio(entry.id)

        assert not full_ts_path.exists()

    def test_delete_audio_resets_status_to_pending(self, storage: StorageService, mock_audio_data: np.ndarray):
        """delete_audio should reset entry status to PENDING."""
        entry = storage.add_entry("Ready entry")
        audio_path = storage.save_audio(entry.id, mock_audio_data, 48000)
        entry.audio_path = audio_path
        entry.status = EntryStatus.READY
        storage.update_entry(entry)

        storage.delete_audio(entry.id)

        updated = storage.get_entry(entry.id)
        assert updated.status == EntryStatus.PENDING

    def test_delete_audio_clears_metadata(self, storage: StorageService, mock_audio_data: np.ndarray):
        """delete_audio should clear audio-related metadata."""
        entry = storage.add_entry("Entry with metadata")
        audio_path = storage.save_audio(entry.id, mock_audio_data, 48000)
        entry.audio_path = audio_path
        entry.duration_sec = 5.0
        entry.audio_generated_at = datetime.now()
        entry.status = EntryStatus.READY
        storage.update_entry(entry)

        storage.delete_audio(entry.id)

        updated = storage.get_entry(entry.id)
        assert updated.audio_path is None
        assert updated.timestamps_path is None
        assert updated.duration_sec is None
        assert updated.audio_generated_at is None


# =============================================================================
# 7. CRUD: get_all_entries (3 теста)
# =============================================================================


class TestGetAllEntries:
    """Tests for get_all_entries method."""

    def test_get_all_entries_returns_all(self, storage: StorageService):
        """get_all_entries should return all entries."""
        storage.add_entry("Entry 1")
        storage.add_entry("Entry 2")
        storage.add_entry("Entry 3")

        entries = storage.get_all_entries()

        assert len(entries) == 3

    def test_get_all_entries_sorted_newest_first(self, storage: StorageService):
        """get_all_entries should return entries sorted by created_at (newest first)."""
        entry1 = storage.add_entry("First")
        time.sleep(0.01)  # Small delay to ensure different timestamps
        entry2 = storage.add_entry("Second")
        time.sleep(0.01)
        entry3 = storage.add_entry("Third")

        entries = storage.get_all_entries()

        assert entries[0].id == entry3.id
        assert entries[1].id == entry2.id
        assert entries[2].id == entry1.id

    def test_get_all_entries_empty_on_fresh_cache(self, storage: StorageService):
        """get_all_entries should return empty list on fresh storage."""
        entries = storage.get_all_entries()

        assert entries == []


# =============================================================================
# 8. Аудио операции (8 тестов)
# =============================================================================


class TestAudioOperations:
    """Tests for audio file operations."""

    def test_save_audio_creates_wav_file(self, storage: StorageService, mock_audio_data: np.ndarray):
        """save_audio should create a WAV file."""
        entry = storage.add_entry("Audio test")
        audio_path = storage.save_audio(entry.id, mock_audio_data, 48000)

        full_path = storage.audio_dir / audio_path
        assert full_path.exists()
        assert full_path.suffix == ".wav"

    def test_save_audio_returns_relative_path(self, storage: StorageService, mock_audio_data: np.ndarray):
        """save_audio should return path relative to audio_dir."""
        entry = storage.add_entry("Relative path test")
        audio_path = storage.save_audio(entry.id, mock_audio_data, 48000)

        assert str(audio_path) == f"{entry.id}.wav"
        assert not audio_path.is_absolute()

    def test_save_audio_saves_correct_sample_rate(self, storage: StorageService, mock_audio_data: np.ndarray):
        """save_audio should save with correct sample rate."""
        from scipy.io import wavfile

        entry = storage.add_entry("Sample rate test")
        sample_rate = 24000
        audio_path = storage.save_audio(entry.id, mock_audio_data, sample_rate)

        full_path = storage.audio_dir / audio_path
        saved_rate, _ = wavfile.read(full_path)

        assert saved_rate == sample_rate

    def test_save_timestamps_creates_json_file(self, storage: StorageService, sample_timestamps: list[dict]):
        """save_timestamps should create a JSON file."""
        entry = storage.add_entry("Timestamps test")
        ts_path = storage.save_timestamps(entry.id, sample_timestamps)

        full_path = storage.audio_dir / ts_path
        assert full_path.exists()
        assert full_path.suffix == ".json"
        assert ".timestamps." in str(ts_path)

    def test_save_timestamps_preserves_data_structure(self, storage: StorageService, sample_timestamps: list[dict]):
        """save_timestamps should preserve the timestamps data structure."""
        entry = storage.add_entry("Structure test")
        ts_path = storage.save_timestamps(entry.id, sample_timestamps)

        with open(storage.audio_dir / ts_path, encoding="utf-8") as f:
            data = json.load(f)

        assert "words" in data
        assert len(data["words"]) == 2
        assert data["words"][0]["word"] == "Hello"
        assert data["words"][1]["original_pos"] == [6, 11]

    def test_load_timestamps_returns_words_list(self, storage: StorageService, sample_timestamps: list[dict]):
        """load_timestamps should return the words list."""
        entry = storage.add_entry("Load test")
        ts_path = storage.save_timestamps(entry.id, sample_timestamps)
        entry.timestamps_path = ts_path
        storage.update_entry(entry)

        loaded = storage.load_timestamps(entry.id)

        assert loaded is not None
        assert len(loaded) == 2
        assert loaded[0]["word"] == "Hello"

    def test_load_timestamps_returns_none_for_missing_entry(self, storage: StorageService):
        """load_timestamps should return None for non-existent entry."""
        result = storage.load_timestamps("non-existent-id")

        assert result is None

    def test_get_audio_path_returns_full_path(self, storage: StorageService, mock_audio_data: np.ndarray):
        """get_audio_path should return full absolute path."""
        entry = storage.add_entry("Path test")
        audio_path = storage.save_audio(entry.id, mock_audio_data, 48000)
        entry.audio_path = audio_path
        storage.update_entry(entry)

        full_path = storage.get_audio_path(entry.id)

        assert full_path is not None
        assert full_path.is_absolute()
        assert full_path.exists()


# =============================================================================
# 9. Статистика (4 теста)
# =============================================================================


class TestStatistics:
    """Tests for cache statistics methods."""

    def test_get_cache_size_sums_all_files(
        self,
        storage: StorageService,
        mock_audio_data: np.ndarray,
        sample_timestamps: list[dict],
    ):
        """get_cache_size should sum sizes of all files in audio_dir."""
        entry = storage.add_entry("Size test")
        storage.save_audio(entry.id, mock_audio_data, 48000)
        storage.save_timestamps(entry.id, sample_timestamps)

        size = storage.get_cache_size()

        assert size > 0

    def test_get_cache_size_empty_cache(self, storage: StorageService):
        """get_cache_size should return 0 for empty cache."""
        size = storage.get_cache_size()

        assert size == 0

    def test_get_audio_count_counts_wav_files(self, storage: StorageService, mock_audio_data: np.ndarray):
        """get_audio_count should count WAV files only."""
        entry1 = storage.add_entry("Audio 1")
        entry2 = storage.add_entry("Audio 2")
        storage.save_audio(entry1.id, mock_audio_data, 48000)
        storage.save_audio(entry2.id, mock_audio_data, 48000)
        storage.save_timestamps(entry1.id, [])  # JSON file, not WAV

        count = storage.get_audio_count()

        assert count == 2

    def test_get_audio_count_empty_cache(self, storage: StorageService):
        """get_audio_count should return 0 for empty cache."""
        count = storage.get_audio_count()

        assert count == 0


# =============================================================================
# 10. Валидация и загрузка (6 тестов)
# =============================================================================


class TestValidationAndLoading:
    """Tests for validation and history loading."""

    def test_validate_entry_status_ready_when_audio_exists(self, temp_cache_dir: Path, mock_audio_data: np.ndarray):
        """Entry should be READY if audio file exists."""
        from scipy.io import wavfile

        # Create audio file manually
        audio_dir = temp_cache_dir / "audio"
        audio_dir.mkdir(parents=True, exist_ok=True)
        audio_file = audio_dir / "test-id.wav"
        wavfile.write(audio_file, 48000, mock_audio_data)

        # Create history with entry pointing to this audio
        history_data = {
            "version": HISTORY_VERSION,
            "entries": [
                {
                    "id": "test-id",
                    "original_text": "Test",
                    "normalized_text": None,
                    "status": "pending",  # Wrong status
                    "created_at": datetime.now().isoformat(),
                    "audio_generated_at": None,
                    "audio_path": "test-id.wav",
                    "timestamps_path": None,
                    "duration_sec": None,
                    "was_regenerated": False,
                    "error_message": None,
                },
            ],
        }
        with open(temp_cache_dir / "history.json", "w", encoding="utf-8") as f:
            json.dump(history_data, f)

        config = UIConfig(cache_dir=temp_cache_dir)
        storage = StorageService(config)

        entry = storage.get_entry("test-id")
        assert entry.status == EntryStatus.READY

    def test_validate_entry_status_pending_when_audio_deleted(self, temp_cache_dir: Path):
        """Entry should be PENDING if audio file is missing."""
        # Create history with entry pointing to non-existent audio
        (temp_cache_dir / "audio").mkdir(parents=True, exist_ok=True)
        history_data = {
            "version": HISTORY_VERSION,
            "entries": [
                {
                    "id": "test-id",
                    "original_text": "Test",
                    "normalized_text": None,
                    "status": "ready",  # Wrong - audio doesn't exist
                    "created_at": datetime.now().isoformat(),
                    "audio_generated_at": None,
                    "audio_path": "missing.wav",
                    "timestamps_path": None,
                    "duration_sec": None,
                    "was_regenerated": False,
                    "error_message": None,
                },
            ],
        }
        with open(temp_cache_dir / "history.json", "w", encoding="utf-8") as f:
            json.dump(history_data, f)

        config = UIConfig(cache_dir=temp_cache_dir)
        storage = StorageService(config)

        entry = storage.get_entry("test-id")
        assert entry.status == EntryStatus.PENDING
        assert entry.audio_path is None

    def test_load_history_handles_missing_file(self, temp_cache_dir: Path):
        """StorageService should handle missing history.json gracefully."""
        config = UIConfig(cache_dir=temp_cache_dir)
        storage = StorageService(config)

        assert storage.get_all_entries() == []

    def test_load_history_parses_json_correctly(self, temp_cache_dir: Path):
        """StorageService should parse history.json correctly."""
        history_data = {
            "version": HISTORY_VERSION,
            "entries": [
                {
                    "id": "entry-1",
                    "original_text": "Привет мир",
                    "normalized_text": "привет мир",
                    "status": "pending",
                    "created_at": "2024-01-15T10:30:00",
                    "audio_generated_at": None,
                    "audio_path": None,
                    "timestamps_path": None,
                    "duration_sec": None,
                    "was_regenerated": False,
                    "error_message": None,
                },
            ],
        }
        with open(temp_cache_dir / "history.json", "w", encoding="utf-8") as f:
            json.dump(history_data, f, ensure_ascii=False)

        config = UIConfig(cache_dir=temp_cache_dir)
        storage = StorageService(config)

        entry = storage.get_entry("entry-1")
        assert entry.original_text == "Привет мир"
        assert entry.normalized_text == "привет мир"

    def test_load_history_handles_corrupted_json(self, temp_cache_dir: Path):
        """StorageService should handle corrupted history.json."""
        # Write invalid JSON
        with open(temp_cache_dir / "history.json", "w", encoding="utf-8") as f:
            f.write("{invalid json content")

        config = UIConfig(cache_dir=temp_cache_dir)
        storage = StorageService(config)

        # Should recover gracefully
        assert storage.get_all_entries() == []
        # Backup should be created
        assert (temp_cache_dir / "history.json.bak").exists()

    def test_save_history_creates_json_file(self, storage: StorageService, config: UIConfig):
        """save_history should create properly formatted JSON."""
        storage.add_entry("Test entry")

        history_file = config.cache_dir / "history.json"
        assert history_file.exists()

        with open(history_file, encoding="utf-8") as f:
            data = json.load(f)

        assert "version" in data
        assert "entries" in data
        assert data["version"] == HISTORY_VERSION


# =============================================================================
# 11. Интеграционные (2 теста)
# =============================================================================


class TestIntegration:
    """Integration tests for complete workflows."""

    def test_full_workflow_add_update_delete(
        self,
        storage: StorageService,
        mock_audio_data: np.ndarray,
        sample_timestamps: list[dict],
    ):
        """Test complete workflow: add -> update with audio -> delete."""
        # Add entry
        entry = storage.add_entry("Integration test text")
        assert entry.status == EntryStatus.PENDING

        # Save audio and timestamps
        audio_path = storage.save_audio(entry.id, mock_audio_data, 48000)
        ts_path = storage.save_timestamps(entry.id, sample_timestamps)

        # Update entry with audio info
        entry.audio_path = audio_path
        entry.timestamps_path = ts_path
        entry.status = EntryStatus.READY
        entry.duration_sec = 1.0
        entry.audio_generated_at = datetime.now()
        storage.update_entry(entry)

        # Verify
        retrieved = storage.get_entry(entry.id)
        assert retrieved.status == EntryStatus.READY
        assert storage.get_audio_path(entry.id).exists()
        assert storage.load_timestamps(entry.id) is not None

        # Delete
        storage.delete_entry(entry.id)
        assert storage.get_entry(entry.id) is None
        assert storage.get_audio_count() == 0

    def test_persistence_across_instances(
        self,
        config: UIConfig,
        mock_audio_data: np.ndarray,
        sample_timestamps: list[dict],
    ):
        """Data should persist across StorageService instances."""
        # First instance: create data
        storage1 = StorageService(config)
        entry = storage1.add_entry("Persistent data")
        audio_path = storage1.save_audio(entry.id, mock_audio_data, 48000)
        ts_path = storage1.save_timestamps(entry.id, sample_timestamps)
        entry.audio_path = audio_path
        entry.timestamps_path = ts_path
        entry.status = EntryStatus.READY
        storage1.update_entry(entry)

        # Second instance: verify data persists
        storage2 = StorageService(config)
        loaded = storage2.get_entry(entry.id)

        assert loaded is not None
        assert loaded.original_text == "Persistent data"
        assert loaded.status == EntryStatus.READY
        assert storage2.get_audio_path(entry.id).exists()
        assert storage2.load_timestamps(entry.id) is not None
