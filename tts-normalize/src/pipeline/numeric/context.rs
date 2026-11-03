//! English context of a numeric span: the language of its sentence and of the
//! nearest word on each side.

/// Where the neighbour scan stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Side {
    /// English prose.
    English,
    /// A word that is not English prose: Cyrillic, a Latin identifier, a
    /// user-dictionary key, a mixed letter-digit token.
    Word,
    /// The text starts or ends here, or a paragraph breaks.
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

        for (index, &ch) in chars.iter().enumerate() {
            if is_cyrillic(ch) {
                has_cyrillic = true;
            }
            // A terminal mark ends a sentence only before whitespace or the end
            // of the text, so "3.14" and "e.g." stay inside one sentence.
            let terminal =
                is_sentence_end(ch) && chars.get(index + 1).is_none_or(|next| next.is_whitespace());
            let paragraph = ch == '\n' && chars.get(index + 1) == Some(&'\n');
            if terminal || paragraph {
                *cyrillic.last_mut().expect("one sentence") = has_cyrillic;
                has_cyrillic = false;
                starts.push(index + 1);
                cyrillic.push(false);
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

/// Whether the span at `[start, end)` (byte offsets in `text`) is English
/// context: English prose on both sides, or English prose on one side and a
/// sentence edge on the other, inside a sentence without Cyrillic words.
pub(crate) fn english_context(
    text: &str,
    start: usize,
    end: usize,
    original_pos: usize,
    sentences: &SentenceMap,
    is_english: &dyn Fn(&str, (usize, usize)) -> bool,
) -> bool {
    let left = side_before(text, start, is_english);
    let right = side_after(text, end, is_english);
    if left == Side::Word || right == Side::Word {
        return false;
    }
    if left != Side::English && right != Side::English {
        return false;
    }
    !sentences.has_cyrillic_at(original_pos)
}

/// The language of the nearest word before `pos`.
pub(crate) fn side_before(
    text: &str,
    pos: usize,
    is_english: &dyn Fn(&str, (usize, usize)) -> bool,
) -> Side {
    let mut end = pos;
    loop {
        while end > 0 {
            let ch = text[..end].chars().next_back().expect("a char before end");
            if !ch.is_whitespace() {
                break;
            }
            end -= ch.len_utf8();
            if ch == '\n' && text[..end].ends_with('\n') {
                return Side::Edge;
            }
        }
        if end == 0 {
            return Side::Edge;
        }
        let ch = text[..end].chars().next_back().expect("a char before end");
        if is_sentence_end(ch) {
            return Side::Edge;
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
        return classify(&text[start..end], (start, end), is_english);
    }
}

/// The language of the nearest word after `pos`.
pub(crate) fn side_after(
    text: &str,
    pos: usize,
    is_english: &dyn Fn(&str, (usize, usize)) -> bool,
) -> Side {
    let mut start = pos;
    loop {
        while start < text.len() {
            let ch = text[start..].chars().next().expect("a char at start");
            if !ch.is_whitespace() {
                break;
            }
            start += ch.len_utf8();
            if ch == '\n' && text[start..].starts_with('\n') {
                return Side::Edge;
            }
        }
        if start >= text.len() {
            return Side::Edge;
        }
        let ch = text[start..].chars().next().expect("a char at start");
        if is_sentence_end(ch) {
            return Side::Edge;
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
        return classify(&text[start..end], (start, end), is_english);
    }
}

fn classify(
    token: &str,
    range: (usize, usize),
    is_english: &dyn Fn(&str, (usize, usize)) -> bool,
) -> Side {
    if token.chars().any(is_cyrillic) {
        return Side::Word;
    }
    let latin = token.chars().all(|ch| ch.is_ascii_alphabetic());
    if latin && is_english(token, range) {
        Side::English
    } else {
        Side::Word
    }
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

fn is_cyrillic(ch: char) -> bool {
    ('\u{0400}'..='\u{04ff}').contains(&ch) && ch.is_alphabetic()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// English prose: ASCII letters only. The layer passes a predicate that
    /// also rejects identifiers and dictionary keys.
    fn latin(token: &str, _range: (usize, usize)) -> bool {
        token.chars().all(|ch| ch.is_ascii_alphabetic())
    }

    #[test]
    fn sentence_map_marks_only_cyrillic_sentences() {
        let map = SentenceMap::build("In 2024 we shipped. Потом починили.");
        assert!(!map.has_cyrillic_at(3));
        assert!(map.has_cyrillic_at(20));
    }

    #[test]
    fn sentence_map_splits_at_paragraph_breaks() {
        let map = SentenceMap::build("In 2024 we shipped\n\nПотом починили");
        assert!(!map.has_cyrillic_at(3));
        assert!(map.has_cyrillic_at(30));
    }

    #[test]
    fn sentence_map_keeps_a_dot_between_digits() {
        let map = SentenceMap::build("In 3.14 we shipped");
        // The dot is not a sentence end, so the whole text is one sentence.
        assert!(!map.has_cyrillic_at(3));
        assert!(!map.has_cyrillic_at(10));
    }

    #[test]
    fn side_skips_punctuation_and_digits() {
        assert_eq!(side_before("In 2024, 3", 10, &latin), Side::English);
        assert_eq!(side_after("3, 2024 shipped", 1, &latin), Side::English);
    }

    #[test]
    fn side_stops_at_a_sentence_end() {
        assert_eq!(side_after("2024. We shipped", 4, &latin), Side::Edge);
        assert_eq!(side_before("In 2024. 3", 10, &latin), Side::Edge);
    }

    #[test]
    fn side_treats_a_paragraph_break_as_an_edge() {
        assert_eq!(side_after("2024\n\nwe shipped", 4, &latin), Side::Edge);
        assert_eq!(side_before("In 2024\n\n3", 10, &latin), Side::Edge);
    }

    #[test]
    fn side_reads_the_nearest_word() {
        assert_eq!(side_before("In 2024", 7, &latin), Side::English);
        assert_eq!(side_before("Установи 2024", 17, &latin), Side::Word);
        assert_eq!(side_after("2024 releases", 5, &latin), Side::English);
    }

    #[test]
    fn context_needs_english_on_one_side_and_no_word_on_the_other() {
        let english = SentenceMap::build("In 2024 we shipped");
        assert!(english_context(
            "In 2024 we shipped",
            3,
            7,
            3,
            &english,
            &latin
        ));

        let cyrillic = SentenceMap::build("In 2024 we shipped в Казань");
        assert!(!english_context(
            "In 2024 we shipped в Казань",
            3,
            7,
            3,
            &cyrillic,
            &latin
        ));

        let russian = SentenceMap::build("Установи Python 2024");
        assert!(!english_context(
            "Установи Python 2024",
            24,
            28,
            24,
            &russian,
            &latin
        ));
    }

    #[test]
    fn identifier_neighbour_is_not_english_prose() {
        let predicate = |token: &str, _range: (usize, usize)| {
            token.chars().all(|ch| ch.is_ascii_alphabetic()) && token != "getUserData"
        };
        let sentences = SentenceMap::build("getUserData 3");
        assert!(!english_context(
            "getUserData 3",
            12,
            13,
            12,
            &sentences,
            &predicate
        ));
    }
}
