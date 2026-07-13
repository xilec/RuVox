"""Torch-free text utilities for Silero synthesis.

Extracted from silero.py so the pure text-processing helpers (chunk splitting,
sanitisation) can be imported and unit-tested without pulling in numpy/torch/scipy.
"""
from __future__ import annotations

import re

# Maximum characters per TTS chunk (Silero limit is ~1000-1500)
MAX_CHUNK_SIZE = 900


def split_into_chunks(text: str) -> list[tuple[str, int]]:
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


def sanitize_for_silero(text: str) -> str:
    """Remove newlines and collapse spaces before passing to Silero.

    Silero's character-level tokenizer does not handle control characters;
    newlines cause a fatal abort inside prepare_tts_model_input.
    """
    text = re.sub(r"\s*\n\s*", " ", text)
    text = re.sub(r" +", " ", text)
    return text.strip()
