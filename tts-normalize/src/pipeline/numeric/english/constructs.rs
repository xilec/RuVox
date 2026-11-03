//! English wording of the numeric constructs that are not a bare cardinal.
//!
//! The numeric layer recognises a unit, percentage, currency, date, time,
//! version, range, reference or ordinal as one span and reads it in Russian in
//! [`super::super::read`]. This module owns the English wording of the same
//! span: [`read`] returns Latin words with the same shape [`super::cardinal`]
//! returns, and the route renders them: the narrated route transliterates them
//! into Cyrillic through [`super::transliterate`], the native route spells them
//! in Latin.
//!
//! A share of shapes has no wording here on purpose and keeps the Russian
//! reading: phone numbers, Roman numerals, city abbreviations, comma decimals,
//! slash fractions and pre-release version suffixes. [`WORDS`] carries the
//! Cyrillic reading of every word this module can emit, so one lookup in
//! [`super::reading`] covers the numeral and the construct vocabulary.

use super::{cardinal, digits, native, number_words, ordinal};
use crate::pipeline::numeric::lexicon::{Currency, Reference, Scale, Unit, UnitKind};
use crate::pipeline::numeric::parse::{
    self, Approx, Body, Date, Number, Sign, Span, Time, Trailing, TrailingKind,
};

/// The English wording of `span`, or `None` when this layer does not read the
/// construct in English.
pub(crate) fn read(span: &Span, expand_units: bool) -> Option<String> {
    match &span.body {
        Body::Number(number) => read_number(span, number, expand_units),
        Body::Date(date) => date_words(*date),
        Body::YearMonth { year, month } => Some(format!(
            "{} {}",
            month_name(*month)?,
            native(u64::from(*year))
        )),
        Body::Range { left, right } => read_range(span, left, right),
        Body::Reference { kind, number } => read_reference(*kind, number),
        Body::Phone { .. } | Body::City | Body::Roman { .. } => None,
    }
}

/// A bare integer is claimed by the #369 predicate (which owns the code-token
/// guards), so this reader must leave it alone; every other shape is a
/// construct.
fn read_number(span: &Span, number: &Number, expand_units: bool) -> Option<String> {
    let trailing = span.trailing;
    if number.is_plain_integer() && trailing.is_none() && number.currency_prefix.is_none() {
        return None;
    }
    // Comma decimals, slash fractions and pre-release versions keep the
    // Russian reading: their wording is out of scope for this change.
    if number.comma_frac.is_some()
        || !number.slash_groups.is_empty()
        || number.half.is_some()
        || number.version_suffix.is_some()
    {
        return None;
    }
    // A size or duration unit, percentage, currency or scale is a measurement
    // the profile decides about: without `expand_units` the engine reads the
    // symbol itself, so the construct keeps the Russian reading. A currency
    // prefix ("$5") is a measurement in the same sense.
    if !expand_units
        && (number.currency_prefix.is_some()
            || trailing.is_some_and(|trailing| {
                matches!(
                    trailing.kind,
                    TrailingKind::Percent
                        | TrailingKind::Unit(_)
                        | TrailingKind::Currency(_)
                        | TrailingKind::Scale(_)
                )
            }))
    {
        return None;
    }

    let mut words = if number.ordinal_suffix.is_some() {
        ordinal(number.int.parse::<u64>().ok()?)?
    } else if let Some(time) = &number.time {
        time_words(time)?
    } else if number.leading_dot {
        format!("point {}", group_words(&number.int))
    } else if !number.dot_groups.is_empty() {
        dotted_words(number)?
    } else {
        integer_words(number)?
    };

    if let Some(currency) = number.currency_prefix {
        words.push(' ');
        words.push_str(currency_words(currency, !counts_as_one(number)));
    }
    if let Some(trailing) = trailing {
        let attach = trailing_words(trailing, number)?;
        if !attach.is_empty() {
            words.push(' ');
            words.push_str(&attach);
        }
    }
    if let Some(approx) = number.approx {
        words = format!("{} {words}", approx_word(approx));
    }
    if let Some(sign) = number.sign {
        words = format!("{} {words}", sign_word(sign));
    }
    Some(words)
}

/// The words of the integer part, digit by digit for a zero-padded code run.
fn integer_words(number: &Number) -> Option<String> {
    let run = number.int.as_str();
    Some(if super::is_code_run(run) {
        digits(run)
    } else {
        number_words(run)
    })
}

/// One dotted group: a leading zero carries information ("v1.02"), so the
/// group is read digit by digit instead of as a number.
fn group_words(group: &str) -> String {
    if group.len() > 1 && group.starts_with('0') {
        digits(group)
    } else {
        number_words(group)
    }
}

fn dotted_words(number: &Number) -> Option<String> {
    let mut parts = Vec::with_capacity(number.dot_groups.len() + 1);
    parts.push(integer_words(number)?);
    for group in &number.dot_groups {
        parts.push(group_words(group));
    }
    Some(parts.join(" point "))
}

fn time_words(time: &Time) -> Option<String> {
    let hours = time.hours.parse::<u64>().ok()?;
    let minutes = time.minutes.parse::<u64>().ok()?;
    let mut words = cardinal(hours)?;
    if minutes > 0 {
        words.push(' ');
        if minutes < 10 {
            words.push_str("oh ");
        }
        words.push_str(&cardinal(minutes)?);
    }
    if let Some(seconds) = &time.seconds {
        let seconds = seconds.parse::<u64>().ok()?;
        if seconds > 0 {
            words.push_str(" and ");
            words.push_str(&cardinal(seconds)?);
            words.push_str(if seconds == 1 { " second" } else { " seconds" });
        }
    }
    Some(words)
}

fn date_words(date: Date) -> Option<String> {
    let day = ordinal(u64::from(date.day))?;
    let month = month_name(date.month)?;
    Some(match date.year {
        Some(year) => format!("the {day} of {month}, {}", native(u64::from(year))),
        None => format!("the {day} of {month}"),
    })
}

fn read_range(span: &Span, left: &Number, right: &Number) -> Option<String> {
    let left_words = bound_words(left)?;
    let right_words = bound_words(right)?;
    let mut words = format!("{left_words} to {right_words}");
    if let Some(trailing) = span.trailing {
        let attach = trailing_words(trailing, right)?;
        if !attach.is_empty() {
            words.push(' ');
            words.push_str(&attach);
        }
    }
    Some(words)
}

/// A range bound: a calendar year reads in the year register, every other
/// integer as a cardinal; a fractional bound keeps the Russian reading.
fn bound_words(number: &Number) -> Option<String> {
    if !number.is_plain_integer() {
        return None;
    }
    let value = number.int.parse::<u64>().ok()?;
    Some(if parse::is_year(value) {
        native(value)
    } else {
        integer_words(number)?
    })
}

fn read_reference(kind: Reference, number: &Number) -> Option<String> {
    let head = match kind {
        Reference::NumberSign => "number",
        Reference::Page => "page",
        Reference::Table => "table",
        Reference::Figure => "figure",
        Reference::Chapter => "chapter",
        Reference::Phone | Reference::TaxId | Reference::Article => return None,
    };
    Some(format!("{head} {}", integer_words(number)?))
}

fn trailing_words(trailing: Trailing, number: &Number) -> Option<String> {
    let plural = !counts_as_one(number);
    Some(match trailing.kind {
        TrailingKind::Percent => "percent".to_string(),
        TrailingKind::Unit(unit) => unit_words(unit, plural)?.to_string(),
        TrailingKind::Currency(currency) => currency_words(currency, plural).to_string(),
        TrailingKind::Scale(scale) => {
            let mut words = scale_word(scale).to_string();
            if let Some(currency) = trailing.currency {
                words.push(' ');
                words.push_str(currency_words(currency, plural));
            }
            words
        }
        TrailingKind::Plus => "or more".to_string(),
        TrailingKind::YearAbbreviation | TrailingKind::YearRange => return None,
    })
}

/// Whether the count is exactly one, so a count noun takes its singular form.
fn counts_as_one(number: &Number) -> bool {
    number.int == "1"
        && number.comma_frac.is_none()
        && number.dot_groups.is_empty()
        && number.slash_groups.is_empty()
        && number.half.is_none()
}

fn approx_word(approx: Approx) -> &'static str {
    match approx {
        Approx::Tilde => "about",
        Approx::Almost => "roughly",
        Approx::PlusMinus => "plus-minus",
    }
}

fn sign_word(sign: Sign) -> &'static str {
    match sign {
        Sign::Minus => "minus",
        Sign::Plus => "plus",
    }
}

fn month_name(month: u32) -> Option<&'static str> {
    let index = usize::try_from(month).ok()?.checked_sub(1)?;
    MONTHS.get(index).copied()
}

fn unit_words(unit: Unit, plural: bool) -> Option<&'static str> {
    Some(match (unit.forms.kind, plural) {
        (UnitKind::Kilogram, false) => "kilogram",
        (UnitKind::Kilogram, true) => "kilograms",
        (UnitKind::Kilometer, false) => "kilometer",
        (UnitKind::Kilometer, true) => "kilometers",
        (UnitKind::Centimeter, false) => "centimeter",
        (UnitKind::Centimeter, true) => "centimeters",
        (UnitKind::Millimeter, false) => "millimeter",
        (UnitKind::Millimeter, true) => "millimeters",
        (UnitKind::Meter, false) => "meter",
        (UnitKind::Meter, true) => "meters",
        (UnitKind::Liter, false) => "liter",
        (UnitKind::Liter, true) => "liters",
        (UnitKind::Milliliter, false) => "milliliter",
        (UnitKind::Milliliter, true) => "milliliters",
        (UnitKind::Gram, false) => "gram",
        (UnitKind::Gram, true) => "grams",
        (UnitKind::Kilobyte, false) => "kilobyte",
        (UnitKind::Kilobyte, true) => "kilobytes",
        (UnitKind::Megabyte, false) => "megabyte",
        (UnitKind::Megabyte, true) => "megabytes",
        (UnitKind::Gigabyte, false) => "gigabyte",
        (UnitKind::Gigabyte, true) => "gigabytes",
        (UnitKind::Terabyte, false) => "terabyte",
        (UnitKind::Terabyte, true) => "terabytes",
        (UnitKind::Millisecond, false) => "millisecond",
        (UnitKind::Millisecond, true) => "milliseconds",
        (UnitKind::Second, false) => "second",
        (UnitKind::Second, true) => "seconds",
        (UnitKind::Minute, false) => "minute",
        (UnitKind::Minute, true) => "minutes",
        (UnitKind::Hour, false) => "hour",
        (UnitKind::Hour, true) => "hours",
        (UnitKind::Pixel, false) => "pixel",
        (UnitKind::Pixel, true) => "pixels",
        (UnitKind::Em, _) => "em",
        (UnitKind::Rem, _) => "rem",
        (UnitKind::Vh, _) => "vee aitch",
        (UnitKind::Vw, _) => "vee double you",
        (UnitKind::Degree, false) => "degree",
        (UnitKind::Degree, true) => "degrees",
        (UnitKind::Celsius, false) => "degree Celsius",
        (UnitKind::Celsius, true) => "degrees Celsius",
        (UnitKind::KilometersPerHour, false) => "kilometer per hour",
        (UnitKind::KilometersPerHour, true) => "kilometers per hour",
        (UnitKind::MetersPerSecond, false) => "meter per second",
        (UnitKind::MetersPerSecond, true) => "meters per second",
    })
}

fn currency_words(currency: Currency, plural: bool) -> &'static str {
    match (currency, plural) {
        (Currency::Ruble, false) => "ruble",
        (Currency::Ruble, true) => "rubles",
        (Currency::Dollar, false) => "dollar",
        (Currency::Dollar, true) => "dollars",
        (Currency::Euro, false) => "euro",
        (Currency::Euro, true) => "euros",
        (Currency::Kopek, false) => "kopek",
        (Currency::Kopek, true) => "kopeks",
        (Currency::Pound, false) => "pound",
        (Currency::Pound, true) => "pounds",
    }
}

fn scale_word(scale: Scale) -> &'static str {
    match scale {
        Scale::Thousand => "thousand",
        Scale::Million => "million",
        Scale::Billion => "billion",
    }
}

/// The ordinal form of a cardinal word ("one" → "first", "twenty" →
/// "twentieth"), used to build a compound ordinal from its last word.
pub(super) fn ordinal_word(word: &str) -> Option<&'static str> {
    Some(match word {
        "zero" => "zeroth",
        "one" => "first",
        "two" => "second",
        "three" => "third",
        "four" => "fourth",
        "five" => "fifth",
        "six" => "sixth",
        "seven" => "seventh",
        "eight" => "eighth",
        "nine" => "ninth",
        "ten" => "tenth",
        "eleven" => "eleventh",
        "twelve" => "twelfth",
        "thirteen" => "thirteenth",
        "fourteen" => "fourteenth",
        "fifteen" => "fifteenth",
        "sixteen" => "sixteenth",
        "seventeen" => "seventeenth",
        "eighteen" => "eighteenth",
        "nineteen" => "nineteenth",
        "twenty" => "twentieth",
        "thirty" => "thirtieth",
        "forty" => "fortieth",
        "fifty" => "fiftieth",
        "sixty" => "sixtieth",
        "seventy" => "seventieth",
        "eighty" => "eightieth",
        "ninety" => "ninetieth",
        "hundred" => "hundredth",
        "thousand" => "thousandth",
        "million" => "millionth",
        _ => return None,
    })
}

/// The month names, indexed by month number.
const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// The Cyrillic reading of every word this module emits. [`super::reading`]
/// looks a word up here as well as in the numeral tables.
pub(super) const WORDS: &[(&str, &str)] = &[
    // Function words.
    ("the", "зэ"),
    ("of", "ов"),
    ("to", "ту"),
    ("point", "поинт"),
    ("percent", "пёрсэнт"),
    ("or", "о"),
    ("more", "мор"),
    ("about", "эбаут"),
    ("roughly", "рафли"),
    ("minus", "майнус"),
    ("plus", "плас"),
    ("number", "намбер"),
    ("page", "пейдж"),
    ("table", "тейбл"),
    ("figure", "фигё"),
    ("chapter", "чаптер"),
    // Scale words not covered by the cardinal reader.
    ("billion", "биллион"),
    // Units.
    ("kilogram", "килограм"),
    ("kilograms", "килограмс"),
    ("kilometer", "километр"),
    ("kilometers", "километрс"),
    ("centimeter", "сентиметр"),
    ("centimeters", "сентиметрс"),
    ("millimeter", "миллиметр"),
    ("millimeters", "миллиметрс"),
    ("meter", "митэ"),
    ("meters", "митэс"),
    ("liter", "литр"),
    ("liters", "литрс"),
    ("milliliter", "миллилитр"),
    ("milliliters", "миллилитрс"),
    ("gram", "грам"),
    ("grams", "грамс"),
    ("kilobyte", "килобайт"),
    ("kilobytes", "килобайтс"),
    ("megabyte", "мегабайт"),
    ("megabytes", "мегабайтс"),
    ("gigabyte", "гигабайт"),
    ("gigabytes", "гигабайтс"),
    ("terabyte", "терабайт"),
    ("terabytes", "терабайтс"),
    ("millisecond", "миллисеконд"),
    ("milliseconds", "миллисекондс"),
    // Serves the unit ("seconds") and the second ordinal.
    ("second", "секонд"),
    ("seconds", "секондс"),
    ("minute", "минит"),
    ("minutes", "минитс"),
    ("hour", "ауэр"),
    ("hours", "ауэрс"),
    ("pixel", "пиксел"),
    ("pixels", "пикселс"),
    ("em", "эм"),
    ("rem", "рем"),
    ("vee", "ви"),
    ("aitch", "эйч"),
    ("double", "дабл"),
    ("you", "ю"),
    ("degree", "дигри"),
    ("degrees", "дигриз"),
    ("celsius", "целсиус"),
    ("per", "пё"),
    // Currencies.
    ("ruble", "рубл"),
    ("rubles", "рублс"),
    ("dollar", "доллар"),
    ("dollars", "долларс"),
    ("euro", "евро"),
    ("euros", "еврос"),
    ("kopek", "копек"),
    ("kopeks", "копекс"),
    ("pound", "паунд"),
    ("pounds", "паундс"),
    // Months.
    ("january", "джануари"),
    ("february", "фебруари"),
    ("march", "марч"),
    ("april", "эйприл"),
    ("may", "мэй"),
    ("june", "джун"),
    ("july", "джулай"),
    ("august", "оугаст"),
    ("september", "септембер"),
    ("october", "октоубер"),
    ("november", "ноувембер"),
    ("december", "дисембер"),
    // Ordinals.
    ("zeroth", "зирос"),
    ("first", "фёст"),
    ("third", "сёд"),
    ("fourth", "фос"),
    ("fifth", "фифс"),
    ("sixth", "сикст"),
    ("seventh", "сэвенс"),
    ("eighth", "эйтс"),
    ("ninth", "найнс"),
    ("tenth", "тенс"),
    ("eleventh", "илэвенс"),
    ("twelfth", "твелфс"),
    ("thirteenth", "сёртинс"),
    ("fourteenth", "фотинс"),
    ("fifteenth", "фифтинс"),
    ("sixteenth", "сикстинс"),
    ("seventeenth", "сэвентинс"),
    ("eighteenth", "эйтинс"),
    ("nineteenth", "найнтинс"),
    ("twentieth", "твентис"),
    ("thirtieth", "фётис"),
    ("fortieth", "фотис"),
    ("fiftieth", "фифтис"),
    ("sixtieth", "сикстис"),
    ("seventieth", "сэвентис"),
    ("eightieth", "эйтис"),
    ("ninetieth", "найнтис"),
    ("hundredth", "хандрэдс"),
    ("thousandth", "саузендс"),
    ("millionth", "миллионс"),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::numeric::english::transliterate;
    use crate::pipeline::numeric::parse;

    /// The narrated rendering of every construct in `input`: the English
    /// wording transliterated, or the source fragment when this layer has no
    /// wording.
    fn english_all(input: &str) -> String {
        english_all_with(input, true)
    }

    fn english_all_with(input: &str, expand_units: bool) -> String {
        let spans = parse::scan(input, expand_units);
        let mut out = String::new();
        let mut last = 0;
        for span in &spans {
            out.push_str(&input[last..span.start]);
            match read(span, expand_units) {
                Some(words) => out.push_str(&transliterate(&words)),
                None => out.push_str(&input[span.start..span.end]),
            }
            last = span.end;
        }
        out.push_str(&input[last..]);
        out
    }

    #[test]
    fn percent_is_invariant() {
        assert_eq!(english_all("50%"), "фифти пёрсэнт");
        assert_eq!(english_all("1%"), "уан пёрсэнт");
    }

    #[test]
    fn units_agree_with_the_count() {
        assert_eq!(english_all("1 MB"), "уан мегабайт");
        assert_eq!(english_all("50 MB"), "фифти мегабайтс");
        assert_eq!(english_all("100 MB"), "уан хандрэд мегабайтс");
        assert_eq!(english_all("1.5em"), "уан поинт файв эм");
        assert_eq!(english_all("5°C"), "файв дигриз целсиус");
    }

    #[test]
    fn currency_agrees_with_the_count() {
        assert_eq!(english_all("$5"), "файв долларс");
        assert_eq!(english_all("$1"), "уан доллар");
        assert_eq!(english_all("5 €"), "файв еврос");
    }

    #[test]
    fn dates_use_the_year_register() {
        assert_eq!(
            english_all("12.05.2024"),
            "зэ твелфс ов мэй, твенти твенти-фо"
        );
        assert_eq!(english_all("5 марта"), "зэ фифс ов марч");
        assert_eq!(english_all("2024-12"), "дисембер твенти твенти-фо");
    }

    #[test]
    fn times_read_hours_and_minutes() {
        assert_eq!(english_all("09:30"), "найн фёти");
        assert_eq!(english_all("09:05"), "найн оу файв");
        assert_eq!(english_all("12:00"), "твелв");
    }

    #[test]
    fn versions_read_with_point() {
        assert_eq!(english_all("1.2.3"), "уан поинт ту поинт фри");
        assert_eq!(english_all("1.02"), "уан поинт зиро ту");
    }

    #[test]
    fn ranges_read_with_to() {
        assert_eq!(english_all("10-20"), "тен ту твенти");
        assert_eq!(
            english_all("2020–2024"),
            "твенти твенти ту твенти твенти-фо"
        );
        assert_eq!(english_all("10–20%"), "тен ту твенти пёрсэнт");
    }

    #[test]
    fn references_read_the_prefix() {
        assert_eq!(english_all("№ 5"), "намбер файв");
    }

    #[test]
    fn ordinals_read_the_ordinal_word() {
        assert_eq!(english_all("1st release"), "фёст release");
        assert_eq!(english_all("21st release"), "твенти-фёст release");
    }

    #[test]
    fn signs_and_approximations_prefix_the_reading() {
        assert_eq!(english_all("-5%"), "майнус файв пёрсэнт");
        assert_eq!(english_all("~5%"), "эбаут файв пёрсэнт");
        assert_eq!(english_all("5 млн"), "файв миллион");
    }

    #[test]
    fn every_construct_word_has_a_reading() {
        for (latin, cyrillic) in WORDS {
            assert!(
                !cyrillic.is_empty() && !cyrillic.chars().any(|ch| ch.is_ascii_alphabetic()),
                "{latin} has no Cyrillic reading: {cyrillic:?}"
            );
        }
    }

    #[test]
    fn bare_integers_and_out_of_scope_shapes_are_left_alone() {
        assert_eq!(english_all("5"), "5");
        assert_eq!(english_all("1,5"), "1,5");
        assert_eq!(english_all("1/2"), "1/2");
        assert_eq!(english_all("1.2.0-beta"), "1.2.0-beta");
        assert_eq!(english_all_with("100 MB", false), "100 MB");
    }
}
