//! English readings of cardinals that sit in English context.
//!
//! [`super::context`] decides *whether* a standalone cardinal is read in
//! English; this module owns *how*. One table carries both the Latin words and
//! their Cyrillic readings, so the native route (the Latin words replace the
//! digits in the text) and the narrating route (Cyrillic synthesized by a
//! Russian-only engine) cannot drift apart.
//!
//! [`constructs`] owns the wording of a construct that is not a bare cardinal
//! (a unit, a date, an ordinal, a version); its word table joins [`reading`],
//! so one lookup serves every word this layer emits.

pub(crate) mod constructs;

/// Above this value every digit is read separately, the same cutoff the
/// Russian reader applies.
pub(crate) const LIMIT: u64 = 999_999_999;

const ZERO: (&str, &str) = ("zero", "зиро");
const HUNDRED: (&str, &str) = ("hundred", "хандред");
const THOUSAND: (&str, &str) = ("thousand", "саузенд");
const MILLION: (&str, &str) = ("million", "миллион");
/// The filler of a year's one-digit tail ("nineteen oh five"); only the native
/// register emits it, but it carries a reading so any reader can say it.
const OH: (&str, &str) = ("oh", "оу");

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

/// The Latin words of `value` for the native prose route, where the engine reads
/// Latin natively and the pipeline writes the spoken form into the text.
///
/// A four-digit value the Russian reader treats as a year (1000–2199) reads as
/// two two-digit pairs ("twenty twenty-four", "nineteen oh five", "nineteen
/// hundred") — because the engine's natural reading of a year is the pair form
/// and a bare four-digit number in prose is far more often a year than a count.
/// A round thousand keeps the cardinal ("two thousand"), and every other value
/// reads as a cardinal.
pub(crate) fn native(value: u64) -> String {
    year(value).unwrap_or_else(|| cardinal(value).expect("a cardinal below the limit"))
}

fn year(value: u64) -> Option<String> {
    // The same year range the Russian reader uses (`parse::is_year`), so the
    // layer has one definition of a calendar year.
    if !super::parse::is_year(value) || value % 1000 == 0 {
        return None;
    }
    let mut words = cardinal(value / 100)?;
    let last = value % 100;
    match last {
        0 => words.push_str(" hundred"),
        1..=9 => {
            words.push_str(" oh ");
            words.push_str(&cardinal(last)?);
        }
        _ => {
            words.push(' ');
            words.push_str(&cardinal(last)?);
        }
    }
    Some(words)
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

/// The English words of a digit run: a cardinal while the reader names the
/// value, the digit-by-digit reading otherwise.
pub(crate) fn number_words(run: &str) -> String {
    match run.parse::<u64>() {
        Ok(value) if value <= LIMIT => cardinal(value).expect("a value inside the limit"),
        _ => digits(run),
    }
}

/// The English ordinal words of `value` ("twenty-first", "one hundredth").
pub(crate) fn ordinal(value: u64) -> Option<String> {
    let words = cardinal(value)?;
    let split = words.rfind([' ', '-']).map(|index| index + 1).unwrap_or(0);
    let (head, last) = words.split_at(split);
    let last = constructs::ordinal_word(last)?;
    Some(format!("{head}{last}"))
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
        .chain([ZERO, HUNDRED, THOUSAND, MILLION, OH].iter())
        .chain(constructs::WORDS.iter());
    table
        .filter(|(latin, _)| latin.eq_ignore_ascii_case(word))
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
    fn native_spells_years_as_pairs() {
        assert_eq!(native(3), "three");
        assert_eq!(native(1900), "nineteen hundred");
        assert_eq!(native(1905), "nineteen oh five");
        assert_eq!(native(1999), "nineteen ninety-nine");
        assert_eq!(native(2000), "two thousand");
        assert_eq!(native(2024), "twenty twenty-four");
        assert_eq!(native(2099), "twenty ninety-nine");
        assert_eq!(native(1000), "one thousand");
        assert_eq!(native(1009), "ten oh nine");
        assert_eq!(native(2009), "twenty oh nine");
        assert_eq!(native(2100), "twenty-one hundred");
        assert_eq!(native(10000), "ten thousand");
        assert_eq!(native(1234), "twelve thirty-four");
        assert_eq!(native(1_000_000), "one million");
    }

    #[test]
    fn every_native_word_has_a_reading() {
        // `push_reading` asserts in debug builds when a word has no table entry;
        // the assertion below also covers release runs and pins the readings.
        for value in [
            3, 1000, 1900, 1905, 1999, 2000, 2009, 2024, 2100, 1234, 1_000_000,
        ] {
            let reading = transliterate(&native(value));
            assert!(
                reading.chars().all(|ch| !ch.is_ascii_alphabetic()),
                "{value} left a Latin word in {reading:?}"
            );
        }
        assert_eq!(transliterate(&native(1905)), "найнтин оу файв");
        assert_eq!(transliterate(&native(2024)), "твенти твенти-фо");
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
