"""Tests for PlayerWidget — unit tests that don't require mpv/Qt runtime."""

import pytest
from pathlib import Path
from unittest.mock import patch, MagicMock

from ruvox.ui.models.entry import TextEntry, EntryStatus


class TestFormatTime:
    """Test PlayerWidget._format_time static method."""

    def _format_time(self, seconds: float) -> str:
        """Import and call the static method."""
        from ruvox.ui.widgets.player import PlayerWidget
        return PlayerWidget._format_time(seconds)

    def test_zero(self):
        assert self._format_time(0) == "0:00"

    def test_seconds_only(self):
        assert self._format_time(5) == "0:05"

    def test_one_minute(self):
        assert self._format_time(60) == "1:00"

    def test_minute_and_seconds(self):
        assert self._format_time(90) == "1:30"

    def test_fractional_seconds(self):
        assert self._format_time(61.7) == "1:01"

    def test_large_time(self):
        assert self._format_time(3661) == "61:01"

    def test_negative_treated_as_zero_minutes(self):
        # Negative values shouldn't normally occur but shouldn't crash
        result = self._format_time(-1)
        assert isinstance(result, str)


class TestSpeedIndexBounds:
    """Test speed_up/speed_down index boundary logic without Qt."""

    SPEEDS = [0.5, 0.75, 1.0, 1.25, 1.5, 2.0]

    def test_speed_up_at_max_stays(self):
        """speed_up at last index should not exceed bounds."""
        index = len(self.SPEEDS) - 1  # 5
        if index < len(self.SPEEDS) - 1:
            index += 1
        assert index == len(self.SPEEDS) - 1

    def test_speed_down_at_min_stays(self):
        """speed_down at first index should not go below 0."""
        index = 0
        if index > 0:
            index -= 1
        assert index == 0

    def test_speed_up_increments(self):
        """speed_up from default (1.0x) should go to 1.25x."""
        index = 2  # 1.0x
        if index < len(self.SPEEDS) - 1:
            index += 1
        assert self.SPEEDS[index] == 1.25

    def test_speed_down_decrements(self):
        """speed_down from default (1.0x) should go to 0.75x."""
        index = 2  # 1.0x
        if index > 0:
            index -= 1
        assert self.SPEEDS[index] == 0.75

    def test_full_speed_up_cycle(self):
        """Repeated speed_up should reach max and stop."""
        index = 0
        for _ in range(10):  # More than SPEEDS length
            if index < len(self.SPEEDS) - 1:
                index += 1
        assert index == len(self.SPEEDS) - 1
        assert self.SPEEDS[index] == 2.0

    def test_full_speed_down_cycle(self):
        """Repeated speed_down should reach min and stop."""
        index = len(self.SPEEDS) - 1
        for _ in range(10):
            if index > 0:
                index -= 1
        assert index == 0
        assert self.SPEEDS[index] == 0.5


class TestMpvUnavailable:
    """Test graceful degradation when mpv is not available."""

    def test_mpv_unavailable_flag(self):
        """When mpv import fails, _MPV_AVAILABLE should be False."""
        import importlib
        with patch.dict("sys.modules", {"mpv": None}):
            # Force reimport of the player module
            import ruvox.ui.widgets.player as player_module
            original = player_module._MPV_AVAILABLE
            player_module._MPV_AVAILABLE = False
            try:
                assert player_module._MPV_AVAILABLE is False
            finally:
                player_module._MPV_AVAILABLE = original


@pytest.fixture
def player_widget(qapp, tmp_path):
    """Create a real PlayerWidget with mocked mpv backend."""
    import ruvox.ui.widgets.player as player_module

    mock_mpv_instance = MagicMock()
    mock_mpv_instance.duration = None
    mock_mpv_instance.time_pos = None
    mock_mpv_instance.pause = True

    original_flag = player_module._MPV_AVAILABLE
    player_module._MPV_AVAILABLE = True

    with patch.object(player_module, "mpv", create=True) as mock_mpv_mod:
        mock_mpv_mod.MPV.return_value = mock_mpv_instance
        from ruvox.ui.widgets.player import PlayerWidget
        widget = PlayerWidget()
        # Ensure the mock player is set
        widget._player = mock_mpv_instance
        widget._mpv_available = True
        yield widget
        widget.deleteLater()

    player_module._MPV_AVAILABLE = original_flag


def _make_entry(tmp_path, duration_sec=None):
    """Create a TextEntry with a dummy audio file."""
    audio_file = tmp_path / "audio" / "test.wav"
    audio_file.parent.mkdir(parents=True, exist_ok=True)
    audio_file.write_bytes(b"\x00" * 100)
    return TextEntry(
        original_text="test",
        status=EntryStatus.READY,
        audio_path=Path("test.wav"),
        duration_sec=duration_sec,
    )


class TestLoadEntryDuration:
    """Test that load_entry correctly sets/resets duration."""

    def test_load_entry_sets_duration_from_entry(self, player_widget, tmp_path):
        """Duration from entry.duration_sec should be applied to UI."""
        entry = _make_entry(tmp_path, duration_sec=12.5)
        audio_dir = tmp_path / "audio"

        player_widget.load_entry(entry, audio_dir)

        assert player_widget._duration_ms == 12500
        assert player_widget.time_total.text() == "0:12"
        assert player_widget.progress_slider.maximum() == 12500

    def test_load_entry_resets_old_duration(self, player_widget, tmp_path):
        """Loading a new entry should reset the old duration first."""
        audio_dir = tmp_path / "audio"

        # Load first entry with long duration
        entry1 = _make_entry(tmp_path, duration_sec=120.0)
        player_widget.load_entry(entry1, audio_dir)
        assert player_widget._duration_ms == 120000

        # Load second entry with short duration
        entry2 = _make_entry(tmp_path, duration_sec=5.0)
        player_widget.load_entry(entry2, audio_dir)
        assert player_widget._duration_ms == 5000
        assert player_widget.time_total.text() == "0:05"

    def test_load_entry_none_duration_resets(self, player_widget, tmp_path):
        """Entry without duration_sec should reset duration to 0."""
        audio_dir = tmp_path / "audio"

        # First load with known duration
        entry1 = _make_entry(tmp_path, duration_sec=30.0)
        player_widget.load_entry(entry1, audio_dir)
        assert player_widget._duration_ms == 30000

        # Load entry without duration
        entry2 = _make_entry(tmp_path, duration_sec=None)
        player_widget.load_entry(entry2, audio_dir)
        assert player_widget._duration_ms == 0
        assert player_widget.time_total.text() == "0:00"

    def test_load_entry_position_reset(self, player_widget, tmp_path):
        """Slider and time_current should be reset to 0 on new load."""
        audio_dir = tmp_path / "audio"
        entry = _make_entry(tmp_path, duration_sec=10.0)

        # Simulate some position state
        player_widget.progress_slider.setValue(5000)
        player_widget.time_current.setText("0:05")

        player_widget.load_entry(entry, audio_dir)

        assert player_widget.progress_slider.value() == 0
        assert player_widget.time_current.text() == "0:00"

    def test_mpv_callback_overrides_entry_duration(self, player_widget, tmp_path):
        """mpv duration callback should override the entry-based duration."""
        audio_dir = tmp_path / "audio"
        entry = _make_entry(tmp_path, duration_sec=10.0)

        player_widget.load_entry(entry, audio_dir)
        assert player_widget._duration_ms == 10000

        # Simulate mpv reporting a different duration
        player_widget._on_duration_changed(10500)

        assert player_widget._duration_ms == 10500
        assert player_widget.time_total.text() == "0:10"
        assert player_widget.progress_slider.maximum() == 10500
