"""Timestamp estimation for chunked synthesis."""
from __future__ import annotations

import re

from ttsd.protocol import WordTimestamp


def extract_words_with_positions(text: str) -> list[tuple[str, int, int]]:
    """Extract words with their character positions from text.

    Returns list of (word, start, end) tuples; punctuation is excluded.
    """
    words = []
    for match in re.finditer(r"\b\w+\b", text):
        words.append((match.group(), match.start(), match.end()))
    return words


def estimate_timestamps_chunked(
    text: str,
    chunk_durations: list[tuple[int, int, float]],
    char_mapping: object | None = None,
) -> list[WordTimestamp]:
    """Estimate word-level timestamps for chunked synthesis output.

    Distributes each chunk's duration proportionally by character count across
    words in that chunk.  When char_mapping is provided (see ipc-contract.md
    Layer 3), normalized positions are translated back to original-text offsets.

    Args:
        text: Full normalized text that was synthesized.
        chunk_durations: List of (norm_start, norm_end, duration_sec) per chunk.
        char_mapping: Optional mapping from Rust pipeline; accepted shapes:
            - list[CharMappingEntry] (Pydantic model with norm_start/norm_end/orig_start/orig_end)
            - list of dicts with the same keys
            - dict with key "char_map" containing list[list[int, int]]
            - list[list[int, int]] indexed by normalized position
            When absent, original_pos reflects normalized-text offsets.
    """
    timestamps: list[WordTimestamp] = []
    audio_offset = 0.0

    for chunk_start, chunk_end, chunk_duration in chunk_durations:
        chunk_text = text[chunk_start:chunk_end]
        chunk_words = extract_words_with_positions(chunk_text)

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
            norm_start = chunk_start + word_start_in_chunk
            norm_end = chunk_start + word_end_in_chunk

            if char_mapping is not None:
                orig_start, orig_end = _map_to_original(char_mapping, norm_start, norm_end)
            else:
                orig_start, orig_end = norm_start, norm_end

            timestamps.append(
                WordTimestamp(
                    word=word,
                    start=round(audio_offset + current_time, 3),
                    end=round(audio_offset + current_time + word_duration, 3),
                    original_pos=(orig_start, orig_end),
                )
            )
            current_time += word_duration

        audio_offset += chunk_duration

    return timestamps


def _map_to_original(char_mapping: object, norm_start: int, norm_end: int) -> tuple[int, int]:
    """Map normalized text positions to original text positions via char_mapping.

    Accepts three input shapes for forward compatibility with future callers:
      1. List of CharMappingEntry-like objects (attrs: norm_start, norm_end,
         orig_start, orig_end) — direct span lookup.
      2. Dict with key "char_map" containing list[[orig_start, orig_end]] indexed
         by normalized position.
      3. List[[orig_start, orig_end]] indexed by normalized position (positional
         array).
    """
    # Shape 1: list of span entries (pydantic models or dicts) with norm_start/orig_start attrs
    if isinstance(char_mapping, list) and char_mapping and _is_span_entry(char_mapping[0]):
        return _map_via_spans(char_mapping, norm_start, norm_end)

    # Shape 2: dict wrapper {"char_map": [...]}
    if isinstance(char_mapping, dict) and "char_map" in char_mapping:
        return _map_via_positional(char_mapping["char_map"], norm_start, norm_end)

    # Shape 3: positional list [[orig_start, orig_end], ...]
    if isinstance(char_mapping, list):
        return _map_via_positional(char_mapping, norm_start, norm_end)

    return norm_start, norm_end


def _is_span_entry(entry: object) -> bool:
    """Return True if entry looks like a CharMappingEntry span (has norm_start attr or key)."""
    if isinstance(entry, dict):
        return "norm_start" in entry
    return hasattr(entry, "norm_start")


def _get_attr(entry: object, name: str) -> int:
    if isinstance(entry, dict):
        return int(entry[name])
    return int(getattr(entry, name))


def _map_via_spans(
    spans: list,
    norm_start: int,
    norm_end: int,
) -> tuple[int, int]:
    """Find the span(s) covering [norm_start, norm_end) and return orig bounds."""
    best_start: int | None = None
    best_end: int | None = None

    for span in spans:
        s_norm_start = _get_attr(span, "norm_start")
        s_norm_end = _get_attr(span, "norm_end")

        # Spans that overlap with our target range
        if s_norm_end <= norm_start:
            continue
        if s_norm_start >= norm_end:
            break

        orig_s = _get_attr(span, "orig_start")
        orig_e = _get_attr(span, "orig_end")

        if best_start is None or orig_s < best_start:
            best_start = orig_s
        if best_end is None or orig_e > best_end:
            best_end = orig_e

    if best_start is None:
        return norm_start, norm_end
    return best_start, best_end  # type: ignore[return-value]


def _map_via_positional(char_map: list, norm_start: int, norm_end: int) -> tuple[int, int]:
    """Map using a positional array [[orig_start, orig_end]] indexed by norm position."""
    if not char_map:
        return norm_start, norm_end

    start_idx = max(0, min(norm_start, len(char_map) - 1))
    end_idx = max(0, min(norm_end - 1, len(char_map) - 1))

    orig_start = int(char_map[start_idx][0])
    orig_end = int(char_map[end_idx][1])

    return orig_start, orig_end
