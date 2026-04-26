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
#[derive(Debug, Clone)]
struct OffsetEntry {
    /// Codepoint position in current text at the time of replacement.
    #[allow(dead_code)]
    current_pos: usize,
    orig_start: usize, // codepoint index in original
    orig_end: usize,   // codepoint index in original (exclusive)
    new_len: usize,    // codepoint count of replacement text
}

// ---- Helper: convert between byte and codepoint indices ----

/// Convert a byte offset in a string to its codepoint index.
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
fn char_len(s: &str) -> usize {
    s.chars().count()
}

/// Text wrapper that tracks all modifications for precise position mapping.
///
/// All position tracking uses **Unicode codepoint** indices, not byte offsets,
/// so multi-byte characters (Cyrillic, emoji, etc.) count as one position each.
pub struct TrackedText {
    pub original: String,
    current: String,
    replacements: Vec<Replacement>,
    offset_entries: Vec<OffsetEntry>,
    sorted_entries_cache: Option<Vec<OffsetEntry>>,
}

impl TrackedText {
    pub fn new(text: impl Into<String>) -> Self {
        let s = text.into();
        Self {
            original: s.clone(),
            current: s,
            replacements: Vec::new(),
            offset_entries: Vec::new(),
            sorted_entries_cache: None,
        }
    }

    pub fn text(&self) -> &str {
        &self.current
    }

    pub fn len(&self) -> usize {
        char_len(&self.current)
    }

    pub fn is_empty(&self) -> bool {
        self.current.is_empty()
    }

    /// Replace all literal occurrences of `from` with `to`, tracking positions.
    ///
    /// Equivalent to Python `TrackedText.replace(old, new)`.
    pub fn replace(&mut self, from: &str, to: &str) {
        let pattern =
            regex::Regex::new(&regex::escape(from)).expect("regex::escape produces valid pattern");
        self.sub(&pattern, |_| to.to_string());
    }

    /// Replace exactly one byte range `[byte_start, byte_end)` in the current text.
    ///
    /// This allows callers to replace a single occurrence without constructing a
    /// literal string that might match elsewhere. Used by markdown link stripping
    /// to remove the leading `[` and the trailing `](url)` independently, so that
    /// the link-text characters retain individual original-position entries and
    /// can still be normalised by later pipeline phases.
    pub fn replace_byte_range(&mut self, byte_start: usize, byte_end: usize, to: &str) {
        let old_text = &self.current[byte_start..byte_end];
        if old_text == to {
            return;
        }

        let char_start = byte_to_char_idx(&self.current, byte_start);
        let char_end = byte_to_char_idx(&self.current, byte_end);

        // Skip if any codepoint in the match is inside an existing replacement.
        let already_replaced = (char_start..char_end)
            .any(|pos| self.is_current_char_pos_inside_replacement(pos));
        if already_replaced {
            return;
        }

        let orig_start = self.current_to_original(char_start);
        let orig_end = if char_end > char_start {
            self.current_to_original(char_end - 1) + 1
        } else {
            orig_start
        };

        if self.find_containing_replacement(orig_start, orig_end).is_some() {
            return;
        }

        self.replacements.push(Replacement {
            orig_start,
            orig_end,
            new_text: to.to_string(),
        });
        self.offset_entries.insert(
            0,
            OffsetEntry {
                current_pos: char_start,
                orig_start,
                orig_end,
                new_len: char_len(to),
            },
        );
        self.sorted_entries_cache = None;

        self.current = format!(
            "{}{}{}",
            &self.current[..byte_start],
            to,
            &self.current[byte_end..]
        );
    }

    /// Regex substitution with a callback, tracking positions for `CharMapping`.
    ///
    /// Matches that overlap already-replaced regions are skipped — exactly
    /// replicating Python `TrackedText.sub` behaviour.
    pub fn sub<F>(&mut self, pattern: &regex::Regex, mut replacer: F)
    where
        F: FnMut(&regex::Captures) -> String,
    {
        // Snapshot current text for iteration (avoid borrow issues).
        let snapshot = self.current.clone();

        // Collect all matches with their new texts.
        let matches: Vec<(usize, usize, String)> = pattern
            .captures_iter(&snapshot)
            .map(|caps| {
                let m = caps.get(0).unwrap();
                let new_text = replacer(&caps);
                (m.start(), m.end(), new_text)
            })
            .collect();

        // Process in reverse order so positions don't shift under us.
        for (byte_start, byte_end, new_text) in matches.into_iter().rev() {
            let old_text = &self.current[byte_start..byte_end];

            // Skip no-op replacements.
            if new_text == old_text {
                continue;
            }

            // Convert byte positions to codepoint positions.
            let char_start = byte_to_char_idx(&self.current, byte_start);
            let char_end = byte_to_char_idx(&self.current, byte_end);

            // Check if any codepoint in the match is inside an existing replacement.
            let mut match_touches_replacement = false;
            for pos in char_start..char_end {
                if self.is_current_char_pos_inside_replacement(pos) {
                    match_touches_replacement = true;
                    break;
                }
            }
            if match_touches_replacement {
                continue;
            }

            // Map current codepoint positions to original codepoint positions.
            let orig_start = self.current_to_original(char_start);
            let orig_end = if char_end > char_start {
                self.current_to_original(char_end - 1) + 1
            } else {
                orig_start
            };

            // Skip if this range overlaps an existing replacement in original coords.
            if self.find_containing_replacement(orig_start, orig_end).is_some() {
                continue;
            }

            // Record replacement.
            self.replacements.push(Replacement {
                orig_start,
                orig_end,
                new_text: new_text.clone(),
            });

            // Insert at front (Python inserts at index 0 when processing in reverse).
            self.offset_entries.insert(
                0,
                OffsetEntry {
                    current_pos: char_start,
                    orig_start,
                    orig_end,
                    new_len: char_len(&new_text),
                },
            );
            self.sorted_entries_cache = None;

            // Apply replacement to current text.
            self.current = format!(
                "{}{}{}",
                &self.current[..byte_start],
                new_text,
                &self.current[byte_end..]
            );
        }
    }

    /// Build the `CharMapping` from all accumulated replacements.
    pub fn build_mapping(self) -> CharMapping {
        let orig_char_len = char_len(&self.original);
        let trans_char_len = char_len(&self.current);

        if self.replacements.is_empty() {
            let char_map: Vec<(usize, usize)> =
                (0..trans_char_len).map(|i| (i, i + 1)).collect();
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

    // ---- Internal helpers ----

    fn get_sorted_entries(&mut self) -> &[OffsetEntry] {
        if self.sorted_entries_cache.is_none() {
            let mut sorted = self.offset_entries.clone();
            sorted.sort_by_key(|e| e.orig_start);
            self.sorted_entries_cache = Some(sorted);
        }
        self.sorted_entries_cache.as_deref().unwrap()
    }

    /// Returns `true` if the codepoint `current_pos` is inside the new text of any
    /// existing replacement (checked in current-text codepoint coordinates).
    fn is_current_char_pos_inside_replacement(&mut self, current_pos: usize) -> bool {
        let sorted = self.get_sorted_entries().to_vec();
        let mut cumulative_delta: i64 = 0;

        for entry in &sorted {
            let old_len = (entry.orig_end - entry.orig_start) as i64;
            let new_len = entry.new_len as i64;
            let delta = new_len - old_len;

            let current_start = (entry.orig_start as i64 + cumulative_delta) as usize;
            let current_end = current_start + entry.new_len;

            if current_pos < current_start {
                return false;
            } else if current_pos < current_end {
                return true;
            } else {
                cumulative_delta += delta;
            }
        }

        false
    }

    /// Find an existing replacement whose original range overlaps `[orig_start, orig_end)`.
    fn find_containing_replacement(
        &self,
        orig_start: usize,
        orig_end: usize,
    ) -> Option<&OffsetEntry> {
        for entry in &self.offset_entries {
            if orig_start == orig_end {
                if entry.orig_start <= orig_start && orig_start < entry.orig_end {
                    return Some(entry);
                }
            } else if orig_start < entry.orig_end && entry.orig_start < orig_end {
                return Some(entry);
            }
        }
        None
    }

    /// Convert a codepoint position in the current text to the corresponding
    /// codepoint position in the original text.
    ///
    /// Negative intermediates are saturated to 0: a non-monotone replacement
    /// chain could in principle produce `cumulative_delta` whose magnitude
    /// exceeds the position being mapped.  Mapping such cases to 0 keeps the
    /// result a valid index instead of wrapping into a huge usize.
    fn current_to_original(&mut self, current_pos: usize) -> usize {
        let sorted = self.get_sorted_entries().to_vec();
        let mut cumulative_delta: i64 = 0;

        let saturating_apply = |pos: usize, delta: i64| -> usize {
            (pos as i64).saturating_sub(delta).max(0) as usize
        };

        for entry in &sorted {
            let old_len = (entry.orig_end - entry.orig_start) as i64;
            let new_len = entry.new_len as i64;
            let delta = new_len - old_len;

            let current_start = (entry.orig_start as i64)
                .saturating_add(cumulative_delta)
                .max(0) as usize;
            let current_end = current_start + entry.new_len;

            if current_pos < current_start {
                return saturating_apply(current_pos, cumulative_delta);
            } else if current_pos < current_end {
                return entry.orig_start;
            } else {
                cumulative_delta += delta;
            }
        }

        saturating_apply(current_pos, cumulative_delta)
    }
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
        let mir_norm_cp = mapping.transformed.find("мир").map(|b| byte_to_char_idx(&mapping.transformed, b)).unwrap();
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
            assert!(orig_end <= 3, "orig_end {} should be <= 3 (len of 'API')", orig_end);
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
        let privet_norm_cp = byte_to_char_idx(&mapping.transformed, mapping.transformed.find("Привет").unwrap());
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
        let tracked = TrackedText::new("Hello world");
        assert_eq!(tracked.text(), "Hello world");
        assert_eq!(tracked.original, "Hello world");
    }
}
