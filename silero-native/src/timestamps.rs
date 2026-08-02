//! Word-level timestamps from the model's own duration predictor.
//!
//! `tts_main` exports `dur_hat`: per-symbol durations in 600-sample frames
//! (12.5 ms @ 48 kHz), already carrying the sos/eos clamps baked into the
//! graph. The engine converts them to the exact integer frame counts the
//! graph renders (`trunc(dur + 0.5)`) and pairs each with the char the
//! symbol was emitted from ([`SymbolDuration`]). This module aligns the
//! words of the original chunk text to that symbol stream letter-by-letter
//! and converts the matched frame ranges to seconds.

use serde::{Deserialize, Serialize};
use tracing::debug;

/// Duration of one `dur_hat` frame in seconds (600 samples @ 48 kHz).
const FRAME_SEC: f32 = 600.0 / 48000.0;

/// Word-level timestamp, mirroring `ttsd.protocol.WordTimestamp`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WordTimestamp {
    pub word: String,
    pub start: f32,
    pub end: f32,
    /// Char offsets of the word in the text passed to `synthesize`
    /// (already the "original" positions at this layer; mapping back through
    /// the normalization pipeline is the caller's job, as in ttsd).
    pub original_pos: (usize, usize),
}

/// One input symbol of `tts_main` and the exact frame count the exported
/// graph rendered for it (`trunc(dur_hat + 0.5)`; sos/eos included).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SymbolDuration {
    pub ch: char,
    pub frames: u32,
}

/// One synthesized chunk for [`timestamps_from_durations`].
pub struct ChunkTiming<'a> {
    /// Chunk text in synthesized-text coordinates (what `synthesize` got).
    pub text: &'a str,
    /// Char offset of the chunk in the full synthesized text.
    pub text_offset: usize,
    /// Actual audio duration of the chunk in seconds.
    pub duration_sec: f32,
    /// Per-symbol frame counts aligned with the model input sequence
    /// (sos/eos included).
    pub durations: &'a [SymbolDuration],
}

/// `\b\w+\b` equivalent: maximal runs of alphanumeric chars or `_`.
/// Positions are char indices, like Python's `match.start()`.
fn extract_words_with_positions(text: &str) -> Vec<(String, usize, usize)> {
    let chars: Vec<char> = text.chars().collect();
    let mut words = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_alphanumeric() || chars[i] == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            words.push((chars[start..i].iter().collect(), start, i));
        } else {
            i += 1;
        }
    }
    words
}

/// Round to milliseconds, as ttsd does (`round(x, 3)`).
fn round3(x: f32) -> f32 {
    (x * 1000.0).round() / 1000.0
}

/// Case- and ё-insensitive letter key for matching original-text words
/// against the symbol stream: the frontend lowercases, and the accentor may
/// substitute `е`↔`ё`, so both sides compare through this.
fn letter_key(c: char) -> char {
    let lower = c.to_lowercase().next().unwrap_or(c);
    if lower == 'ё' { 'е' } else { lower }
}

/// Compute word timestamps for all chunks. Per chunk, words are aligned to
/// the symbol stream in order; the audio timeline then advances by the
/// actual chunk duration, exactly as ttsd's chunked estimator does.
pub fn timestamps_from_durations(chunks: &[ChunkTiming]) -> Vec<WordTimestamp> {
    let mut timestamps = Vec::new();
    let mut audio_offset = 0.0f32;
    for chunk in chunks {
        align_chunk(chunk, audio_offset, &mut timestamps);
        audio_offset += chunk.duration_sec.max(0.0);
    }
    timestamps
}

/// End index (exclusive) of a `+` run starting at `i` when the run is
/// directly attached to a following letter — an accentor stress marker
/// (`+я`, `сто+ю`). A standalone literal `+` (surrounded by spaces, as in
/// "правило + команда") is not attached and returns `None`.
fn attached_plus_run_end(durations: &[SymbolDuration], i: usize) -> Option<usize> {
    if durations.get(i)?.ch != '+' {
        return None;
    }
    let mut end = i;
    while durations.get(end).is_some_and(|sd| sd.ch == '+') {
        end += 1;
    }
    if durations.get(end).is_some_and(|sd| sd.ch.is_alphabetic()) {
        Some(end)
    } else {
        None
    }
}

/// Align one chunk's words to its symbol stream.
///
/// The stream cursor only moves forward, so timestamps are monotonic by
/// construction. Symbols that are not letters (`^` markers, punctuation,
/// spaces, sos/eos, standalone `+`) are skipped during matching; their
/// frames still count through the cumulative timeline, so pauses surface as
/// gaps between words and markers inside a word are covered by its range.
/// An attached `+` run directly ahead of the word's first letter opens the
/// word's range: the accentor emits stress markers immediately before the
/// stressed vowel and the model renders real audio frames for them (the
/// audible onset includes them).
///
/// The stream letters are an in-order subsequence of the original text's
/// letters: the frontend only drops characters it cannot map (latin,
/// digits, symbols) and never reorders or drops Cyrillic. A mismatched word
/// letter was therefore dropped before synthesis — it is skipped without
/// consuming the stream symbol, and the cursor only advances on matches, so
/// one unmapped word (e.g. "get_variablesслэш") cannot misalign the rest of
/// the chunk. A word with no matched letters at all (pure latin, digits)
/// gets a zero-length timestamp at the cursor.
fn align_chunk(chunk: &ChunkTiming, audio_offset: f32, out: &mut Vec<WordTimestamp>) {
    // cum[i] = start time of symbol i in seconds; cum[len] = total.
    let mut cum = Vec::with_capacity(chunk.durations.len() + 1);
    cum.push(0.0f32);
    for sd in chunk.durations {
        cum.push(cum.last().copied().unwrap_or(0.0) + sd.frames as f32 * FRAME_SEC);
    }

    let mut cursor = 0usize;
    for (word, start, end) in extract_words_with_positions(chunk.text) {
        let mut first_match: Option<usize> = None;
        let mut last_match: Option<usize> = None;
        for wc in word.chars() {
            if !wc.is_alphabetic() {
                // Digits / '_' can appear in extracted words but never in the
                // model alphabet — they match nothing by construction.
                continue;
            }
            let seeking_first = last_match.is_none();
            // Skip non-letter symbols between/inside words. While seeking the
            // word's first letter, stop at an attached `+` run — it belongs
            // to this word. Mid-word `+` is skipped (already inside the
            // range via the cumulative timeline).
            while cursor < chunk.durations.len() {
                if chunk.durations[cursor].ch.is_alphabetic() {
                    break;
                }
                if seeking_first && attached_plus_run_end(chunk.durations, cursor).is_some() {
                    break;
                }
                cursor += 1;
            }
            // Fold an attached `+` run into the word's range.
            if seeking_first {
                if let Some(run_end) = attached_plus_run_end(chunk.durations, cursor) {
                    first_match.get_or_insert(cursor);
                    cursor = run_end;
                }
            }
            match chunk.durations.get(cursor) {
                Some(sd) if letter_key(sd.ch) == letter_key(wc) => {
                    first_match.get_or_insert(cursor);
                    last_match = Some(cursor);
                    cursor += 1;
                }
                Some(sd) => {
                    // Mismatch: this letter never reached the stream (the
                    // frontend dropped it — latin, digits, symbols). Skip the
                    // letter and keep the stream symbol for the next one;
                    // consuming or stopping here cascades the misalignment
                    // into every following word.
                    debug!(
                        word = %word,
                        stream_char = %sd.ch,
                        word_char = %wc,
                        "skipping letter absent from the symbol stream"
                    );
                    continue;
                }
                None => break,
            }
        }

        let (local_start, local_end) = match (first_match, last_match) {
            (Some(first), Some(last)) => (cum[first], cum[last + 1]),
            // Unspoken word: zero-length where the next spoken content
            // starts (an attached `+` run belongs to the next word, so the
            // zero-length lands no later than that word's start).
            _ => {
                while cursor < chunk.durations.len()
                    && !chunk.durations[cursor].ch.is_alphabetic()
                    && attached_plus_run_end(chunk.durations, cursor).is_none()
                {
                    cursor += 1;
                }
                (cum[cursor], cum[cursor])
            }
        };
        let ts_start = round3(audio_offset + local_start);
        let ts_end = round3((audio_offset + local_end).min(audio_offset + chunk.duration_sec));
        out.push(WordTimestamp {
            word,
            start: ts_start,
            // Clamping to the chunk duration must not invert the range.
            end: ts_end.max(ts_start),
            original_pos: (chunk.text_offset + start, chunk.text_offset + end),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sd(ch: char, frames: u32) -> SymbolDuration {
        SymbolDuration { ch, frames }
    }

    /// sos + "аб в" + eos, one frame per symbol except sos (2).
    fn simple_durations() -> Vec<SymbolDuration> {
        vec![
            sd('|', 2),
            sd('а', 1),
            sd('б', 1),
            sd(' ', 1),
            sd('в', 1),
            sd('~', 3),
        ]
    }

    #[test]
    fn words_split_on_punctuation() {
        let words = extract_words_with_positions("привет, мир! get_user");
        let texts: Vec<&str> = words.iter().map(|(w, _, _)| w.as_str()).collect();
        assert_eq!(texts, vec!["привет", "мир", "get_user"]);
        assert_eq!(words[0].1..words[0].2, 0..6);
    }

    #[test]
    fn word_ranges_follow_symbol_frames() {
        let durations = simple_durations();
        let ts = timestamps_from_durations(&[ChunkTiming {
            text: "аб в",
            text_offset: 0,
            duration_sec: 9.0 * FRAME_SEC,
            durations: &durations,
        }]);
        assert_eq!(ts.len(), 2);
        // "аб" starts after sos (2 frames) and spans 2 frames.
        assert!((ts[0].start - 2.0 * FRAME_SEC).abs() < 1e-3);
        assert!((ts[0].end - 4.0 * FRAME_SEC).abs() < 1e-3);
        // "в" starts after the space frame; eos is excluded from its end.
        assert!((ts[1].start - 5.0 * FRAME_SEC).abs() < 1e-3);
        assert!((ts[1].end - 6.0 * FRAME_SEC).abs() < 1e-3);
        assert_eq!(ts[1].original_pos, (3, 4));
    }

    #[test]
    fn punctuation_pause_becomes_a_gap() {
        let durations = vec![
            sd('|', 1),
            sd('а', 2),
            sd(',', 4),
            sd(' ', 1),
            sd('б', 2),
            sd('~', 1),
        ];
        let ts = timestamps_from_durations(&[ChunkTiming {
            text: "а, б",
            text_offset: 0,
            duration_sec: 11.0 * FRAME_SEC,
            durations: &durations,
        }]);
        assert_eq!(ts.len(), 2);
        // The comma's 4 frames belong to no word.
        assert!((ts[0].end - 3.0 * FRAME_SEC).abs() < 1e-3);
        assert!((ts[1].start - 8.0 * FRAME_SEC).abs() < 1e-3);
        assert!(ts[1].start > ts[0].end);
    }

    #[test]
    fn stress_markers_are_covered_by_the_word_range() {
        // Accentor output "а+б": '+' is not a letter but sits inside the
        // word — the word range spans it, with no gap.
        let durations = vec![sd('|', 1), sd('а', 1), sd('+', 1), sd('б', 1), sd('~', 1)];
        let ts = timestamps_from_durations(&[ChunkTiming {
            text: "аб",
            text_offset: 0,
            duration_sec: 5.0 * FRAME_SEC,
            durations: &durations,
        }]);
        assert_eq!(ts.len(), 1);
        assert!((ts[0].start - FRAME_SEC).abs() < 1e-3);
        assert!((ts[0].end - 4.0 * FRAME_SEC).abs() < 1e-3);
    }

    #[test]
    fn yo_matches_e() {
        // Accentor replaced 'е' with 'ё' in the stream.
        let durations = vec![sd('|', 1), sd('с', 1), sd('ё', 1), sd('л', 1), sd('~', 1)];
        let ts = timestamps_from_durations(&[ChunkTiming {
            text: "сел",
            text_offset: 0,
            duration_sec: 5.0 * FRAME_SEC,
            durations: &durations,
        }]);
        assert_eq!(ts.len(), 1);
        assert!((ts[0].end - 4.0 * FRAME_SEC).abs() < 1e-3);
    }

    #[test]
    fn uppercase_word_matches_lowercase_stream() {
        let durations = vec![sd('|', 1), sd('а', 1), sd('б', 1), sd('~', 1)];
        let ts = timestamps_from_durations(&[ChunkTiming {
            text: "АБ",
            text_offset: 0,
            duration_sec: 4.0 * FRAME_SEC,
            durations: &durations,
        }]);
        assert_eq!(ts.len(), 1);
        assert!((ts[0].end - 3.0 * FRAME_SEC).abs() < 1e-3);
    }

    #[test]
    fn unspoken_word_gets_zero_length_at_cursor() {
        // "123" was filtered out before the model; "аб" follows in the stream.
        let durations = simple_durations();
        let ts = timestamps_from_durations(&[ChunkTiming {
            text: "123 аб в",
            text_offset: 0,
            duration_sec: 9.0 * FRAME_SEC,
            durations: &durations,
        }]);
        assert_eq!(ts.len(), 3);
        assert_eq!(ts[0].start, ts[0].end);
        assert!((ts[0].start - 2.0 * FRAME_SEC).abs() < 1e-3);
        // The following words still align to their own symbols.
        assert!((ts[1].start - 2.0 * FRAME_SEC).abs() < 1e-3);
        assert!((ts[1].end - 4.0 * FRAME_SEC).abs() < 1e-3);
        assert_eq!(ts[1].original_pos, (4, 6));
    }

    #[test]
    fn mid_word_mismatch_truncates_at_last_matched_letter() {
        // "война2024": only "война" reached the model.
        let durations = vec![
            sd('|', 1),
            sd('в', 1),
            sd('о', 1),
            sd('й', 1),
            sd('н', 1),
            sd('а', 1),
            sd('~', 1),
        ];
        let ts = timestamps_from_durations(&[ChunkTiming {
            text: "война2024",
            text_offset: 0,
            duration_sec: 7.0 * FRAME_SEC,
            durations: &durations,
        }]);
        assert_eq!(ts.len(), 1);
        assert!((ts[0].start - FRAME_SEC).abs() < 1e-3);
        assert!((ts[0].end - 6.0 * FRAME_SEC).abs() < 1e-3);
        // The position still covers the full original word.
        assert_eq!(ts[0].original_pos, (0, 9));
    }

    #[test]
    fn zero_frame_symbols_keep_words_ordered() {
        // Model assigned zero frames to 'б' and ','.
        let durations = vec![
            sd('|', 1),
            sd('а', 1),
            sd('б', 0),
            sd(',', 0),
            sd(' ', 1),
            sd('в', 1),
            sd('~', 1),
        ];
        let ts = timestamps_from_durations(&[ChunkTiming {
            text: "аб, в",
            text_offset: 0,
            duration_sec: 5.0 * FRAME_SEC,
            durations: &durations,
        }]);
        assert_eq!(ts.len(), 2);
        for pair in ts.windows(2) {
            assert!(pair[0].end <= pair[1].start);
            assert!(pair[0].start <= pair[0].end);
        }
        // 'б' has zero frames: word "аб" ends where 'а' ends.
        assert!((ts[0].end - 2.0 * FRAME_SEC).abs() < 1e-3);
    }

    #[test]
    fn end_is_clamped_to_chunk_duration() {
        // Frames sum beyond the actual (PQMF-rounded) audio duration.
        let durations = simple_durations();
        let ts = timestamps_from_durations(&[ChunkTiming {
            text: "аб в",
            text_offset: 0,
            duration_sec: 5.5 * FRAME_SEC,
            durations: &durations,
        }]);
        let last = ts.last().expect("timestamps");
        assert!(last.end <= 5.5 * FRAME_SEC + 1e-3);
        assert!(last.end >= last.start);
    }

    #[test]
    fn chunked_shifts_time_and_positions() {
        let durations = simple_durations();
        let chunk = |text: &'static str, offset: usize, dur: f32| ChunkTiming {
            text,
            text_offset: offset,
            duration_sec: dur,
            durations: &durations,
        };
        let ts = timestamps_from_durations(&[chunk("аб в", 0, 1.0), chunk("аб в", 100, 2.0)]);
        assert_eq!(ts.len(), 4);
        // Second chunk starts on the accumulated audio offset, not on its
        // own frame timeline.
        assert!((ts[2].start - (1.0 + 2.0 * FRAME_SEC)).abs() < 1e-3);
        assert_eq!(ts[2].original_pos, (100, 102));
        assert_eq!(ts[3].original_pos, (103, 104));
        for pair in ts.windows(2) {
            assert!(pair[0].end <= pair[1].start + 1e-3);
        }
    }

    #[test]
    fn no_words_no_timestamps_but_timeline_advances() {
        let durations = simple_durations();
        let ts = timestamps_from_durations(&[
            ChunkTiming {
                text: "… !",
                text_offset: 0,
                duration_sec: 1.5,
                durations: &durations,
            },
            ChunkTiming {
                text: "аб в",
                text_offset: 10,
                duration_sec: 1.0,
                durations: &durations,
            },
        ]);
        assert_eq!(ts.len(), 2);
        assert!((ts[0].start - (1.5 + 2.0 * FRAME_SEC)).abs() < 1e-3);
        assert_eq!(ts[0].original_pos, (10, 12));
    }

    #[test]
    fn leading_stress_marker_opens_the_word_range() {
        // Accentor output "+я" (stressed single-vowel word): the '+' frames
        // are rendered audio — the word starts at the marker, not at 'я'.
        let durations = vec![sd('|', 5), sd('+', 11), sd('я', 9), sd('~', 3)];
        let ts = timestamps_from_durations(&[ChunkTiming {
            text: "Я",
            text_offset: 0,
            duration_sec: 28.0 * FRAME_SEC,
            durations: &durations,
        }]);
        assert_eq!(ts.len(), 1);
        assert!((ts[0].start - 5.0 * FRAME_SEC).abs() < 1e-3);
        assert!((ts[0].end - 25.0 * FRAME_SEC).abs() < 1e-3);
    }

    #[test]
    fn standalone_plus_does_not_open_next_word() {
        // A literal '+' in the text ("правило + команда") is surrounded by
        // spaces — it is NOT a stress marker and must not start the next
        // word's range. Regression: it used to fold into "команда" and
        // cascade-misalign every following word.
        let durations = vec![
            sd('|', 1),
            sd('а', 1),
            sd('б', 1),
            sd(' ', 1),
            sd('+', 5),
            sd(' ', 1),
            sd('в', 1),
            sd('г', 1),
            sd('~', 2),
        ];
        let ts = timestamps_from_durations(&[ChunkTiming {
            text: "аб + вг",
            text_offset: 0,
            duration_sec: 14.0 * FRAME_SEC,
            durations: &durations,
        }]);
        assert_eq!(ts.len(), 2);
        // "вг" starts at 'в' (after the standalone '+'), spans 2 frames.
        assert!((ts[1].start - 10.0 * FRAME_SEC).abs() < 1e-3);
        assert!((ts[1].end - 12.0 * FRAME_SEC).abs() < 1e-3);
    }

    #[test]
    fn attached_plus_without_space_opens_next_word() {
        // "аб +вг": a `+` directly attached to the next letter is
        // syntactically an explicit stress marker (like user-typed "з+амок")
        // — its frames are part of the audible word onset and open the
        // word's range. Contrast with the standalone '+' above.
        let durations = vec![
            sd('|', 1),
            sd('а', 1),
            sd('б', 1),
            sd(' ', 1),
            sd('+', 5),
            sd('в', 1),
            sd('г', 1),
            sd('~', 2),
        ];
        let ts = timestamps_from_durations(&[ChunkTiming {
            text: "аб +вг",
            text_offset: 0,
            duration_sec: 13.0 * FRAME_SEC,
            durations: &durations,
        }]);
        assert_eq!(ts.len(), 2);
        assert!((ts[1].start - 4.0 * FRAME_SEC).abs() < 1e-3);
        assert!((ts[1].end - 11.0 * FRAME_SEC).abs() < 1e-3);
    }

    #[test]
    fn words_after_standalone_plus_stay_aligned() {
        // The production failure: after "правило +", fourteen following
        // words collapsed to zero-length timestamps at one instant.
        let durations = vec![
            sd('|', 1),
            sd('а', 1),
            sd('б', 1),
            sd(' ', 1),
            sd('+', 5),
            sd(' ', 1),
            sd('в', 1),
            sd('г', 1),
            sd(' ', 1),
            sd('д', 1),
            sd('е', 1),
            sd(' ', 1),
            sd('ж', 1),
            sd('з', 1),
            sd('~', 2),
        ];
        let ts = timestamps_from_durations(&[ChunkTiming {
            text: "аб + вг де жз",
            text_offset: 0,
            duration_sec: 18.0 * FRAME_SEC,
            durations: &durations,
        }]);
        assert_eq!(ts.len(), 4);
        let expected = [(1.0, 3.0), (10.0, 12.0), (13.0, 15.0), (16.0, 18.0)];
        for (t, &(s, e)) in ts.iter().zip(&expected) {
            assert!((t.start - s * FRAME_SEC).abs() < 1e-3, "{}: {t:?}", t.word);
            assert!((t.end - e * FRAME_SEC).abs() < 1e-3, "{}: {t:?}", t.word);
        }
    }

    #[test]
    fn dropped_latin_letters_do_not_cascade() {
        // The production failure: "get_variablesслэш" — the latin prefix was
        // dropped by the frontend, only "слэш" reached the stream. The word
        // must span its Cyrillic tail and the next word must stay aligned
        // (a mismatch used to shift every following word by one).
        let durations = vec![
            sd('|', 1),
            sd('с', 1),
            sd('л', 1),
            sd('э', 1),
            sd('ш', 1),
            sd(' ', 1),
            sd('с', 1),
            sd('е', 1),
            sd('т', 1),
            sd('~', 2),
        ];
        let ts = timestamps_from_durations(&[ChunkTiming {
            text: "get_variablesслэш сет",
            text_offset: 0,
            duration_sec: 11.0 * FRAME_SEC,
            durations: &durations,
        }]);
        assert_eq!(ts.len(), 2);
        // "get_variablesслэш" spans only its spoken tail "слэш".
        assert!((ts[0].start - 1.0 * FRAME_SEC).abs() < 1e-3, "{:?}", ts[0]);
        assert!((ts[0].end - 5.0 * FRAME_SEC).abs() < 1e-3, "{:?}", ts[0]);
        // "сет" aligns to its own symbols, not shifted into the tail.
        assert!((ts[1].start - 6.0 * FRAME_SEC).abs() < 1e-3, "{:?}", ts[1]);
        assert!((ts[1].end - 9.0 * FRAME_SEC).abs() < 1e-3, "{:?}", ts[1]);
    }

    #[test]
    fn latin_only_word_zero_length_without_cascade() {
        // A fully unspoken word ("abc") gets a zero-length timestamp and the
        // following words keep their own alignment.
        let durations = vec![
            sd('|', 1),
            sd('д', 1),
            sd('е', 1),
            sd(' ', 1),
            sd('ж', 1),
            sd('з', 1),
            sd('~', 2),
        ];
        let ts = timestamps_from_durations(&[ChunkTiming {
            text: "abc де жз",
            text_offset: 0,
            duration_sec: 9.0 * FRAME_SEC,
            durations: &durations,
        }]);
        assert_eq!(ts.len(), 3);
        assert_eq!(ts[0].start, ts[0].end);
        assert!((ts[0].start - 1.0 * FRAME_SEC).abs() < 1e-3);
        assert!((ts[1].start - 1.0 * FRAME_SEC).abs() < 1e-3);
        assert!((ts[1].end - 3.0 * FRAME_SEC).abs() < 1e-3);
        assert!((ts[2].start - 4.0 * FRAME_SEC).abs() < 1e-3);
        assert!((ts[2].end - 6.0 * FRAME_SEC).abs() < 1e-3);
    }

    #[test]
    fn timestamps_are_millisecond_rounded() {
        let durations = vec![sd('|', 1), sd('а', 1), sd('~', 1)];
        let ts = timestamps_from_durations(&[ChunkTiming {
            text: "а",
            text_offset: 0,
            duration_sec: 3.0 * FRAME_SEC,
            durations: &durations,
        }]);
        // 1 frame = 12.5 ms exactly → 0.012 / 0.025 after ms rounding.
        assert_eq!(ts[0].start, 0.013);
        assert_eq!(ts[0].end, 0.025);
    }
}
