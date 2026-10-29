//! Span annotations: the decisions the pipeline records for the adapter.
//!
//! A span is `(kind, language, range)` over the normalized text. Recording
//! happens mid-pipeline in original coordinates (regex matches are byte
//! offsets, but `TrackedText` can map any current position back to the
//! document), and the whole set is resolved into output coordinates once, in
//! [`resolve_spans`], from the `char_map`.

use crate::pipeline::tracked_text::{CharMapping, TrackedText};

/// What a span annotation marks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpanKind {
    /// A code identifier the engine reads itself.
    CodeIdentifier,
    /// A Latin run in prose the engine reads itself.
    ProseLatin,
    /// A fragment the user dictionary replaced.
    DictionaryOverride,
}

/// An annotation over the normalized text, in output codepoint coordinates.
///
/// Spans express decisions, never markup: the adapter decides how to render
/// them for its engine, and the normalized text itself never carries tags.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub kind: SpanKind,
    /// The language the engine reads the fragment in; `None` when the pipeline
    /// rewrote the fragment instead of leaving it to the engine.
    pub lang: Option<&'static str>,
    /// Half-open codepoint range in [`ProcessedText::text`].
    pub start: usize,
    pub end: usize,
}

/// A span as recorded while processing, in original-document coordinates.
#[derive(Debug, Clone)]
pub(crate) struct RecordedSpan {
    kind: SpanKind,
    lang: Option<&'static str>,
    start: usize,
    end: usize,
}

/// Maps every byte offset of `text` to its codepoint index.
///
/// Recording converts a regex byte match into a codepoint range; without the
/// table each conversion scans the prefix, which turns a document with many
/// spans into quadratic work.
pub(crate) fn byte_to_char_offsets(text: &str) -> Vec<usize> {
    let mut offsets = vec![0usize; text.len() + 1];
    let mut byte = 0usize;
    let mut chars = 0usize;
    for ch in text.chars() {
        for slot in &mut offsets[byte..byte + ch.len_utf8()] {
            *slot = chars;
        }
        byte += ch.len_utf8();
        chars += 1;
    }
    offsets[text.len()] = chars;
    offsets
}

/// Records byte ranges of `snapshot` as spans of `kind` in original coordinates.
pub(crate) fn record_byte_ranges(
    tracked: &mut TrackedText,
    offsets: &[usize],
    kind: SpanKind,
    lang: Option<&'static str>,
    ranges: &[(usize, usize)],
    recorded: &mut Vec<RecordedSpan>,
) {
    for &(start, end) in ranges {
        let (orig_start, orig_end) = tracked.original_range(offsets[start], offsets[end]);
        if orig_end > orig_start {
            recorded.push(RecordedSpan {
                kind,
                lang,
                start: orig_start,
                end: orig_end,
            });
        }
    }
}

/// Records what `find` selects over the current text as spans of `kind`.
pub(crate) fn record_native(
    tracked: &mut TrackedText,
    kind: SpanKind,
    lang: Option<&'static str>,
    find: impl FnOnce(&str) -> Vec<(usize, usize)>,
    recorded: &mut Vec<RecordedSpan>,
) {
    let snapshot = tracked.text().to_string();
    let ranges = find(&snapshot);
    let offsets = byte_to_char_offsets(&snapshot);
    record_byte_ranges(tracked, &offsets, kind, lang, &ranges, recorded);
}

/// Resolves recorded spans from original into output coordinates in one pass.
///
/// `char_map` is monotone in original position, so a single advancing cursor
/// covers every span: no reverse index, no string search.
pub(crate) fn resolve_spans(char_map: &[(usize, usize)], recorded: Vec<RecordedSpan>) -> Vec<Span> {
    let kept = keep_specific_spans(recorded);
    if kept.is_empty() {
        return Vec::new();
    }

    let mut resolved = Vec::with_capacity(kept.len());
    let mut cursor = 0usize;
    for span in kept {
        while cursor < char_map.len() && char_map[cursor].1 <= span.start {
            cursor += 1;
        }
        let start = cursor;
        let mut end = cursor;
        while cursor < char_map.len() && char_map[cursor].0 < span.end {
            cursor += 1;
            end = cursor;
        }
        if end > start {
            resolved.push(Span {
                kind: span.kind,
                lang: span.lang,
                start,
                end,
            });
        }
    }
    resolved
}

/// Drops overlapping records, keeping the most specific decision per region.
///
/// The overlap has to be resolved in original coordinates, not by comparing
/// live ranges: a native identifier also matches the prose word pattern, and
/// the dictionary pre-pass runs between the two recordings, so the current
/// text at each recording differs.
fn keep_specific_spans(recorded: Vec<RecordedSpan>) -> Vec<RecordedSpan> {
    let mut specific: Vec<RecordedSpan> = recorded
        .iter()
        .filter(|span| span.kind != SpanKind::ProseLatin)
        .cloned()
        .collect();
    // Priority first, then position: a dictionary override inside a native
    // identifier claims the region, and the identifier annotation is dropped
    // rather than emitted with a language that no longer describes it.
    specific.sort_by_key(|span| (kind_priority(span.kind), span.start, span.end - span.start));

    let mut kept: Vec<RecordedSpan> = Vec::with_capacity(specific.len());
    for span in specific {
        let overlaps = kept
            .iter()
            .any(|kept| span.start < kept.end && kept.start < span.end);
        if !overlaps {
            kept.push(span);
        }
    }

    let mut prose: Vec<RecordedSpan> = recorded
        .into_iter()
        .filter(|span| span.kind == SpanKind::ProseLatin)
        .collect();
    prose.sort_by_key(|span| (span.start, span.end));
    for span in prose {
        let claimed = kept
            .iter()
            .any(|kept| span.start < kept.end && kept.start < span.end);
        if !claimed {
            kept.push(span);
        }
    }

    kept.sort_by_key(|span| span.start);
    kept
}

/// Narrower wins when two records cover the same original range.
fn kind_priority(kind: SpanKind) -> u8 {
    match kind {
        SpanKind::DictionaryOverride => 0,
        SpanKind::CodeIdentifier => 1,
        SpanKind::ProseLatin => 2,
    }
}

/// Asserts the span invariants the `position-mapping` spec requires, in debug
/// builds only: sorted, non-overlapping, non-empty, inside the output.
pub(crate) fn debug_assert_spans(mapping: &CharMapping, spans: &[Span]) {
    debug_assert_eq!(
        mapping.char_map.len(),
        mapping.transformed.chars().count(),
        "char_map length must stay the codepoint count of the transformed text"
    );
    let mut previous_end = 0;
    for span in spans {
        debug_assert!(span.start < span.end, "empty span reported: {span:?}");
        debug_assert!(span.end <= mapping.char_map.len(), "span past the text");
        debug_assert!(
            span.start >= previous_end,
            "spans must be sorted and non-overlapping: {span:?}"
        );
        previous_end = span.end;
    }
}
