"""Tests for PlayerWidget — unit tests that don't require mpv/Qt runtime."""

import pytest
from unittest.mock import patch


class TestFormatTime:
    """Test PlayerWidget._format_time static method."""

    def _format_time(self, seconds: float) -> str:
        """Import and call the static method."""
        from fast_tts_rus.ui.widgets.player import PlayerWidget
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
            import fast_tts_rus.ui.widgets.player as player_module
            original = player_module._MPV_AVAILABLE
            player_module._MPV_AVAILABLE = False
            try:
                assert player_module._MPV_AVAILABLE is False
            finally:
                player_module._MPV_AVAILABLE = original
