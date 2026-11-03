//! English context of a numeric span: the language of its sentence and the
//! nearest word on each side.

/// How far a neighbour scan goes before it gives up. A number separated from
/// every word by a long run of digits or punctuation is not "between English
/// words", and an unbounded scan over a run of integers would make the layer
/// quadratic in the length of the text.
pub(crate) const NEIGHBOUR_WINDOW: usize = 128;

/// A word the neighbour scan found, in snapshot byte offsets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Word {
    pub start: usize,
    pub end: usize,
}

/// The language of one side of a numeric span.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Side {
    /// English prose.
    English,
    /// A word that is not English prose: a Cyrillic word, a Latin identifier, a
    /// user-dictionary key, a mixed letter-digit token.
    Word,
    /// The text, a sentence or a paragraph ends here.
    Edge,
}

/// Which sentences of the original text contain a Cyrillic word.
///
/// Built over the original input, not over the current snapshot: phases before
/// the numeric one insert Russian words (a markdown list marker becomes
/// "первое:"), and an English sentence must not look Russian because of a word
/// the pipeline itself put there.
pub(crate) struct SentenceMap {
    /// Char offset of each sentence start, ascending.
    starts: Vec<usize>,
    /// Whether the sentence at the same index contains a Cyrillic word.
    cyrillic: Vec<bool>,
}

impl SentenceMap {
    pub(crate) fn build(original: &str) -> Self {
        let chars: Vec<char> = original.chars().collect();
        let mut starts = vec![0usize];
        let mut cyrillic = vec![false];
        let mut has_cyrillic = false;
        // A blank line is a paragraph break; a single newline is whitespace, so
        // a soft-wrapped line stays in the same sentence. Counting newlines
        // also covers CRLF input, where "\r" sits between them.
        let mut pending_newline = false;

        for (index, &ch) in chars.iter().enumerate() {
            if is_cyrillic(ch) {
                has_cyrillic = true;
            }
            // A terminal mark ends a sentence only before whitespace or the end
            // of the text, so "3.14" and "e.g." stay inside one sentence.
            let terminal =
                is_sentence_end(ch) && chars.get(index + 1).is_none_or(|next| next.is_whitespace());
            let paragraph = if ch == '\n' {
                let break_here = pending_newline;
                pending_newline = true;
                break_here
            } else {
                if !ch.is_whitespace() {
                    pending_newline = false;
                }
                false
            };
            if terminal || paragraph {
                *cyrillic.last_mut().expect("one sentence") = has_cyrillic;
                has_cyrillic = false;
                starts.push(index + 1);
                cyrillic.push(false);
                pending_newline = false;
            }
        }
        *cyrillic.last_mut().expect("one sentence") = has_cyrillic;

        // A text that ends with a sentence break opens one empty sentence.
        if starts.len() > 1 && *starts.last().expect("one sentence") >= chars.len() {
            starts.pop();
            cyrillic.pop();
        }

        Self { starts, cyrillic }
    }

    /// Whether the sentence containing `char_pos` has a Cyrillic word.
    pub(crate) fn has_cyrillic_at(&self, char_pos: usize) -> bool {
        let index = self
            .starts
            .partition_point(|&start| start <= char_pos)
            .max(1)
            - 1;
        self.cyrillic[index]
    }
}

/// Prefix counts of prose letters in the original text, in char units.
///
/// A pipeline-inserted word (a markdown list marker becomes "первое:") is not
/// a neighbour, and the only way to tell it from a real word is to look at the
/// original fragment it came from: digits or punctuation mean insertion. That
/// check used to scan the original from its first char for every number; the
/// prefix makes it O(1) and keeps the context check linear in the text.
pub(crate) struct LetterPrefix {
    /// `counts[i]` is the number of prose letters before char `i`.
    counts: Vec<u32>,
}

impl LetterPrefix {
    pub(crate) fn build(original: &str) -> Self {
        let mut counts = Vec::with_capacity(original.chars().count() + 1);
        counts.push(0);
        let mut count = 0u32;
        for ch in original.chars() {
            count += u32::from(is_cyrillic(ch) || ch.is_ascii_alphabetic());
            counts.push(count);
        }
        Self { counts }
    }

    /// Whether the original fragment `[start, end)` holds a prose letter.
    pub(crate) fn has_letters(&self, start: usize, end: usize) -> bool {
        let end = end.min(self.counts.len().saturating_sub(1));
        let start = start.min(end);
        self.counts[end] > self.counts[start]
    }
}

/// The nearest word before `pos`, or `None` when the text, a sentence or a
/// paragraph ends first.
///
/// Whitespace, punctuation and digits are transparent: "In 2024, 3" finds "In"
/// before the second number. The scan is bounded by [`NEIGHBOUR_WINDOW`], so a
/// long run of digits reads as an edge, not as an ever-growing search.
pub(crate) fn word_before(text: &str, pos: usize) -> Option<Word> {
    let mut end = pos;
    let mut newlines = 0u32;
    while pos.saturating_sub(end) <= NEIGHBOUR_WINDOW {
        if end == 0 {
            return None;
        }
        let ch = text[..end]
            .chars()
            .next_back()
            .expect("a char before the cursor");
        if ch.is_whitespace() {
            if ch == '\n' {
                newlines += 1;
                if newlines >= 2 {
                    return None;
                }
            }
            end -= ch.len_utf8();
            continue;
        }
        newlines = 0;
        if is_sentence_end(ch) {
            return None;
        }
        if ch.is_ascii_digit() {
            end -= 1;
            continue;
        }
        if !ch.is_alphanumeric() {
            end -= ch.len_utf8();
            continue;
        }
        let start = word_start_before(text, end);
        return Some(Word { start, end });
    }
    None
}

/// The nearest word after `pos`, or `None` when the text, a sentence or a
/// paragraph ends first.
pub(crate) fn word_after(text: &str, pos: usize) -> Option<Word> {
    let mut start = pos;
    let mut newlines = 0u32;
    while start.saturating_sub(pos) <= NEIGHBOUR_WINDOW {
        if start >= text.len() {
            return None;
        }
        let ch = text[start..].chars().next().expect("a char at the cursor");
        if ch.is_whitespace() {
            if ch == '\n' {
                newlines += 1;
                if newlines >= 2 {
                    return None;
                }
            }
            start += ch.len_utf8();
            continue;
        }
        newlines = 0;
        if is_sentence_end(ch) {
            return None;
        }
        if ch.is_ascii_digit() {
            start += 1;
            continue;
        }
        if !ch.is_alphanumeric() {
            start += ch.len_utf8();
            continue;
        }
        let end = word_end_after(text, start);
        return Some(Word { start, end });
    }
    None
}

fn word_start_before(text: &str, end: usize) -> usize {
    let mut start = end;
    while start > 0 {
        let ch = text[..start]
            .chars()
            .next_back()
            .expect("a char before start");
        if ch.is_alphanumeric() || ch == '_' {
            start -= ch.len_utf8();
        } else {
            break;
        }
    }
    start
}

fn word_end_after(text: &str, start: usize) -> usize {
    let mut end = start;
    while end < text.len() {
        let ch = text[end..].chars().next().expect("a char at end");
        if ch.is_alphanumeric() || ch == '_' {
            end += ch.len_utf8();
        } else {
            break;
        }
    }
    end
}

fn is_sentence_end(ch: char) -> bool {
    matches!(ch, '.' | '!' | '?' | '…')
}

/// A Cyrillic letter; the sentence guard and the pipeline-insertion check use
/// the same definition.
pub(crate) fn is_cyrillic(ch: char) -> bool {
    ('\u{0400}'..='\u{04ff}').contains(&ch) && ch.is_alphabetic()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sentence_map_marks_only_cyrillic_sentences() {
        let map = SentenceMap::build("In 2024 we shipped. Потом починили.");
        assert!(!map.has_cyrillic_at(3));
        assert!(map.has_cyrillic_at(20));
    }

    #[test]
    fn letter_prefix_tells_digits_from_words() {
        let prefix = LetterPrefix::build("1. Python 3.");
        // "1." is a markdown list marker the pipeline turns into a label.
        assert!(!prefix.has_letters(0, 2));
        assert!(prefix.has_letters(3, 9));
        assert!(!prefix.has_letters(10, 11));
        assert!(prefix.has_letters(0, usize::MAX));
        assert!(!prefix.has_letters(10, 10));
    }

    #[test]
    fn sentence_map_splits_at_paragraph_breaks() {
        let map = SentenceMap::build("In 2024 we shipped\n\nПотом починили");
        assert!(!map.has_cyrillic_at(3));
        assert!(map.has_cyrillic_at(30));
    }

    #[test]
    fn sentence_map_splits_at_crlf_paragraph_breaks() {
        let map = SentenceMap::build("In 2024 we shipped\r\n\r\nПотом починили");
        assert!(!map.has_cyrillic_at(3));
        assert!(map.has_cyrillic_at(32));
    }

    #[test]
    fn sentence_map_keeps_a_dot_between_digits() {
        let map = SentenceMap::build("In 3.14 we shipped");
        // The dot is not a sentence end, so the whole text is one sentence.
        assert!(!map.has_cyrillic_at(3));
        assert!(!map.has_cyrillic_at(10));
    }

    #[test]
    fn word_before_skips_punctuation_and_digits() {
        assert_eq!(
            word_before("In 2024, 3", 10),
            Some(Word { start: 0, end: 2 })
        );
        assert_eq!(
            word_before("Установи 2024", 17),
            Some(Word { start: 0, end: 16 })
        );
    }

    #[test]
    fn word_after_skips_punctuation_and_digits() {
        assert_eq!(
            word_after("3, 2024 shipped", 1),
            Some(Word { start: 8, end: 15 })
        );
    }

    #[test]
    fn word_scans_stop_at_a_sentence_end() {
        assert_eq!(word_after("2024. We shipped", 4), None);
        assert_eq!(word_before("In 2024. 3", 10), None);
    }

    #[test]
    fn word_scans_stop_at_a_paragraph_break() {
        assert_eq!(word_after("2024\n\nwe shipped", 4), None);
        assert_eq!(word_before("In 2024\n\n3", 10), None);
    }

    #[test]
    fn word_scans_stop_at_a_crlf_paragraph_break() {
        assert_eq!(word_after("2024\r\n\r\nwe shipped", 4), None);
        assert_eq!(word_before("In 2024\r\n\r\n3", 11), None);
    }

    #[test]
    fn a_long_digit_run_reads_as_an_edge() {
        let digits = "1 ".repeat(NEIGHBOUR_WINDOW);
        let text = format!("word {digits}");
        assert_eq!(word_before(&text, text.len()), None);
        let text = format!("{digits}word");
        assert_eq!(word_after(&text, 0), None);
    }
}
