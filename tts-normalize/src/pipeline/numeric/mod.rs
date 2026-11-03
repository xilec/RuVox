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

mod context;
mod english;
mod lexicon;
mod morphology;
mod parse;
mod read;

use self::english::constructs;
use crate::dictionary::UserDictionary;
use crate::pipeline::spans::{RecordedSpan, SpanKind, byte_to_char_offsets, record_byte_ranges};
use crate::pipeline::tracked_text::TrackedText;

/// How the pipeline renders a number the layer reads in English.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NumberRoute {
    /// Transliterate the English reading into Cyrillic.
    Narrate,
    /// Leave the digits to the engine and annotate them.
    Native,
}

/// The pipeline decisions the numeric layer needs: the route decides how an
/// English reading is rendered, the dictionary decides what counts as English
/// prose, and the recorders collect native spans and the original ranges the
/// layer spelled.
pub(crate) struct NumericRouting<'a> {
    pub route: NumberRoute,
    /// Language a native fragment is annotated with.
    pub native_lang: Option<&'static str>,
    /// User-dictionary keys are not English prose.
    pub dictionary: &'a UserDictionary,
    pub recorded: &'a mut Vec<RecordedSpan>,
    /// Original ranges the native route replaced with spelled words; a later
    /// identifier scan must not relabel the spelled text.
    pub claimed: &'a mut Vec<(usize, usize)>,
}

/// The phase entry point: rewrite every numeric span of `tracked`.
///
/// `expand_units` mirrors the engine profile: an engine that reads unit symbols
/// itself keeps them, so a span carrying a unit, percentage, currency or scale
/// word is left untouched and its digits stay with the symbol.
pub struct NumericSpanNormalizer;

/// What a claimed span reads as.
enum EnglishNumber {
    /// A value the cardinal reader covers; the route picks the register.
    Cardinal(u64),
    /// A value above the reader's range: every digit reads on its own.
    Digits(String),
}

/// What the layer decided to read in English for one span.
enum EnglishClaim {
    /// A standalone cardinal (#369): the route chooses the register.
    Number(EnglishNumber),
    /// A construct: only the narrated route reads it in English today.
    Construct(String),
}

impl EnglishNumber {
    /// The cardinal Latin words, which the narrated route transliterates.
    fn latin(&self) -> String {
        match self {
            Self::Cardinal(value) => {
                english::cardinal(*value).expect("a cardinal value below the limit")
            }
            Self::Digits(digits) => english::digits(digits),
        }
    }

    /// The Latin words for the native route: four-digit years read as pairs.
    fn native(&self) -> String {
        match self {
            Self::Cardinal(value) => english::native(*value),
            Self::Digits(digits) => english::digits(digits),
        }
    }
}

impl NumericSpanNormalizer {
    pub fn new() -> Self {
        Self
    }

    /// The English reading of a plain cardinal, if the span is one the English
    /// rule may claim.
    fn english_words(span: &parse::Span) -> Option<EnglishNumber> {
        let parse::Body::Number(number) = &span.body else {
            return None;
        };
        if span.trailing.is_some()
            || !number.is_plain_integer()
            || number.sign.is_some()
            || number.approx.is_some()
            || number.currency_prefix.is_some()
        {
            return None;
        }
        let digits = number.int.replace([' ', '\u{a0}'], "");
        if !digits.chars().all(|ch| ch.is_ascii_digit()) {
            return None;
        }
        // A zero-padded token of three or more digits is a code and keeps the
        // digit-by-digit Russian reading; the English rule does not claim it.
        if digits.len() >= 3 && digits.starts_with('0') {
            return None;
        }
        Some(match digits.parse::<u64>() {
            Ok(value) if value <= english::LIMIT => EnglishNumber::Cardinal(value),
            _ => EnglishNumber::Digits(digits),
        })
    }

    /// Whether the span sits in English context (see the Numbers requirement).
    #[allow(clippy::too_many_arguments)]
    fn english_context(
        tracked: &mut TrackedText,
        snapshot: &str,
        offsets: &[usize],
        span: (usize, usize),
        original_start: usize,
        sentences: &context::SentenceMap,
        letters: &context::LetterPrefix,
        identifiers: &[(usize, usize)],
        dictionary: &UserDictionary,
    ) -> bool {
        let left = Self::side(
            &mut *tracked,
            snapshot,
            offsets,
            span.0,
            true,
            letters,
            identifiers,
            dictionary,
        );
        // One foreign word is enough: the other side cannot make the number
        // English, and scanning it would only add work.
        if left == context::Side::Word {
            return false;
        }
        let right = Self::side(
            &mut *tracked,
            snapshot,
            offsets,
            span.1,
            false,
            letters,
            identifiers,
            dictionary,
        );
        if right == context::Side::Word {
            return false;
        }
        if left != context::Side::English && right != context::Side::English {
            return false;
        }
        !sentences.has_cyrillic_at(original_start)
    }

    /// The side a word takes, or `None` for a word the pipeline itself
    /// inserted: a markdown list marker becomes "первое:", and that marker must
    /// not veto the English context of the item text.
    #[allow(clippy::too_many_arguments)]
    fn side(
        tracked: &mut TrackedText,
        snapshot: &str,
        offsets: &[usize],
        pos: usize,
        before: bool,
        letters: &context::LetterPrefix,
        identifiers: &[(usize, usize)],
        dictionary: &UserDictionary,
    ) -> context::Side {
        let origin = pos;
        let mut cursor = pos;
        loop {
            let word = if before {
                context::word_before(snapshot, cursor)
            } else {
                context::word_after(snapshot, cursor)
            };
            let Some(word) = word else {
                return context::Side::Edge;
            };
            // Skipping inserted words must not walk the whole text either.
            let distance = if before {
                origin.saturating_sub(word.start)
            } else {
                word.start.saturating_sub(origin)
            };
            if distance > context::NEIGHBOUR_WINDOW {
                return context::Side::Edge;
            }
            let token = &snapshot[word.start..word.end];
            match Self::classify(
                &mut *tracked,
                offsets,
                token,
                (word.start, word.end),
                letters,
                identifiers,
                dictionary,
            ) {
                Some(side) => return side,
                None => cursor = if before { word.start } else { word.end },
            }
        }
    }

    /// The side of one word; `None` means the word is a pipeline insertion.
    fn classify(
        tracked: &mut TrackedText,
        offsets: &[usize],
        token: &str,
        range: (usize, usize),
        letters: &context::LetterPrefix,
        identifiers: &[(usize, usize)],
        dictionary: &UserDictionary,
    ) -> Option<context::Side> {
        if token.chars().any(context::is_cyrillic) {
            let (start, end) = tracked.original_range(offsets[range.0], offsets[range.1]);
            // A label the pipeline produced from digits or punctuation (a
            // markdown list marker) is not a neighbour; a Cyrillic reading of a
            // URL, path or inline code is not prose and still vetoes the number.
            return if letters.has_letters(start, end) {
                Some(context::Side::Word)
            } else {
                None
            };
        }
        let latin = token.chars().all(|ch| ch.is_ascii_alphabetic());
        if latin
            && !super::overlaps_ranges(identifiers, range.0, range.1)
            && dictionary.resolve(&token.to_lowercase(), false).is_none()
        {
            Some(context::Side::English)
        } else {
            Some(context::Side::Word)
        }
    }

    pub fn process(
        &self,
        tracked: &mut TrackedText,
        expand_units: bool,
        routing: &mut NumericRouting<'_>,
    ) {
        let snapshot = tracked.text().to_string();
        let spans = parse::scan(&snapshot, expand_units);
        if spans.is_empty() {
            return;
        }

        let route = routing.route;
        let native_lang = routing.native_lang;
        let dictionary = routing.dictionary;
        let recorded = &mut *routing.recorded;
        let claimed = &mut *routing.claimed;

        let needs_context = spans.iter().any(|span| {
            Self::english_words(span).is_some()
                || (route == NumberRoute::Narrate && constructs::read(span, expand_units).is_some())
        });
        let offsets = if needs_context {
            byte_to_char_offsets(&snapshot)
        } else {
            Vec::new()
        };
        let identifiers = if needs_context {
            super::find_identifier_ranges(&snapshot)
        } else {
            Vec::new()
        };
        let sentences = if needs_context {
            context::SentenceMap::build(&tracked.original)
        } else {
            context::SentenceMap::build("")
        };
        let letters = if needs_context {
            context::LetterPrefix::build(&tracked.original)
        } else {
            context::LetterPrefix::build("")
        };

        let mut replacements: Vec<(usize, usize, String)> = Vec::new();
        let mut native_ranges: Vec<(usize, usize)> = Vec::new();
        for span in &spans {
            let claim = match Self::english_words(span) {
                Some(number) => Some(EnglishClaim::Number(number)),
                None if route == NumberRoute::Narrate => {
                    constructs::read(span, expand_units).map(EnglishClaim::Construct)
                }
                None => None,
            };
            let english = match claim {
                Some(claim) => {
                    let original_start = tracked
                        .original_range(offsets[span.start], offsets[span.end])
                        .0;
                    let matched = Self::english_context(
                        &mut *tracked,
                        &snapshot,
                        &offsets,
                        (span.start, span.end),
                        original_start,
                        &sentences,
                        &letters,
                        &identifiers,
                        dictionary,
                    );
                    if matched { Some(claim) } else { None }
                }
                None => None,
            };
            match english {
                Some(EnglishClaim::Number(number)) => match route {
                    NumberRoute::Narrate => replacements.push((
                        span.start,
                        span.end,
                        english::transliterate(&number.latin()),
                    )),
                    NumberRoute::Native => {
                        let (orig_start, orig_end) =
                            tracked.original_range(offsets[span.start], offsets[span.end]);
                        if orig_end > orig_start {
                            claimed.push((orig_start, orig_end));
                        }
                        replacements.push((span.start, span.end, number.native()));
                        native_ranges.push((span.start, span.end));
                    }
                },
                Some(EnglishClaim::Construct(latin)) => {
                    replacements.push((span.start, span.end, english::transliterate(&latin)))
                }
                None => {
                    if let Some(reading) = span.read(&snapshot, expand_units) {
                        replacements.push((span.start, span.end, reading));
                    }
                }
            }
        }

        if !native_ranges.is_empty() {
            record_byte_ranges(
                tracked,
                &offsets,
                SpanKind::ProseLatin,
                native_lang,
                &native_ranges,
                recorded,
            );
        }
        if !replacements.is_empty() {
            tracked.replace_byte_ranges(replacements);
        }
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

    #[test]
    fn decades_cover_the_twentieth_and_twenty_first_centuries() {
        assert_eq!(read_all("в 1910-х годах", true), "в десятых годах");
        assert_eq!(
            read_all("в 2010-х годах", true),
            "в две тысячи десятых годах"
        );
        assert_eq!(read_all("2010-е годы", true), "две тысячи десятые годы");
    }

    #[test]
    fn currency_prefix_agrees_with_the_count() {
        assert_eq!(read_all("$2", true), "два доллара");
        assert_eq!(read_all("$21", true), "двадцать один доллар");
        assert_eq!(read_all("около $1", true), "около одного доллара");
        assert_eq!(read_all("$1,5", true), "полтора доллара");
    }

    #[test]
    fn year_range_outside_the_nineteen_hundreds() {
        assert_eq!(
            read_all("1812–1814 годы", true),
            "с тысяча восемьсот двенадцатого по тысяча восемьсот четырнадцатый год"
        );
    }

    #[test]
    fn fractions_agree_in_gender_and_number() {
        assert_eq!(read_all("0,1", true), "ноль целых одна десятая");
        assert_eq!(
            read_all("2,25 кг", true),
            "две целых двадцать пять сотых килограмма"
        );
        assert_eq!(
            read_all("около 21 кг", true),
            "около двадцати одного килограмма"
        );
        assert_eq!(read_all("более 1 кг", true), "более одного килограмма");
    }

    #[test]
    fn scale_one_keeps_the_case() {
        assert_eq!(
            read_all("около 1 млн рублей", true),
            "около миллиона рублей"
        );
    }

    #[test]
    fn dative_and_prepositional_contexts() {
        assert_eq!(read_all("к 8 марта", true), "к восьмому марта");
        assert_eq!(read_all("к 5-й главе", true), "к пятой главе");
        assert_eq!(read_all("к 5 кг", true), "к пяти килограммам");
        assert_eq!(read_all("об 1 кг", true), "об одном килограмме");
        assert_eq!(read_all("о 2 кг", true), "о двух килограммах");
        assert_eq!(read_all("при 10 %", true), "при десяти процентах");
        assert_eq!(read_all("к $5", true), "к пяти долларам");
        assert_eq!(read_all("к €5", true), "к пяти евро");
        assert_eq!(read_all("к 1 млн", true), "к миллиону");
        assert_eq!(read_all("к 2 тыс.", true), "к двум тысячам");
        assert_eq!(read_all("к 1 °C", true), "к одному градусу Цельсия");
    }

    #[test]
    fn slash_and_mixed_fractions_own_their_unit() {
        assert_eq!(read_all("1/2 кг", true), "одна вторая килограмма");
        assert_eq!(read_all("1½ кг", true), "полтора килограмма");
        assert_eq!(
            read_all("около 1/2 кг", true),
            "около одной второй килограмма"
        );
        assert_eq!(read_all("к 1,5 кг", true), "к полутора килограммам");
    }

    #[test]
    fn huge_numbers_read_digit_by_digit() {
        assert_eq!(
            read_all("1000000000000", true),
            "один ноль ноль ноль ноль ноль ноль ноль ноль ноль ноль ноль ноль"
        );
    }

    #[test]
    fn dotted_groups_keep_leading_zeros() {
        assert_eq!(read_all("v1.02", true), "один точка ноль два");
    }

    #[test]
    fn dotted_groups_above_the_cardinal_limit_read_digit_by_digit() {
        assert_eq!(
            read_all("1.1758881234567", true),
            "один точка один семь пять восемь восемь восемь один два три четыре пять шесть семь"
        );
        assert_eq!(
            read_all("1.1000000000000", true),
            "один точка один ноль ноль ноль ноль ноль ноль ноль ноль ноль ноль ноль ноль"
        );
    }

    #[test]
    fn hundred_thousandth_is_one_word() {
        assert_eq!(read_all("100000-й", true), "стотысячный");
    }

    #[test]
    fn roman_abbreviation_and_single_letter_centuries() {
        assert_eq!(read_all("XX в.", true), "двадцатый век");
        assert_eq!(read_all("в XX в.", true), "в двадцатом веке");
        assert_eq!(read_all("V век", true), "пятый век");
    }

    #[test]
    fn version_suffix_digits_are_read() {
        assert_eq!(
            read_all("1.2.0-rc1", true),
            "один точка два точка ноль эр си один"
        );
        assert_eq!(
            read_all("1.0.0-rc01", true),
            "один точка ноль точка ноль эр си ноль один"
        );
    }

    #[test]
    fn bare_degree_is_owned() {
        assert_eq!(read_all("Угол 45°", true), "Угол сорок пять градусов");
    }

    #[test]
    fn spaced_plus_is_not_a_suffix() {
        assert_eq!(read_all("2 + 2", true), "два + два");
    }

    #[test]
    fn currency_prefix_is_left_alone_for_native_units() {
        assert_eq!(read_all("$5", false), "$5");
    }

    #[test]
    fn ordinal_suffixes_that_were_missing() {
        assert_eq!(read_all("5-ие классы", true), "пятые классы");
        assert_eq!(read_all("5-ими", true), "пятыми");
        assert_eq!(read_all("5-ого", true), "пятого");
    }

    #[test]
    fn phone_groups() {
        assert_eq!(
            read_all("+7 921 1234567", true),
            "плюс семь девятьсот двадцать один один два три четыре пять шесть семь"
        );
        assert_eq!(read_all("1 + 2 3 4", true), "один + два три четыре");
    }

    #[test]
    fn short_time_units_are_recognized() {
        assert_eq!(read_all("5 мин", true), "пять минут");
        assert_eq!(read_all("10 сек", true), "десять секунд");
        assert_eq!(read_all("2 ч", true), "два часа");
    }

    #[test]
    fn currency_words_keep_the_sentence_period() {
        assert_eq!(read_all("5 USD.", true), "пять долларов.");
        assert_eq!(read_all("5 руб.", true), "пять рублей");
    }

    #[test]
    fn year_range_word_does_not_claim_a_non_year_range() {
        assert_eq!(read_all("5–10 годы", true), "5–10 годы");
    }

    #[test]
    fn long_fraction_tails_use_the_technical_register() {
        assert_eq!(
            read_all("0,0000001", true),
            "ноль точка ноль ноль ноль ноль ноль ноль один"
        );
        assert_eq!(
            read_all("0,0000001 кг", true),
            "ноль точка ноль ноль ноль ноль ноль ноль один килограммов"
        );
    }

    #[test]
    fn prefixes_need_a_word_start() {
        // The prefix stays for the symbol phase; the digits are still read,
        // but the numeric layer must not merge them into a currency reading.
        assert_eq!(read_all("US$5", true), "US$пять");
        assert_eq!(read_all("x~5", true), "x~пять");
    }
}
