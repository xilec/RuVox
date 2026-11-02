//! Numeric span layer: one candidate pass that classifies and reads every
//! numeric fragment in the text.
//!
//! Before this layer the pipeline had one regex phase per numeric shape
//! (sizes, dates, times, percentage ranges, percentages, ranges, versions,
//! leading-dot decimals, bare numbers) and every phase raced the others for the
//! same digits: "10-20%" had to be consumed by the percentage-range phase
//! before the percentage phase claimed "20%", versions had to run before
//! numbers, and every new shape meant another ordering constraint. The layer
//! replaces that set with a single scan over the text:
//!
//! 1. [`parse`] finds maximal numeric candidates, including the separators that
//!    belong to a number (space/NBSP digit groups, decimal comma/dot, minus and
//!    plus signs, range dashes, slash dates, time colons, version dots,
//!    ordinal suffixes) and the words that belong to the construct (units,
//!    currencies, scale words, month names, reference prefixes).
//! 2. [`read`] classifies each candidate by its shape and neighbouring context
//!    and asks [`morphology`] for the spoken form in the required case and
//!    gender.
//!
//! One pass means no cross-phase races: a construct is either recognised as a
//! whole or not at all, and a fragment another phase owns (a URL, an
//! identifier) is skipped by the start guards instead of being partially
//! consumed.

mod lexicon;
mod morphology;
mod parse;
mod read;

use crate::pipeline::tracked_text::TrackedText;

/// The phase entry point: rewrite every numeric span of `tracked`.
///
/// `expand_units` mirrors the engine profile: an engine that reads unit symbols
/// itself keeps them, so a span carrying a unit, percentage, currency or scale
/// word is left untouched and its digits stay with the symbol.
pub struct NumericSpanNormalizer;

impl NumericSpanNormalizer {
    pub fn new() -> Self {
        Self
    }

    pub fn process(&self, tracked: &mut TrackedText, expand_units: bool) {
        let snapshot = tracked.text().to_string();
        let replacements: Vec<(usize, usize, String)> = parse::scan(&snapshot, expand_units)
            .iter()
            .filter_map(|span| {
                span.read(&snapshot, expand_units)
                    .map(|reading| (span.start, span.end, reading))
            })
            .collect();
        if replacements.is_empty() {
            return;
        }
        tracked.replace_byte_ranges(replacements);
    }
}

impl Default for NumericSpanNormalizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Rewrite every span the layer recognises, leaving unrecognised text as
    /// it is. Mirrors [`NumericSpanNormalizer::process`] without a pipeline.
    fn read_all(input: &str, units: bool) -> String {
        let spans = parse::scan(input, units);
        let mut out = String::new();
        let mut last = 0;
        for span in &spans {
            out.push_str(&input[last..span.start]);
            match span.read(input, units) {
                Some(reading) => out.push_str(&reading),
                None => out.push_str(&input[span.start..span.end]),
            }
            last = span.end;
        }
        out.push_str(&input[last..]);
        out
    }

    #[test]
    fn grouped_integers_use_the_whole_group() {
        assert_eq!(read_all("10 000", true), "десять тысяч");
        assert_eq!(read_all("10\u{a0}000", true), "десять тысяч");
        assert_eq!(read_all("1 500", true), "тысяча пятьсот");
        assert_eq!(
            read_all("1 234,56", true),
            "одна тысяча двести тридцать четыре целых пятьдесят шесть сотых"
        );
    }

    #[test]
    fn ranges_read_both_bounds() {
        // En dash as in prose: the tracked-text dash phase runs after this
        // layer, so a span keeps the source dash.
        assert_eq!(read_all("5–10 кг", true), "от пяти до десяти килограммов");
        assert_eq!(read_all("10–20%", true), "от десяти до двадцати процентов");
        assert_eq!(
            read_all("2010–2015 гг.", true),
            "с две тысячи десятого по две тысячи пятнадцатый год"
        );
        assert_eq!(
            read_all("20–30 °C", true),
            "от двадцати до тридцати градусов Цельсия"
        );
        assert_eq!(
            read_all("1,5–2 кг", true),
            "от полутора до двух килограммов"
        );
    }

    #[test]
    fn decimals_and_fractions_read_naturally() {
        assert_eq!(read_all("1,5 кг", true), "полтора килограмма");
        assert_eq!(
            read_all("0,25 л", true),
            "ноль целых двадцать пять сотых литра"
        );
        assert_eq!(read_all("3.14", true), "три точка четырнадцать");
        assert_eq!(read_all(".5", true), "ноль точка пять");
        assert_eq!(read_all("1/2", true), "одна вторая");
    }

    #[test]
    fn dates_and_times_read_as_calendar_values() {
        assert_eq!(
            read_all("31.12.2024", true),
            "тридцать первое декабря две тысячи двадцать четвёртого года"
        );
        assert_eq!(
            read_all("с 5 марта по 10 апреля", true),
            "с пятого марта по десятое апреля"
        );
        assert_eq!(read_all("10:30", true), "десять часов тридцать минут");
        assert_eq!(
            read_all("с 10:00 до 18:00", true),
            "с десяти до восемнадцати часов"
        );
        assert_eq!(
            read_all("2024-12", true),
            "декабрь две тысячи двадцать четвёртого года"
        );
        assert_eq!(
            read_all("с 1990 по 2020 годы", true),
            "с тысяча девятьсот девяностого по две тысячи двадцатый год"
        );
    }

    #[test]
    fn ordinals_inherit_case_from_context() {
        assert_eq!(read_all("5-й день", true), "пятый день");
        assert_eq!(read_all("из 5-й главы", true), "из пятой главы");
        assert_eq!(read_all("40-й", true), "сороковой");
        assert_eq!(read_all("в 1990-х", true), "в девяностых");
        assert_eq!(read_all("в 5-х классах", true), "в пятых классах");
    }

    #[test]
    fn money_and_scale_words_are_owned_together() {
        assert_eq!(read_all("$5", true), "пять долларов");
        assert_eq!(read_all("5 тыс. ₽", true), "пять тысяч рублей");
        assert_eq!(read_all("1,5 млн $", true), "полтора миллиона долларов");
        assert_eq!(read_all("10 000 рублей", true), "десять тысяч рублей");
    }

    #[test]
    fn units_native_leaves_measurements_to_the_engine() {
        assert_eq!(read_all("5 кг", false), "5 кг");
        assert_eq!(read_all("50–60%", false), "50–60%");
        // A bare number without a unit is still the layer's own.
        assert_eq!(read_all("5", false), "пять");
    }
}
