//! English readings of cardinals that sit in English context.
//!
//! [`super::context`] decides *whether* a standalone cardinal is read in
//! English; this module owns *how*. One table carries both the Latin words and
//! their Cyrillic readings, so the native route (digits left to the engine,
//! #370 spells these words into the text) and the narrating route (Cyrillic
//! synthesized by a Russian-only engine) cannot drift apart.

/// Above this value every digit is read separately, the same cutoff the
/// Russian reader applies.
const LIMIT: u64 = 999_999_999;

const ZERO: (&str, &str) = ("zero", "зиро");
const HUNDRED: (&str, &str) = ("hundred", "хандред");
const THOUSAND: (&str, &str) = ("thousand", "саузенд");
const MILLION: (&str, &str) = ("million", "миллион");

const UNITS: [(&str, &str); 20] = [
    ("zero", "зиро"),
    ("one", "уан"),
    ("two", "ту"),
    ("three", "сри"),
    ("four", "фо"),
    ("five", "файв"),
    ("six", "сикс"),
    ("seven", "сэвен"),
    ("eight", "эйт"),
    ("nine", "найн"),
    ("ten", "тен"),
    ("eleven", "илэвен"),
    ("twelve", "твелв"),
    ("thirteen", "сёртин"),
    ("fourteen", "фотин"),
    ("fifteen", "фифтин"),
    ("sixteen", "сикстин"),
    ("seventeen", "сэвентин"),
    ("eighteen", "эйтин"),
    ("nineteen", "найнтин"),
];

const TENS: [(&str, &str); 8] = [
    ("twenty", "твенти"),
    ("thirty", "сёти"),
    ("forty", "фоти"),
    ("fifty", "фифти"),
    ("sixty", "сиксти"),
    ("seventy", "сэвенти"),
    ("eighty", "эйти"),
    ("ninety", "найнти"),
];

/// The cardinal words of `value`, or `None` above [`LIMIT`].
pub(crate) fn cardinal(value: u64) -> Option<String> {
    if value > LIMIT {
        return None;
    }
    Some(below_billion(value))
}

/// The digit-by-digit reading of a run of decimal digits ("007" → "zero zero
/// seven").
pub(crate) fn digits(digits: &str) -> String {
    let mut words: Vec<&str> = Vec::with_capacity(digits.len());
    for digit in digits.chars().filter_map(|ch| ch.to_digit(10)) {
        words.push(UNITS[digit as usize].0);
    }
    words.join(" ")
}

/// The Cyrillic reading of a Latin numeral phrase ("twenty-four" →
/// "твенти-фо").
pub(crate) fn transliterate(latin: &str) -> String {
    let mut out = String::with_capacity(latin.len() * 2);
    let mut word = String::new();
    for ch in latin.chars() {
        if ch.is_ascii_alphabetic() {
            word.push(ch);
        } else {
            push_reading(&mut out, &word);
            word.clear();
            out.push(ch);
        }
    }
    push_reading(&mut out, &word);
    out
}

fn push_reading(out: &mut String, word: &str) {
    if word.is_empty() {
        return;
    }
    match reading(word) {
        Some(cyrillic) => out.push_str(cyrillic),
        None => {
            // Every word this module emits has a reading; a miss means the
            // word table and the reader drifted apart.
            debug_assert!(false, "no Cyrillic reading for the numeral word {word:?}");
            out.push_str(word);
        }
    }
}

fn reading(word: &str) -> Option<&'static str> {
    let table = UNITS
        .iter()
        .chain(TENS.iter())
        .chain([ZERO, HUNDRED, THOUSAND, MILLION].iter());
    table
        .filter(|(latin, _)| *latin == word)
        .map(|(_, cyrillic)| *cyrillic)
        .next()
}

fn below_billion(value: u64) -> String {
    let millions = value / 1_000_000;
    let thousands = (value / 1_000) % 1_000;
    let rest = value % 1_000;

    let mut parts: Vec<String> = Vec::with_capacity(3);
    if millions > 0 {
        parts.push(format!("{} {}", below_thousand(millions), MILLION.0));
    }
    if thousands > 0 {
        parts.push(format!("{} {}", below_thousand(thousands), THOUSAND.0));
    }
    if rest > 0 {
        parts.push(below_thousand(rest));
    }
    if parts.is_empty() {
        ZERO.0.to_string()
    } else {
        parts.join(" ")
    }
}

fn below_thousand(value: u64) -> String {
    let hundreds = value / 100;
    let rest = value % 100;

    let mut parts: Vec<String> = Vec::with_capacity(2);
    if hundreds > 0 {
        parts.push(format!("{} {}", UNITS[hundreds as usize].0, HUNDRED.0));
    }
    if rest > 0 {
        parts.push(below_hundred(rest));
    }
    parts.join(" ")
}

fn below_hundred(value: u64) -> String {
    debug_assert!(value < 100);
    if value < 20 {
        return UNITS[value as usize].0.to_string();
    }
    let tens = TENS[(value / 10 - 2) as usize].0;
    match value % 10 {
        0 => tens.to_string(),
        unit => format!("{tens}-{}", UNITS[unit as usize].0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cardinal_words_cover_the_range() {
        assert_eq!(cardinal(0).as_deref(), Some("zero"));
        assert_eq!(cardinal(7).as_deref(), Some("seven"));
        assert_eq!(cardinal(13).as_deref(), Some("thirteen"));
        assert_eq!(cardinal(21).as_deref(), Some("twenty-one"));
        assert_eq!(cardinal(100).as_deref(), Some("one hundred"));
        assert_eq!(cardinal(101).as_deref(), Some("one hundred one"));
        assert_eq!(cardinal(1000).as_deref(), Some("one thousand"));
        assert_eq!(cardinal(2024).as_deref(), Some("two thousand twenty-four"));
        assert_eq!(
            cardinal(1_000_000).as_deref(),
            Some("one million"),
            "a lone scale word keeps its 'one' in English"
        );
        assert_eq!(
            cardinal(LIMIT).as_deref(),
            Some(
                "nine hundred ninety-nine million nine hundred ninety-nine \
                 thousand nine hundred ninety-nine"
            )
        );
    }

    #[test]
    fn above_the_limit_has_no_cardinal() {
        assert_eq!(cardinal(LIMIT + 1), None);
        assert_eq!(
            digits("1000000000"),
            "one zero zero zero zero zero zero zero zero zero"
        );
    }

    #[test]
    fn every_emitted_word_has_a_reading() {
        for value in 0..=100_000 {
            let reading = transliterate(&cardinal(value).expect("inside the range"));
            assert!(
                reading.chars().all(|ch| !ch.is_ascii_alphabetic()),
                "untransliterated word in {value}: {reading}"
            );
        }
        for value in [1_000_000, 999_999_999] {
            let reading = transliterate(&cardinal(value).expect("inside the range"));
            assert!(reading.chars().all(|ch| !ch.is_ascii_alphabetic()));
        }
    }

    #[test]
    fn transliteration_keeps_the_phrases_apart() {
        assert_eq!(
            transliterate("two thousand twenty-four"),
            "ту саузенд твенти-фо"
        );
        assert_eq!(transliterate("one hundred one"), "уан хандред уан");
    }
}
