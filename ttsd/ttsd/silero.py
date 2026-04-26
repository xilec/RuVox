"""Silero TTS engine wrapper."""
from __future__ import annotations

import logging
import re
from dataclasses import dataclass
from pathlib import Path

import numpy as np
import torch
from scipy.io import wavfile

from ttsd.protocol import WordTimestamp
from ttsd.timestamps import estimate_timestamps_chunked

logger = logging.getLogger("ttsd.silero")

# Maximum characters per TTS chunk (Silero limit is ~1000-1500)
MAX_CHUNK_SIZE = 900


@dataclass
class SynthesisOutput:
    timestamps: list[WordTimestamp]
    duration_sec: float


class SileroEngine:
    """Silero TTS wrapper. Load once, synthesize many."""

    def __init__(self) -> None:
        self._model: object | None = None
        self._loaded: bool = False

    def is_loaded(self) -> bool:
        return self._loaded

    def load(self) -> None:
        if self._loaded:
            return
        logger.info("Loading Silero model (downloads on first run)...")
        import socket

        old_timeout = socket.getdefaulttimeout()
        socket.setdefaulttimeout(60.0)
        try:
            model, _ = torch.hub.load(
                repo_or_dir="snakers4/silero-models",
                model="silero_tts",
                language="ru",
                speaker="v5_ru",
            )
        finally:
            socket.setdefaulttimeout(old_timeout)
        self._model = model
        self._loaded = True
        logger.info("Silero model loaded")

    def synthesize(
        self,
        text: str,
        speaker: str,
        sample_rate: int,
        out_wav: Path,
        char_mapping: object | None = None,
    ) -> SynthesisOutput:
        if not self._loaded:
            self.load()

        assert self._model is not None

        if not text.strip():
            raise ValueError("text must not be empty")

        chunks = self._split_into_chunks(text)
        logger.debug("Synthesizing %d chars in %d chunks", len(text), len(chunks))

        audio_parts: list[np.ndarray] = []
        chunk_durations: list[tuple[int, int, float]] = []

        for i, (chunk_text, chunk_start) in enumerate(chunks):
            logger.debug("Chunk %d/%d: %d chars", i + 1, len(chunks), len(chunk_text))
            silero_text = self._sanitize_for_silero(chunk_text)
            with torch.no_grad():
                audio = self._model.apply_tts(  # type: ignore[union-attr]
                    text=silero_text,
                    speaker=speaker,
                    sample_rate=sample_rate,
                )
            audio_np = audio.numpy() if isinstance(audio, torch.Tensor) else audio
            audio_parts.append(audio_np)
            chunk_duration = len(audio_np) / sample_rate
            chunk_durations.append((chunk_start, chunk_start + len(chunk_text), chunk_duration))

        full_audio = np.concatenate(audio_parts) if len(audio_parts) > 1 else audio_parts[0]
        duration_sec = len(full_audio) / sample_rate

        out_wav.parent.mkdir(parents=True, exist_ok=True)
        wavfile.write(out_wav, sample_rate, full_audio)
        logger.debug("Wrote %d samples to %s (%.2fs)", len(full_audio), out_wav, duration_sec)

        timestamps = estimate_timestamps_chunked(text, chunk_durations, char_mapping)

        return SynthesisOutput(timestamps=timestamps, duration_sec=duration_sec)

    @staticmethod
    def _split_into_chunks(text: str) -> list[tuple[str, int]]:
        """Split text into chunks for TTS processing.

        Returns list of (chunk_text, start_position) tuples.
        """
        if len(text) <= MAX_CHUNK_SIZE:
            return [(text, 0)]

        chunks: list[tuple[str, int]] = []
        current_pos = 0

        while current_pos < len(text):
            chunk_end = min(current_pos + MAX_CHUNK_SIZE, len(text))

            if chunk_end >= len(text):
                chunks.append((text[current_pos:], current_pos))
                break

            chunk_text = text[current_pos:chunk_end]

            best_split = -1
            for match in re.finditer(r"[.!?]\s+", chunk_text):
                best_split = match.end()

            if best_split == -1:
                for match in re.finditer(r"[,;:]\s+", chunk_text):
                    best_split = match.end()

            if best_split == -1:
                for match in re.finditer(r"\s+", chunk_text):
                    best_split = match.end()

            if best_split == -1 or best_split < len(chunk_text) // 2:
                best_split = MAX_CHUNK_SIZE

            actual_chunk = text[current_pos : current_pos + best_split].strip()
            if actual_chunk:
                chunks.append((actual_chunk, current_pos))

            current_pos += best_split

        return chunks

    @staticmethod
    def _sanitize_for_silero(text: str) -> str:
        """Remove newlines and collapse spaces before passing to Silero.

        Silero's character-level tokenizer does not handle control characters;
        newlines cause a fatal abort inside prepare_tts_model_input.
        """
        text = re.sub(r"\s*\n\s*", " ", text)
        text = re.sub(r" +", " ", text)
        return text.strip()
