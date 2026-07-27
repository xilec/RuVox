/// Character-level mapping from transformed to original positions.
///
/// All indices in `char_map` are **Unicode codepoint** (char) indices, not byte
/// offsets. This matches the format used by golden fixtures and the on-disk
/// `timestamps.json` files.
#[derive(Debug, Clone)]
pub struct CharMapping {
    pub original: String,
    pub transformed: String,
    /// For each **codepoint** position in `transformed`, the range in `original`
    /// as `(orig_start, orig_end)` (exclusive end, codepoint units).
    pub char_map: Vec<(usize, usize)>,
}

impl CharMapping {
    /// Map a range in transformed text (codepoint indices) to a range in original text.
    ///
    /// Returns the minimal bounding range in `original` covering all codepoints in
    /// `transformed[trans_start..trans_end]`.
    pub fn get_original_range(&self, trans_start: usize, trans_end: usize) -> (usize, usize) {
        if self.char_map.is_empty() {
            return (trans_start, trans_end);
        }

        let len = self.char_map.len();

        // Clamp trans_start to valid index (mirrors Python `min(trans_start, len-1)`)
        let ts = trans_start.min(len - 1);

        if trans_start >= len {
            // Position past end of text — same clamped ts used below, but Python
            // would have entered the `if trans_start >= len` branch and returned early.
            // We replicate: after clamping, ts == len-1, we still continue to read
            // char_map[ts]. The `if trans_start >= len` in Python only triggers when
            // the unclamped value is past end, BUT clamping happens before, so the
            // branch is actually never reached (clamped ts < len always). So just fall
            // through and read char_map[ts].
            let entry = self.char_map[ts];
            return entry;
        }

        let mut orig_start = self.char_map[ts].0;
        let mut orig_end = self.char_map[ts].1;

        let te = trans_end.min(len);
        for i in (ts + 1)..te {
            let entry = self.char_map[i];
            if entry.0 < orig_start {
                orig_start = entry.0;
            }
            if entry.1 > orig_end {
                orig_end = entry.1;
            }
        }

        (orig_start, orig_end)
    }

    /// Get word boundaries in original text (codepoint indices) for a position in
    /// transformed text.
    ///
    /// Expands outward to whitespace boundaries on both sides.
    pub fn get_original_word_range(&self, trans_pos: usize) -> (usize, usize) {
        let (orig_start, orig_end) = self.get_original_range(trans_pos, trans_pos + 1);

        let chars: Vec<char> = self.original.chars().collect();

        let mut word_start = orig_start;
        let mut word_end = orig_end;

        while word_start > 0 && !chars[word_start - 1].is_whitespace() {
            word_start -= 1;
        }

        while word_end < chars.len() && !chars[word_end].is_whitespace() {
            word_end += 1;
        }

        (word_start, word_end)
    }
}

// ---- Internal tracking types ----

/// Records a single replacement for building CharMapping.
#[derive(Debug, Clone)]
struct Replacement {
    orig_start: usize, // codepoint index in original
    orig_end: usize,   // codepoint index in original (exclusive)
    new_text: String,
}

/// Tracks a replacement entry for position mapping.
///
/// Entries live in `TrackedText::offset_entries`, sorted by `orig_start`.
/// Original ranges are disjoint (overlapping candidates are rejected before
/// insertion), so `orig_start` values are distinct and `orig_end` is sorted
/// as well — every query binary-searches the index instead of scanning it.
#[derive(Debug, Clone)]
struct OffsetEntry {
    orig_start: usize, // codepoint index in original
    orig_end: usize,   // codepoint index in original (exclusive)
    new_len: usize,    // codepoint count of replacement text
}

/// A queued replacement whose byte range refers to the current text.
#[derive(Debug)]
struct PendingReplace {
    byte_start: usize,
    byte_end: usize,
    to: String,
}

/// A queued replacement that passed all overlap checks, ready to splice.
#[derive(Debug)]
struct AcceptedReplace {
    byte_start: usize,
    byte_end: usize,
    orig_start: usize,
    orig_end: usize,
    new_text: String,
}

// ---- Helper: convert between byte and codepoint indices ----

/// Convert a byte offset in a string to its codepoint index.
///
/// Production code converts offsets in bulk during the batch splice; this
/// per-offset form is used by the mapping tests.
#[cfg(test)]
fn byte_to_char_idx(s: &str, byte_offset: usize) -> usize {
    s[..byte_offset].chars().count()
}

/// Convert a codepoint index to the byte offset in a string.
#[allow(dead_code)]
pub fn char_to_byte_idx(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(b, _)| b)
        .unwrap_or(s.len())
}

/// Codepoint length of a string.
///
/// Counts UTF-8 lead bytes (everything except `0b10xx_xxxx` continuation
/// bytes) instead of decoding every char — measurably cheaper in unoptimized
/// builds, and the batch splice walks the whole text once per phase.
fn char_len(s: &str) -> usize {
    s.as_bytes()
        .iter()
        .filter(|b| (*b & 0b1100_0000) != 0b1000_0000)
        .count()
}

/// Text wrapper that tracks all modifications for precise position mapping.
///
/// All position tracking uses **Unicode codepoint** indices, not byte offsets,
/// so multi-byte characters (Cyrillic, emoji, etc.) count as one position each.
///
/// ## Why replacements are applied in batches
///
/// A normalization phase queues many replacements (one per matched word,
/// number, symbol, …). Applying each one eagerly costs O(n) for the string
/// splice plus O(history) for the position-map bookkeeping, which made a
/// phase O(M·n + M²) and wedged the app at 100% CPU on large pastes
/// (see openspec change `fix-pipeline-quadratic`). Instead, replacements are
/// queued in `pending` and applied by `flush_pending` in a single
/// left-to-right pass: one string rebuild and one sorted merge into the
/// entry index per batch, so a phase costs O(n + M log M).
pub struct TrackedText {
    pub original: String,
    current: String,
    pending: Vec<PendingReplace>,
    replacements: Vec<Replacement>,
    /// Sorted by `orig_start`; ranges disjoint (see `OffsetEntry`).
    offset_entries: Vec<OffsetEntry>,
    /// `delta_prefix[i]` = sum of `new_len - old_len` over
    /// `offset_entries[..i]`. One element longer than `offset_entries`.
    delta_prefix: Vec<i64>,
}

impl TrackedText {
    pub fn new(text: impl Into<String>) -> Self {
        let s = text.into();
        Self {
            original: s.clone(),
            current: s,
            pending: Vec::new(),
            replacements: Vec::new(),
            offset_entries: Vec::new(),
            delta_prefix: vec![0],
        }
    }

    pub fn text(&mut self) -> &str {
        self.flush_pending();
        &self.current
    }

    pub fn len(&mut self) -> usize {
        self.flush_pending();
        char_len(&self.current)
    }

    // &mut self: queued replacements must be flushed before the answer is
    // meaningful — clippy's convention lint does not know that.
    #[allow(clippy::wrong_self_convention)]
    pub fn is_empty(&mut self) -> bool {
        self.flush_pending();
        self.current.is_empty()
    }

    /// Replace all literal occurrences of `from` with `to`, tracking positions.
    ///
    /// Equivalent to Python `TrackedText.replace(old, new)`.
    ///
    /// WARNING: this is an unbounded replace-all — it also hits `from` embedded
    /// in longer tokens ("use" inside "user", "42" inside "142"). Phases that
    /// collect boundary-checked matches must apply them via
    /// [`Self::replace_byte_range`] instead (see #75, #84, #109). Literal
    /// `replace` is reserved for constant patterns where every occurrence must
    /// be substituted (quotes, dashes, operators, symbols, C++/C#/F# terms).
    pub fn replace(&mut self, from: &str, to: &str) {
        let pattern =
            regex::Regex::new(&regex::escape(from)).expect("regex::escape produces valid pattern");
        self.sub(&pattern, |_| to.to_string());
    }

    /// Queue a replacement of exactly one byte range `[byte_start, byte_end)`
    /// in the current text.
    ///
    /// This allows callers to replace a single occurrence without constructing
    /// a literal string that might match elsewhere. Used by markdown link
    /// stripping to remove the leading `[` and the trailing `](url)`
    /// independently, so that the link-text characters retain individual
    /// original-position entries and can still be normalised by later pipeline
    /// phases.
    ///
    /// Application is batched (see the struct docs): the range must refer to
    /// the text as it stands before any other queued-but-unapplied
    /// replacement. All callers compute ranges against a snapshot and queue
    /// them in reverse document order, which satisfies this.
    pub fn replace_byte_range(&mut self, byte_start: usize, byte_end: usize, to: &str) {
        self.pending.push(PendingReplace {
            byte_start,
            byte_end,
            to: to.to_string(),
        });
    }

    /// Regex substitution with a callback, tracking positions for `CharMapping`.
    ///
    /// Matches that overlap already-replaced regions are skipped — exactly
    /// replicating Python `TrackedText.sub` behaviour.
    pub fn sub<F>(&mut self, pattern: &regex::Regex, mut replacer: F)
    where
        F: FnMut(&regex::Captures) -> String,
    {
        self.flush_pending();

        // Take the text out for iteration (the replacer only sees captures,
        // never `self`), avoiding a full-text clone per phase.
        let snapshot = std::mem::take(&mut self.current);

        // Collect all matches with their new texts, then apply them as one
        // batch. Regex matches are disjoint, so the application order inside
        // a batch cannot change the result; ascending order enables a single
        // left-to-right splice pass in flush_pending.
        self.pending
            .extend(pattern.captures_iter(&snapshot).map(|caps| {
                let m = caps.get(0).unwrap();
                PendingReplace {
                    byte_start: m.start(),
                    byte_end: m.end(),
                    to: replacer(&caps),
                }
            }));
        // Move the text back (O(1)) so flush_pending splices the real string.
        self.current = snapshot;
        self.flush_pending();
    }

    /// Build the `CharMapping` from all accumulated replacements.
    pub fn build_mapping(mut self) -> CharMapping {
        self.flush_pending();
        let orig_char_len = char_len(&self.original);
        let trans_char_len = char_len(&self.current);

        if self.replacements.is_empty() {
            let char_map: Vec<(usize, usize)> = (0..trans_char_len).map(|i| (i, i + 1)).collect();
            return CharMapping {
                original: self.original,
                transformed: self.current,
                char_map,
            };
        }

        // Sort replacements by original codepoint position.
        let mut sorted = self.replacements;
        sorted.sort_by_key(|r| r.orig_start);

        let mut char_map: Vec<(usize, usize)> = Vec::new();
        let mut orig_idx: usize = 0;

        for repl in &sorted {
            // Identity mapping for unchanged codepoints before this replacement.
            while orig_idx < repl.orig_start {
                char_map.push((orig_idx, orig_idx + 1));
                orig_idx += 1;
            }

            // All codepoints in new_text map to the replaced original range.
            let repl_char_len = char_len(&repl.new_text);
            for _ in 0..repl_char_len {
                char_map.push((repl.orig_start, repl.orig_end));
            }

            orig_idx = repl.orig_end;
        }

        // Identity mapping for remaining unchanged codepoints.
        while orig_idx < orig_char_len {
            char_map.push((orig_idx, orig_idx + 1));
            orig_idx += 1;
        }

        CharMapping {
            original: self.original,
            transformed: self.current,
            char_map,
        }
    }

    // ---- Batch application ----

    /// Validate and apply all queued replacements in a single pass.
    ///
    /// One batch costs O(n + m log r) (n = text length, m = queued
    /// replacements, r = recorded entries): one left-to-right string rebuild
    /// plus one sorted merge into the entry index.
    fn flush_pending(&mut self) {
        if self.pending.is_empty() {
            return;
        }
        let mut batch = std::mem::take(&mut self.pending);
        // Callers queue ranges in reverse document order; sort ascending for
        // the splice pass. Ranges are disjoint by construction (regex matches
        // never overlap; snapshot-based loops queue disjoint regions).
        batch.sort_by_key(|p| p.byte_start);
        assert!(
            batch.windows(2).all(|w| w[0].byte_end <= w[1].byte_start),
            "flush_pending: queued replacement ranges must be disjoint"
        );

        // Walk the text once, converting byte offsets to codepoint indices on
        // the way, and validate each queued replacement against the committed
        // entries. A batch's own ranges are disjoint, so they cannot affect
        // each other's checks — validating against the committed index is
        // equivalent to the old per-replacement sequential validation.
        let mut accepted: Vec<AcceptedReplace> = Vec::with_capacity(batch.len());
        let mut byte_cursor = 0;
        let mut char_cursor = 0;
        for p in batch {
            char_cursor += char_len(&self.current[byte_cursor..p.byte_start]);
            let char_start = char_cursor;
            char_cursor += char_len(&self.current[p.byte_start..p.byte_end]);
            let char_end = char_cursor;
            byte_cursor = p.byte_end;

            if self.current[p.byte_start..p.byte_end] == p.to {
                continue;
            }
            if self.overlaps_current_replacement(char_start, char_end) {
                continue;
            }
            let orig_start = self.current_to_original(char_start);
            let orig_end = if char_end > char_start {
                self.current_to_original(char_end - 1) + 1
            } else {
                orig_start
            };
            if self.find_containing_replacement(orig_start, orig_end) {
                continue;
            }
            accepted.push(AcceptedReplace {
                byte_start: p.byte_start,
                byte_end: p.byte_end,
                orig_start,
                orig_end,
                new_text: p.to,
            });
        }
        if accepted.is_empty() {
            return;
        }

        // Single string rebuild for the whole batch.
        let new_byte_len = accepted.iter().fold(self.current.len(), |acc, a| {
            acc + a.new_text.len() - (a.byte_end - a.byte_start)
        });
        let mut next = String::with_capacity(new_byte_len);
        let mut cursor = 0;
        for a in &accepted {
            next.push_str(&self.current[cursor..a.byte_start]);
            next.push_str(&a.new_text);
            cursor = a.byte_end;
        }
        next.push_str(&self.current[cursor..]);
        self.current = next;

        self.replacements
            .extend(accepted.iter().map(|a| Replacement {
                orig_start: a.orig_start,
                orig_end: a.orig_end,
                new_text: a.new_text.clone(),
            }));

        // Accepted ranges ascend in current coordinates, hence in original
        // coordinates too — merge into the sorted index in one pass.
        let new_entries: Vec<OffsetEntry> = accepted
            .into_iter()
            .map(|a| OffsetEntry {
                orig_start: a.orig_start,
                orig_end: a.orig_end,
                new_len: char_len(&a.new_text),
            })
            .collect();
        self.merge_entries(new_entries);
    }

    /// Merge new entries (ascending `orig_start`, disjoint from the existing
    /// ranges) into the sorted index and rebuild the delta prefix sums.
    fn merge_entries(&mut self, new_entries: Vec<OffsetEntry>) {
        let old = std::mem::take(&mut self.offset_entries);
        let mut merged = Vec::with_capacity(old.len() + new_entries.len());
        let mut new_iter = new_entries.into_iter().peekable();
        for entry in old {
            while let Some(ne) = new_iter.next_if(|ne| ne.orig_start < entry.orig_start) {
                merged.push(ne);
            }
            merged.push(entry);
        }
        merged.extend(new_iter);
        self.offset_entries = merged;

        self.delta_prefix.clear();
        self.delta_prefix.push(0);
        let mut cumulative: i64 = 0;
        for entry in &self.offset_entries {
            cumulative += entry.new_len as i64 - (entry.orig_end - entry.orig_start) as i64;
            self.delta_prefix.push(cumulative);
        }
    }

    // ---- Interval index queries ----

    /// Start of entry `i`'s replacement text in current-text codepoint
    /// coordinates. Saturates at 0 like the old cumulative-delta scan: a
    /// non-monotone replacement chain could otherwise wrap a `usize`.
    fn entry_current_start(&self, i: usize) -> i64 {
        (self.offset_entries[i].orig_start as i64 + self.delta_prefix[i]).max(0)
    }

    /// End (exclusive) of entry `i`'s replacement text in current coordinates.
    fn entry_current_end(&self, i: usize) -> i64 {
        self.entry_current_start(i) + self.offset_entries[i].new_len as i64
    }

    /// Returns `true` if any codepoint in `[char_start, char_end)` (current
    /// coordinates) lies inside the replacement text of an existing entry.
    fn overlaps_current_replacement(&self, char_start: usize, char_end: usize) -> bool {
        if char_start >= char_end {
            return false;
        }
        // Entry current ranges are disjoint and ascending, so only the first
        // entry ending after `char_start` can overlap the range.
        let candidate = partition_point(self.offset_entries.len(), |i| {
            self.entry_current_end(i) <= char_start as i64
        });
        candidate < self.offset_entries.len()
            && self.entry_current_start(candidate) < char_end as i64
    }

    /// Returns `true` if an existing entry's original range overlaps
    /// `[orig_start, orig_end)` — or contains the point, for an empty range.
    fn find_containing_replacement(&self, orig_start: usize, orig_end: usize) -> bool {
        if orig_start == orig_end {
            // Last entry starting at or before the point contains it iff the
            // point is below that entry's end.
            let i = partition_point(self.offset_entries.len(), |i| {
                self.offset_entries[i].orig_start <= orig_start
            });
            i > 0 && orig_start < self.offset_entries[i - 1].orig_end
        } else {
            // Original ranges are disjoint and ascending: only the first
            // entry ending after `orig_start` can overlap.
            let i = partition_point(self.offset_entries.len(), |i| {
                self.offset_entries[i].orig_end <= orig_start
            });
            i < self.offset_entries.len() && self.offset_entries[i].orig_start < orig_end
        }
    }

    /// Convert a codepoint position in the current text to the corresponding
    /// codepoint position in the original text.
    ///
    /// Negative intermediates are saturated to 0: a non-monotone replacement
    /// chain could in principle produce a delta whose magnitude exceeds the
    /// position being mapped. Mapping such cases to 0 keeps the result a
    /// valid index instead of wrapping into a huge usize.
    fn current_to_original(&self, current_pos: usize) -> usize {
        let pos = current_pos as i64;
        // Last entry whose replacement text starts at or before `pos`.
        let i = partition_point(self.offset_entries.len(), |i| {
            self.entry_current_start(i) <= pos
        });
        if i == 0 {
            // Before the first entry: no delta accumulated yet.
            return current_pos;
        }
        if pos < self.entry_current_end(i - 1) {
            self.offset_entries[i - 1].orig_start
        } else {
            (pos - self.delta_prefix[i]).max(0) as usize
        }
    }
}

/// First index in `0..len` for which `pred` is false, assuming `pred` is
/// monotone (`true…true false…false`). Binary search over an index range —
/// `slice::partition_point` does not apply without a materialized slice.
fn partition_point(len: usize, mut pred: impl FnMut(usize) -> bool) -> usize {
    let mut lo = 0;
    let mut hi = len;
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if pred(mid) {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    lo
}

#[cfg(test)]
mod tests {
    use super::*;
    use regex::Regex;

    // ============================================================
    // Port of: TestTrackedTextBasic
    // ============================================================

    /// Python: TestTrackedTextBasic::test_no_changes
    #[test]
    fn test_no_changes() {
        let tracked = TrackedText::new("Hello world");
        let mapping = tracked.build_mapping();

        assert_eq!(mapping.original, "Hello world");
        assert_eq!(mapping.transformed, "Hello world");
        assert_eq!(mapping.char_map.len(), 11);
        for i in 0..11 {
            assert_eq!(mapping.char_map[i], (i, i + 1));
        }
    }

    /// Python: TestTrackedTextBasic::test_simple_replace
    #[test]
    fn test_simple_replace() {
        let mut tracked = TrackedText::new("Hello world");
        tracked.replace("world", "мир");

        assert_eq!(tracked.text(), "Hello мир");
        let mapping = tracked.build_mapping();

        assert_eq!(mapping.original, "Hello world");
        assert_eq!(mapping.transformed, "Hello мир");
    }

    /// Python: TestTrackedTextBasic::test_simple_sub
    #[test]
    fn test_simple_sub() {
        let mut tracked = TrackedText::new("Hello world");
        let pat = Regex::new("world").unwrap();
        tracked.sub(&pat, |_| "мир".to_string());

        assert_eq!(tracked.text(), "Hello мир");
    }

    /// Python: TestTrackedTextBasic::test_chaining
    /// (Rust uses &mut self, so chaining is sequential calls)
    #[test]
    fn test_chaining() {
        let mut tracked = TrackedText::new("Hello world");
        tracked.replace("Hello", "Привет");
        tracked.replace("world", "мир");

        assert_eq!(tracked.text(), "Привет мир");
    }

    // ============================================================
    // Port of: TestCharMapping
    // ============================================================

    /// Python: TestCharMapping::test_simple_replacement_mapping
    /// All indices are codepoint-based, matching Python semantics.
    #[test]
    fn test_simple_replacement_mapping() {
        let mut tracked = TrackedText::new("Hello world");
        tracked.replace("world", "мир");
        let mapping = tracked.build_mapping();

        // "Hello " stays same (codepoints 0–5)
        for i in 0..6 {
            assert_eq!(mapping.char_map[i], (i, i + 1));
        }

        // "мир" (3 codepoints) maps to original "world" (codepoints 6–11)
        for i in 6..9 {
            assert_eq!(mapping.char_map[i], (6, 11));
        }
    }

    /// Python: TestCharMapping::test_get_original_range_unchanged
    #[test]
    fn test_get_original_range_unchanged() {
        let tracked = TrackedText::new("Hello world");
        let mapping = tracked.build_mapping();

        let result = mapping.get_original_range(0, 5);
        assert_eq!(result, (0, 5));
    }

    /// Python: TestCharMapping::test_get_original_range_replaced
    #[test]
    fn test_get_original_range_replaced() {
        let mut tracked = TrackedText::new("Hello world");
        tracked.replace("world", "мир");
        let mapping = tracked.build_mapping();

        // "мир" at codepoints 6–9 should map back to "world" at 6–11
        let result = mapping.get_original_range(6, 9);
        assert_eq!(result, (6, 11));
    }

    /// Python: TestCharMapping::test_get_original_range_spanning
    #[test]
    fn test_get_original_range_spanning() {
        let mut tracked = TrackedText::new("Hello world");
        tracked.replace("world", "мир");
        let mapping = tracked.build_mapping();

        // Range spanning "o " (codepoints 4–6) and first char of "мир"
        let result = mapping.get_original_range(4, 7);
        assert_eq!(result.0, 4);
        assert_eq!(result.1, 11);
    }

    // ============================================================
    // Port of: TestMultipleReplacements
    // ============================================================

    /// Python: TestMultipleReplacements::test_two_replacements
    #[test]
    fn test_two_replacements() {
        let mut tracked = TrackedText::new("Hello world");
        tracked.replace("Hello", "Привет");
        tracked.replace("world", "мир");

        assert_eq!(tracked.text(), "Привет мир");
        let mapping = tracked.build_mapping();

        // "Привет" (6 codepoints) maps to "Hello" (codepoints 0–5)
        for i in 0..6 {
            assert_eq!(mapping.char_map[i], (0, 5));
        }

        // Space at codepoint 6 maps to space at codepoint 5–6
        assert_eq!(mapping.char_map[6], (5, 6));

        // "мир" (3 codepoints) maps to "world" (codepoints 6–11)
        for i in 7..10 {
            assert_eq!(mapping.char_map[i], (6, 11));
        }
    }

    /// Python: TestMultipleReplacements::test_expanding_replacement
    #[test]
    fn test_expanding_replacement() {
        let mut tracked = TrackedText::new("12345");
        tracked.replace("12345", "двенадцать тысяч триста сорок пять");

        let mapping = tracked.build_mapping();

        for i in 0..char_len(&mapping.transformed) {
            assert_eq!(mapping.char_map[i], (0, 5));
        }
    }

    /// Python: TestMultipleReplacements::test_contracting_replacement
    #[test]
    fn test_contracting_replacement() {
        let mut tracked = TrackedText::new("getUserData");
        tracked.replace("getUserData", "гет");

        let mapping = tracked.build_mapping();

        // "гет" (3 codepoints) maps to full "getUserData" (11 codepoints)
        for i in 0..3 {
            assert_eq!(mapping.char_map[i], (0, 11));
        }
    }

    // ============================================================
    // Port of: TestRegexSubstitution
    // ============================================================

    /// Python: TestRegexSubstitution::test_regex_with_groups
    #[test]
    fn test_regex_with_groups() {
        let mut tracked = TrackedText::new("test_value_here");
        let pat = Regex::new("_").unwrap();
        tracked.sub(&pat, |_| " ".to_string());

        assert_eq!(tracked.text(), "test value here");
    }

    /// Python: TestRegexSubstitution::test_regex_callback
    #[test]
    fn test_regex_callback() {
        let mut tracked = TrackedText::new("hello WORLD");
        let pat = Regex::new("[A-Z]+").unwrap();
        tracked.sub(&pat, |caps| caps[0].to_lowercase());

        assert_eq!(tracked.text(), "hello world");
    }

    /// Python: TestRegexSubstitution::test_limited_count
    /// Python supports count= parameter; Rust simulates via a counter in the closure.
    #[test]
    fn test_limited_count() {
        let mut tracked = TrackedText::new("a_b_c_d");
        let pat = Regex::new("_").unwrap();
        let mut count = 0;
        tracked.sub(&pat, |caps| {
            if count < 2 {
                count += 1;
                " ".to_string()
            } else {
                caps[0].to_string() // no-op
            }
        });

        assert_eq!(tracked.text(), "a b c_d");
    }

    // ============================================================
    // Port of: TestWordBoundaryMapping
    // ============================================================

    /// Python: TestWordBoundaryMapping::test_get_original_word_range
    #[test]
    fn test_get_original_word_range() {
        let tracked = TrackedText::new("Hello world test");
        let mapping = tracked.build_mapping();

        let (word_start, word_end) = mapping.get_original_word_range(6);
        assert_eq!(word_start, 6);
        assert_eq!(word_end, 11);
    }

    /// Python: TestWordBoundaryMapping::test_get_word_range_after_replacement
    #[test]
    fn test_get_word_range_after_replacement() {
        let mut tracked = TrackedText::new("Hello world");
        tracked.replace("world", "мир");
        let mapping = tracked.build_mapping();

        // Position 7 in "Hello мир" is inside "мир" → should map to "world" bounds
        let (word_start, word_end) = mapping.get_original_word_range(7);
        assert_eq!(word_start, 6);
        assert_eq!(word_end, 11);
    }

    // ============================================================
    // Port of: TestEdgeCases
    // ============================================================

    /// Python: TestEdgeCases::test_empty_text
    #[test]
    fn test_empty_text() {
        let tracked = TrackedText::new("");
        let mapping = tracked.build_mapping();

        assert_eq!(mapping.original, "");
        assert_eq!(mapping.transformed, "");
        assert_eq!(mapping.char_map.len(), 0);
    }

    /// Python: TestEdgeCases::test_replacement_at_start
    #[test]
    fn test_replacement_at_start() {
        let mut tracked = TrackedText::new("Hello world");
        tracked.replace("Hello", "Привет");
        let mapping = tracked.build_mapping();

        assert_eq!(mapping.transformed, "Привет world");
        assert_eq!(mapping.char_map[0], (0, 5));
    }

    /// Python: TestEdgeCases::test_replacement_at_end
    #[test]
    fn test_replacement_at_end() {
        let mut tracked = TrackedText::new("Hello world");
        tracked.replace("world", "мир");
        let mapping = tracked.build_mapping();

        assert_eq!(mapping.transformed, "Hello мир");
        assert_eq!(*mapping.char_map.last().unwrap(), (6, 11));
    }

    /// Python: TestEdgeCases::test_adjacent_replacements
    #[test]
    fn test_adjacent_replacements() {
        let mut tracked = TrackedText::new("AB");
        tracked.replace("A", "1");
        tracked.replace("B", "2");
        let mapping = tracked.build_mapping();

        assert_eq!(mapping.transformed, "12");
        assert_eq!(mapping.char_map[0], (0, 1));
        assert_eq!(mapping.char_map[1], (1, 2));
    }

    /// Python: TestEdgeCases::test_position_past_end
    /// Python clamps trans_start to len-1 before checking the branch, so the
    /// result is the last char's entry, not (last.1, last.1).
    #[test]
    fn test_position_past_end() {
        let tracked = TrackedText::new("Hello");
        let mapping = tracked.build_mapping();

        let result = mapping.get_original_range(10, 15);
        assert_eq!(result, (4, 5)); // last char position
    }

    // ============================================================
    // Port of: TestRealWorldCases
    // ============================================================

    /// Python: TestRealWorldCases::test_number_expansion
    #[test]
    fn test_number_expansion() {
        let mut tracked = TrackedText::new("Осталось 42 дня");
        let pat = Regex::new(r"\b42\b").unwrap();
        tracked.sub(&pat, |_| "сорок два".to_string());

        let mapping = tracked.build_mapping();

        // "Осталось " = 9 codepoints; "42" at codepoints 9–11
        // "сорок два" = 9 codepoints
        let prefix_cp = char_len("Осталось ");
        let forty_two_cp = char_len("42");
        let soroki_dva_cp = char_len("сорок два");

        for i in prefix_cp..(prefix_cp + soroki_dva_cp) {
            assert_eq!(
                mapping.char_map[i],
                (prefix_cp, prefix_cp + forty_two_cp),
                "char_map[{}] should map to 42's codepoint range",
                i
            );
        }
    }

    /// Python: TestRealWorldCases::test_code_identifier
    #[test]
    fn test_code_identifier() {
        let mut tracked = TrackedText::new("Вызови getUserData");
        let pat = Regex::new("getUserData").unwrap();
        tracked.sub(&pat, |_| "гет юзер дата".to_string());

        let mapping = tracked.build_mapping();

        // "Вызови " = 7 codepoints
        let original_start_cp = char_len("Вызови ");
        let original_end_cp = original_start_cp + char_len("getUserData");

        let get_user_data_ru = "гет юзер дата";
        for i in original_start_cp..(original_start_cp + char_len(get_user_data_ru)) {
            assert_eq!(
                mapping.char_map[i],
                (original_start_cp, original_end_cp),
                "char_map[{}] should map to getUserData's codepoint range",
                i
            );
        }
    }

    /// Python: TestRealWorldCases::test_multiple_transformations
    #[test]
    fn test_multiple_transformations() {
        let mut tracked = TrackedText::new("Test 123 API");
        let pat1 = Regex::new(r"\b123\b").unwrap();
        tracked.sub(&pat1, |_| "сто двадцать три".to_string());

        let pat2 = Regex::new(r"\bAPI\b").unwrap();
        tracked.sub(&pat2, |_| "эй пи ай".to_string());

        let mapping = tracked.build_mapping();

        assert!(mapping.transformed.contains("сто двадцать три"));
        assert!(mapping.transformed.contains("эй пи ай"));
    }

    // ============================================================
    // Port of: TestCharMappingConsistency (from test_char_mapping.py)
    // ============================================================

    /// Python: TestCharMappingConsistency::test_simple_text
    /// char_map length == codepoint count of transformed text.
    #[test]
    fn test_char_map_length_equals_transformed_length_simple() {
        let tracked = TrackedText::new("Привет мир");
        let mapping = tracked.build_mapping();
        assert_eq!(mapping.char_map.len(), char_len(&mapping.transformed));
    }

    /// char_map length should equal transformed codepoint count after expansion.
    #[test]
    fn test_char_map_length_equals_transformed_after_expansion() {
        let mut tracked = TrackedText::new("Test 42 end");
        let pat = Regex::new(r"\b42\b").unwrap();
        tracked.sub(&pat, |_| "сорок два".to_string());

        let mapping = tracked.build_mapping();
        assert_eq!(
            mapping.char_map.len(),
            char_len(&mapping.transformed),
            "char_map len {} != transformed codepoint len {}",
            mapping.char_map.len(),
            char_len(&mapping.transformed)
        );
    }

    /// char_map length should equal transformed codepoint count after contraction.
    #[test]
    fn test_char_map_length_equals_transformed_after_contraction() {
        let mut tracked = TrackedText::new("getUserData");
        let pat = Regex::new("getUserData").unwrap();
        tracked.sub(&pat, |_| "гет".to_string());

        let mapping = tracked.build_mapping();
        assert_eq!(mapping.char_map.len(), char_len(&mapping.transformed));
    }

    /// Python: TestPreprocessingTracking::test_multiple_spaces_tracked
    #[test]
    fn test_multiple_spaces_collapsed() {
        let mut tracked = TrackedText::new("Привет   мир"); // 3 spaces
        let pat = Regex::new(r" {2,}").unwrap();
        tracked.sub(&pat, |_| " ".to_string());

        let mapping = tracked.build_mapping();
        assert_eq!(mapping.char_map.len(), char_len(&mapping.transformed));

        // "Привет" = 6 codepoints, then 3 spaces → "мир" at codepoint 9
        let mir_orig_cp = char_len("Привет   ");
        assert_eq!(mir_orig_cp, 9);

        // In transformed: "Привет мир" → "мир" at codepoint 7
        let mir_norm_cp = mapping
            .transformed
            .find("мир")
            .map(|b| byte_to_char_idx(&mapping.transformed, b))
            .unwrap();
        assert_eq!(mir_norm_cp, 7);

        let (orig_start, _orig_end) = mapping.get_original_range(mir_norm_cp, mir_norm_cp + 3);
        assert_eq!(orig_start, mir_orig_cp);
    }

    // ============================================================
    // Port of: TestCharMappingRanges
    // ============================================================

    /// Python: TestCharMappingRanges::test_identity_range
    #[test]
    fn test_identity_range_unchanged() {
        let tracked = TrackedText::new("Простой текст");
        let mapping = tracked.build_mapping();

        // "Простой" = 7 codepoints
        let (orig_start, orig_end) = mapping.get_original_range(0, 7);
        assert_eq!(orig_start, 0);
        assert_eq!(orig_end, 7);
    }

    /// Python: TestCharMappingRanges::test_expanded_word_range
    #[test]
    fn test_expanded_word_range() {
        let mut tracked = TrackedText::new("API");
        let pat = Regex::new(r"\bAPI\b").unwrap();
        tracked.sub(&pat, |_| "эй пи ай".to_string());

        let mapping = tracked.build_mapping();
        assert_eq!(mapping.char_map.len(), char_len(&mapping.transformed));

        for i in 0..char_len(&mapping.transformed) {
            let (orig_start, orig_end) = mapping.get_original_range(i, i + 1);
            assert!(orig_start < 3 || (orig_start == 0 && orig_end <= 3));
            assert!(
                orig_end <= 3,
                "orig_end {} should be <= 3 (len of 'API')",
                orig_end
            );
        }
    }

    // ============================================================
    // Port of: TestPreprocessingTracking
    // ============================================================

    /// Python: TestPreprocessingTracking::test_tabs_converted_to_space
    #[test]
    fn test_tab_converted_to_space() {
        let mut tracked = TrackedText::new("Привет\tмир");
        let pat = Regex::new(r"\t").unwrap();
        tracked.sub(&pat, |_| " ".to_string());

        let mapping = tracked.build_mapping();
        assert_eq!(mapping.char_map.len(), char_len(&mapping.transformed));

        // "Привет" = 6 cp, tab = 1 cp → "мир" at cp 7
        let mir_orig_cp = char_len("Привет\t");
        let mir_norm_b = mapping.transformed.find("мир").unwrap();
        let mir_norm_cp = byte_to_char_idx(&mapping.transformed, mir_norm_b);
        let (orig_start, _) = mapping.get_original_range(mir_norm_cp, mir_norm_cp + 3);
        assert_eq!(orig_start, mir_orig_cp);
    }

    /// Python: TestPreprocessingTracking::test_bom_removed_tracked
    #[test]
    fn test_bom_removed_tracked() {
        let text = "\u{feff}Привет мир";
        let mut tracked = TrackedText::new(text);
        let pat = Regex::new("\u{feff}").unwrap();
        tracked.sub(&pat, |_| String::new());

        let mapping = tracked.build_mapping();
        assert_eq!(mapping.original, text);
        assert!(!mapping.transformed.starts_with('\u{feff}'));

        // BOM is 1 codepoint; "Привет" starts at codepoint 1 in original
        let bom_cp = 1usize;
        let privet_norm_cp = byte_to_char_idx(
            &mapping.transformed,
            mapping.transformed.find("Привет").unwrap(),
        );
        assert_eq!(privet_norm_cp, 0);

        let (orig_start, _) = mapping.get_original_range(0, 6);
        assert_eq!(orig_start, bom_cp);
    }

    // ============================================================
    // Additional: overlap / nested replacement handling
    // ============================================================

    /// Replacement inside an already-replaced region must be skipped.
    #[test]
    fn test_nested_replacement_skipped() {
        let mut tracked = TrackedText::new("Hello world");
        tracked.replace("world", "foo bar");
        // "foo" is inside the replaced region — must be skipped
        tracked.replace("foo", "baz");

        assert_eq!(tracked.text(), "Hello foo bar");
    }

    /// char_map length == transformed codepoint length for nested replacement scenario.
    #[test]
    fn test_nested_replacement_char_map_consistent() {
        let mut tracked = TrackedText::new("Test: 5.2-Codex and 25%");
        let pat1 = Regex::new(r"5\.2-Codex").unwrap();
        tracked.sub(&pat1, |_| "пять точка два Codex".to_string());

        // Normalize whitespace (may find spaces inside the replaced text — must skip)
        let pat2 = Regex::new(r" {2,}").unwrap();
        tracked.sub(&pat2, |_| " ".to_string());

        let mapping = tracked.build_mapping();
        assert_eq!(
            mapping.char_map.len(),
            char_len(&mapping.transformed),
            "char_map len {} != transformed codepoint len {}",
            mapping.char_map.len(),
            char_len(&mapping.transformed)
        );
    }

    /// Python: TestCreateTrackedText::test_create_tracked_text
    #[test]
    fn test_new_tracked_text() {
        let mut tracked = TrackedText::new("Hello world");
        assert_eq!(tracked.text(), "Hello world");
        assert_eq!(tracked.original, "Hello world");
    }

    /// Adjacent disjoint matches within a single batch are all applied: the
    /// splice walk must handle the empty gap between neighboring ranges.
    #[test]
    fn test_batch_adjacent_matches() {
        let mut tracked = TrackedText::new("ab ba");
        let pat = Regex::new("[ab]").unwrap();
        tracked.sub(&pat, |caps| {
            if &caps[0] == "a" {
                "AA".to_string()
            } else {
                "BB".to_string()
            }
        });

        assert_eq!(tracked.text(), "AABB BBAA");
        let mapping = tracked.build_mapping();
        assert_eq!(
            mapping.char_map,
            vec![
                (0, 1),
                (0, 1),
                (1, 2),
                (1, 2),
                (2, 3),
                (3, 4),
                (3, 4),
                (4, 5),
                (4, 5)
            ]
        );
    }

    /// A batch where every replacement equals the matched text is an all-
    /// no-op batch: the text and the identity mapping must be untouched.
    #[test]
    fn test_batch_all_noops_identity() {
        let mut tracked = TrackedText::new("a b a");
        let pat = Regex::new("[ab]").unwrap();
        tracked.sub(&pat, |caps| caps[0].to_string());

        assert_eq!(tracked.text(), "a b a");
        let mapping = tracked.build_mapping();
        for i in 0..5 {
            assert_eq!(mapping.char_map[i], (i, i + 1));
        }
    }
}
