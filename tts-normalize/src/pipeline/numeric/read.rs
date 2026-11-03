//! Spoken readings for the spans the scanner recognises.
//!
//! One place turns a shape into words. Classification lives here as well: the
//! reading is chosen from the shape plus the words around it (a year needs a
//! year word, a day needs a month, a month word needs a year), so the layer has
//! a single classification step rather than competing regex phases.

use super::lexicon::{self, Currency, MONTHS_GENITIVE, MONTHS_NOMINATIVE, Reference, Scale, Unit};
use super::morphology::{self, Case, Gender, ends_in_one};
use super::parse::{
    Approx, Body, Date, MixedFraction, Number, OrdinalSuffix, Sign, Span, Time, Trailing,
    TrailingKind, is_year, read_digits, read_word, skip_spaces,
};

/// Percent noun forms by count.
const PERCENT: (&str, &str, &str) = ("процент", "процента", "процентов");
/// Minute noun forms by count.
const MINUTE: (&str, &str, &str) = ("минута", "минуты", "минут");
/// Second noun forms by count.
const SECOND: (&str, &str, &str) = ("секунда", "секунды", "секунд");
/// Hour noun forms by count.
const HOUR: (&str, &str, &str) = ("час", "часа", "часов");

impl Span {
    /// Read the span, or `None` when the layer does not own it.
    pub(crate) fn read(&self, text: &str, units: bool) -> Option<String> {
        match &self.body {
            Body::Number(number) => read_number(self, number, text, units),
            Body::Date(date) => Some(read_date(*date, text, self.start)),
            Body::YearMonth { year, month } => Some(read_year_month(*year, *month)),
            Body::Range { left, right } => read_range(self, left, right, text, units),
            Body::Reference { kind, number } => read_reference(*kind, number),
            Body::Phone { groups, labeled } => Some(read_phone(groups, *labeled)),
            Body::City => Some("город".to_string()),
            Body::Roman { value, abbreviated } => {
                Some(read_roman(*value, *abbreviated, text, self.start, self.end))
            }
        }
    }
}

// ── Numbers ──────────────────────────────────────────────────────────────────

fn read_number(span: &Span, number: &Number, text: &str, units: bool) -> Option<String> {
    if !units && (owns_measurement(span.trailing) || number.currency_prefix.is_some()) {
        return None;
    }
    let forced = match number.approx {
        Some(Approx::Tilde) => Some(Case::Genitive),
        _ => None,
    };
    let mut reading = if let Some(suffix) = number.ordinal_suffix {
        read_suffix(number, suffix, text, span)
    } else if let Some(time) = &number.time {
        Some(read_time(time, text, span))
    } else if number.leading_dot {
        read_leading_dot(number, span.trailing)
    } else if !number.dot_groups.is_empty() {
        read_dotted(number, span.trailing, text, span)
    } else if let Some(frac) = &number.comma_frac {
        read_fraction(number, frac, span.trailing, text, span, forced)
    } else if !number.slash_groups.is_empty() {
        read_slash(number, span.trailing, text, span, forced)
    } else if let Some(half) = number.half {
        read_mixed(number, half, span.trailing, text, span, forced)
    } else {
        read_plain(number, span.trailing, text, span, forced)
    }?;

    if let Some(currency) = number.currency_prefix {
        let value = readable_value(number)?;
        let case = case_at(text, span.start, forced);
        let (nom, gen_sg, gen_pl, _) = currency.forms();
        // A whole count agrees with the currency noun ("два доллара",
        // "двадцать один доллар", "около одного доллара"); a fraction names
        // the quantity itself ("полтора доллара").
        let noun = if has_fraction(number) {
            fraction_noun((nom, gen_sg, gen_pl), case, false)
        } else {
            noun_after_count(value, (nom, gen_sg, gen_pl), case)
        };
        reading.push(' ');
        reading.push_str(&noun);
    }

    if let Some(approx) = number.approx {
        let prefix = match approx {
            Approx::Tilde => "около",
            Approx::Almost => "примерно",
            Approx::PlusMinus => "плюс-минус",
        };
        reading = format!("{prefix} {reading}");
    }

    if let Some(sign) = number.sign {
        let prefix = match sign {
            Sign::Minus => "минус",
            Sign::Plus => "плюс",
        };
        reading = format!("{prefix} {reading}");
    }
    Some(reading)
}

fn owns_measurement(trailing: Option<Trailing>) -> bool {
    trailing.is_some_and(|trailing| {
        matches!(
            trailing.kind,
            TrailingKind::Percent
                | TrailingKind::Unit(_)
                | TrailingKind::Currency(_)
                | TrailingKind::Scale(_)
        )
    })
}

/// Whether the reading of `number` is a fractional quantity rather than a
/// whole count.
fn has_fraction(number: &Number) -> bool {
    number.comma_frac.is_some() || number.half.is_some() || !number.slash_groups.is_empty()
}

fn read_plain(
    number: &Number,
    trailing: Option<Trailing>,
    text: &str,
    span: &Span,
    forced: Option<Case>,
) -> Option<String> {
    let case = case_at(text, span.start, forced);
    match trailing.map(|trailing| trailing.kind) {
        None => read_bare_number(number, text, span, forced),
        Some(kind) => {
            let value = readable_value(number)?;
            match kind {
                TrailingKind::Percent => Some(read_percent(value, case)),
                TrailingKind::Unit(unit) => Some(read_unit(value, unit, case)),
                TrailingKind::Currency(currency) => Some(read_currency(value, currency, case)),
                TrailingKind::Scale(scale) => Some(read_scale(
                    value,
                    scale,
                    trailing.expect("a scale trailing").currency,
                    case,
                )),
                TrailingKind::Plus => Some(format!(
                    "{} и больше",
                    morphology::count(
                        value,
                        lexicon::noun_gender(&next_word(text, span.end)),
                        case
                    )
                )),
                TrailingKind::YearAbbreviation => is_year(value).then(|| {
                    format!(
                        "{} года",
                        morphology::ordinal(value, Gender::Masculine, Case::Genitive)
                    )
                }),
                TrailingKind::YearRange => is_year(value).then(|| {
                    format!(
                        "{} год",
                        morphology::ordinal(value, Gender::Masculine, Case::Accusative)
                    )
                }),
            }
        }
    }
}

fn read_bare_number(
    number: &Number,
    text: &str,
    span: &Span,
    forced: Option<Case>,
) -> Option<String> {
    let digits = number.int.as_str();

    if digits.len() >= 3 && digits.starts_with('0') {
        return Some(digit_words(digits));
    }

    let Some(value) = readable_value(number) else {
        // The cardinal builder names values up to 999_999_999_999; a longer
        // magnitude is still read, ID-style, one digit at a time.
        return Some(digit_words(digits));
    };

    if is_day_range_left(value, text, span) {
        return Some(morphology::ordinal(value, Gender::Neuter, Case::Genitive));
    }

    let prev = prev_word(text, span.start);
    if let Some(gender) = lexicon::ordinal_head(&prev) {
        return Some(morphology::ordinal(value, gender, head_case(&prev)));
    }

    if let Some(reading) = read_year(number, text, span) {
        return Some(reading);
    }

    let case = case_at(text, span.start, forced);
    let next = next_word(text, span.end);
    Some(morphology::count(value, lexicon::noun_gender(&next), case))
}

/// A year reads as an ordinal only when something in the text makes it one:
/// a year word next to it, a month or season before it, or the left bound of
/// a year range.
fn read_year(number: &Number, text: &str, span: &Span) -> Option<String> {
    let value = readable_value(number)?;
    if number.int.len() != 4 || !is_year(value) {
        return None;
    }
    let next = next_token(text, span.end);
    let prev = prev_word(text, span.start);
    if let Some(case) = year_word_case(&next, &prev) {
        return Some(morphology::ordinal(value, Gender::Masculine, case));
    }
    let season_before = lexicon::month_from_nominative(&prev).is_some()
        || lexicon::YEAR_CONTEXT_WORDS.contains(&prev.as_str());
    if season_before {
        return Some(format!(
            "{} года",
            morphology::ordinal(value, Gender::Masculine, Case::Genitive)
        ));
    }
    if matches!(prev.as_str(), "с" | "со" | "от") && matches!(next.as_str(), "по" | "до") {
        return Some(morphology::ordinal(
            value,
            Gender::Masculine,
            Case::Genitive,
        ));
    }
    None
}

/// Case a year takes from the word that follows it.
fn year_word_case(token: &str, prev: &str) -> Option<Case> {
    match token {
        "год" => Some(Case::Nominative),
        "года" => Some(Case::Genitive),
        "году" => Some(if matches!(prev, "к" | "ко" | "по") {
            Case::Dative
        } else {
            Case::Prepositional
        }),
        "годом" => Some(Case::Instrumental),
        "годе" => Some(Case::Prepositional),
        "годы" => Some(Case::Nominative),
        "годов" => Some(Case::Genitive),
        "годах" => Some(Case::Prepositional),
        _ => None,
    }
}

fn read_percent(value: u64, case: Case) -> String {
    format!(
        "{} {}",
        morphology::count(value, Gender::Masculine, case),
        noun_after_count(value, PERCENT, case)
    )
}

fn read_unit(value: u64, unit: Unit, case: Case) -> String {
    let forms = unit.forms;
    if forms.fixed {
        return format!(
            "{} {}",
            morphology::count(value, forms.gender, case),
            forms.sg_nom
        );
    }
    let noun = noun_after_count(value, (forms.sg_nom, forms.sg_gen, forms.pl_gen), case);
    let mut out = format!("{} {noun}", morphology::count(value, forms.gender, case));
    if !forms.tail.is_empty() {
        out.push(' ');
        out.push_str(forms.tail);
    }
    out
}

fn read_currency(value: u64, currency: Currency, case: Case) -> String {
    let (nom, gen_sg, gen_pl, gender) = currency.forms();
    let noun = noun_after_count(value, (nom, gen_sg, gen_pl), case);
    format!("{} {noun}", morphology::count(value, gender, case))
}

fn read_scale(value: u64, scale: Scale, currency: Option<Currency>, case: Case) -> String {
    let (nom, gen_sg, gen_pl, gender) = scale.forms();
    let mut out = if value == 1 {
        // A lone scale word drops "один" but keeps the case: "около
        // миллиона", not "около миллион".
        noun_after_count(1, (nom, gen_sg, gen_pl), case)
    } else {
        format!(
            "{} {}",
            morphology::count(value, gender, case),
            noun_after_count(value, (nom, gen_sg, gen_pl), case)
        )
    };
    if let Some(currency) = currency {
        let (_, _, currency_pl, _) = currency.forms();
        out.push(' ');
        out.push_str(currency_pl);
    }
    out
}

fn read_fraction(
    number: &Number,
    frac: &str,
    trailing: Option<Trailing>,
    text: &str,
    span: &Span,
    forced: Option<Case>,
) -> Option<String> {
    let int = readable_value(number)?;
    if frac.len() > MAX_FRACTION_DIGITS {
        return Some(read_technical_fraction(int, frac, trailing));
    }
    let case = case_at(text, span.start, forced);
    let adjectival = !(frac == "5" && int >= 1);
    let tail = trailing_reading(trailing, text, span, case, adjectival);
    let mut out = morphology::fraction(int, frac, tail.gender, case);
    if let Some(suffix) = tail.suffix {
        out.push(' ');
        out.push_str(&suffix);
    }
    Some(out)
}

/// Fractional tails longer than this are not spoken as a natural fraction
/// ("N целых M миллионных"); the technical "точка" register reads them digit
/// by digit instead.
const MAX_FRACTION_DIGITS: usize = 6;

fn read_technical_fraction(int: u64, frac: &str, trailing: Option<Trailing>) -> String {
    let mut out = format!(
        "{} точка {}",
        morphology::count(int, Gender::Masculine, Case::Nominative),
        digit_words(frac)
    );
    match trailing.map(|trailing| trailing.kind) {
        Some(TrailingKind::Percent) => out.push_str(" процентов"),
        Some(TrailingKind::Unit(unit)) if unit.forms.fixed => {
            out.push(' ');
            out.push_str(unit.forms.sg_nom);
        }
        Some(TrailingKind::Unit(unit)) => {
            out.push(' ');
            out.push_str(unit.forms.pl_gen);
        }
        Some(TrailingKind::Currency(currency)) => {
            out.push(' ');
            out.push_str(currency.forms().2);
        }
        _ => {}
    }
    out
}

fn read_slash(
    number: &Number,
    trailing: Option<Trailing>,
    text: &str,
    span: &Span,
    forced: Option<Case>,
) -> Option<String> {
    let numerator = readable_value(number)?;
    let denominator: u64 = number.slash_groups.first()?.parse().ok()?;
    let case = case_at(text, span.start, forced);
    let mut out = morphology::simple_fraction(numerator, denominator, case)?;
    // A simple fraction is an adjective ("одна вторая"), so its unit stays
    // in the genitive singular regardless of the outer case.
    let tail = trailing_reading(trailing, text, span, case, true);
    if let Some(suffix) = tail.suffix {
        out.push(' ');
        out.push_str(&suffix);
    }
    Some(out)
}

fn read_mixed(
    number: &Number,
    half: MixedFraction,
    trailing: Option<Trailing>,
    text: &str,
    span: &Span,
    forced: Option<Case>,
) -> Option<String> {
    let int = readable_value(number)?;
    let case = case_at(text, span.start, forced);
    let adjectival = int == 0 || !matches!(half, MixedFraction::Half);
    let tail = trailing_reading(trailing, text, span, case, adjectival);
    // A bare mixed fraction reads "две с половиной"; with a unit the
    // integer part agrees with the unit ("два с половиной километра").
    let gender = if trailing.is_some() {
        tail.gender
    } else {
        Gender::Feminine
    };
    let mut out = match half {
        MixedFraction::Half => morphology::fraction(int, "5", gender, case),
        MixedFraction::Quarter if int == 0 => "одна четвёртая".to_string(),
        MixedFraction::Quarter => format!(
            "{} с четвертью",
            morphology::count(int, gender, Case::Nominative)
        ),
        MixedFraction::ThreeQuarters if int == 0 => "три четверти".to_string(),
        MixedFraction::ThreeQuarters => format!(
            "{} и три четверти",
            morphology::count(int, gender, Case::Nominative)
        ),
    };
    if let Some(suffix) = tail.suffix {
        out.push(' ');
        out.push_str(&suffix);
    }
    Some(out)
}

/// Spoken noun after a fractional reading, and the gender the fraction agrees
/// with.
struct QuantityTail {
    gender: Gender,
    suffix: Option<String>,
}

/// Noun a fractional quantity takes from its trailing construct.
///
/// A fraction ends in "-ая/-их", so the noun stays in the genitive singular
/// ("одна вторая килограмма"). A numeral-like fraction takes the genitive
/// plural in an oblique context ("около полутора долларов"); the
/// `adjectival` flag selects the first rule.
fn trailing_reading(
    trailing: Option<Trailing>,
    text: &str,
    span: &Span,
    case: Case,
    adjectival: bool,
) -> QuantityTail {
    let Some(trailing) = trailing else {
        let next = next_word(text, span.end);
        return QuantityTail {
            gender: lexicon::noun_gender(&next),
            suffix: None,
        };
    };
    match trailing.kind {
        TrailingKind::Unit(unit) => {
            if unit.forms.fixed {
                return QuantityTail {
                    gender: unit.forms.gender,
                    suffix: Some(unit.forms.sg_nom.to_string()),
                };
            }
            let noun = fraction_noun(
                (unit.forms.sg_nom, unit.forms.sg_gen, unit.forms.pl_gen),
                case,
                adjectival,
            );
            QuantityTail {
                gender: unit.forms.gender,
                suffix: Some(append_tail(&noun, unit.forms.tail)),
            }
        }
        TrailingKind::Scale(scale) => {
            let forms = scale.forms();
            let noun = fraction_noun((forms.0, forms.1, forms.2), case, adjectival);
            let suffix = match trailing.currency {
                Some(currency) => format!("{noun} {}", currency.forms().2),
                None => noun.to_string(),
            };
            QuantityTail {
                gender: forms.3,
                suffix: Some(suffix),
            }
        }
        TrailingKind::Currency(currency) => {
            let forms = currency.forms();
            let noun = fraction_noun((forms.0, forms.1, forms.2), case, adjectival);
            QuantityTail {
                gender: forms.3,
                suffix: Some(noun.to_string()),
            }
        }
        TrailingKind::Percent => {
            let noun = fraction_noun(PERCENT, case, adjectival);
            QuantityTail {
                gender: Gender::Masculine,
                suffix: Some(noun.to_string()),
            }
        }
        TrailingKind::YearAbbreviation | TrailingKind::YearRange | TrailingKind::Plus => {
            let next = next_word(text, span.end);
            QuantityTail {
                gender: lexicon::noun_gender(&next),
                suffix: None,
            }
        }
    }
}

/// Noun after a fractional quantity: an adjectival fraction keeps the
/// genitive singular ("одна вторая килограмма"); a numeral-like one takes the
/// genitive plural, and the dative/prepositional plural when the preposition
/// puts the quantity into an oblique case ("к полутора килограммам").
fn fraction_noun(
    forms: (&'static str, &'static str, &'static str),
    case: Case,
    adjectival: bool,
) -> String {
    if adjectival {
        return forms.1.to_string();
    }
    match case {
        Case::Genitive => forms.2.to_string(),
        Case::Dative | Case::Prepositional => oblique_plural(forms.0, case == Case::Dative),
        _ => forms.1.to_string(),
    }
}

fn read_leading_dot(number: &Number, trailing: Option<Trailing>) -> Option<String> {
    let frac = number.comma_frac.as_deref()?;
    let mut out = format!("ноль точка {}", digit_words(frac));
    match trailing.map(|trailing| trailing.kind) {
        Some(TrailingKind::Percent) => out.push_str(" процентов"),
        Some(TrailingKind::Unit(unit)) if unit.forms.fixed => {
            out.push(' ');
            out.push_str(unit.forms.sg_nom);
        }
        Some(TrailingKind::Unit(unit)) => {
            out.push(' ');
            out.push_str(unit.forms.pl_gen);
        }
        _ => {}
    }
    Some(out)
}

fn read_dotted(
    number: &Number,
    trailing: Option<Trailing>,
    text: &str,
    span: &Span,
) -> Option<String> {
    if matches!(
        trailing.map(|trailing| trailing.kind),
        Some(TrailingKind::Scale(_))
    ) {
        let frac = number.dot_groups.first()?;
        let currency = trailing.and_then(|trailing| trailing.currency);
        return read_fraction(number, frac, trailing, text, span, None).map(|reading| {
            if let Some(currency) = currency {
                format!("{reading} {}", currency.forms().2)
            } else {
                reading
            }
        });
    }

    let mut out = technical(number)?;
    if let Some(suffix) = number.version_suffix {
        out.push(' ');
        out.push_str(suffix);
        if let Some(digits) = &number.version_suffix_digits {
            out.push(' ');
            out.push_str(&version_digits(digits));
        }
    }
    match trailing.map(|trailing| trailing.kind) {
        Some(TrailingKind::Percent) => out.push_str(" процентов"),
        Some(TrailingKind::Unit(unit)) if unit.forms.fixed => {
            out.push(' ');
            out.push_str(unit.forms.sg_nom);
        }
        Some(TrailingKind::Unit(unit)) => {
            out.push(' ');
            out.push_str(unit.forms.pl_gen);
        }
        Some(TrailingKind::Currency(currency)) => {
            out.push(' ');
            out.push_str(currency.forms().2);
        }
        _ => {}
    }
    Some(out)
}

fn technical(number: &Number) -> Option<String> {
    let mut parts = vec![morphology::count(
        readable_value(number)?,
        Gender::Masculine,
        Case::Nominative,
    )];
    for group in &number.dot_groups {
        parts.push(group_reading(group));
    }
    Some(parts.join(" точка "))
}

/// A numeric group in the technical register: a count while the cardinal
/// builder can name it, digit by digit once it cannot (a group can exceed
/// [`MAX_CARDINAL`] even though its digits fit `u64`).
fn magnitude_reading(digits: &str) -> String {
    match digits.parse::<u64>() {
        Ok(value) if value <= MAX_CARDINAL => {
            morphology::count(value, Gender::Masculine, Case::Nominative)
        }
        _ if digits.is_empty() => "ноль".to_string(),
        _ => digit_words(digits),
    }
}

/// One dotted group: a leading zero carries information ("v1.02"), so the
/// group is read digit by digit instead of as a number.
fn group_reading(group: &str) -> String {
    if group.len() > 1 && group.starts_with('0') {
        return digit_words(group);
    }
    magnitude_reading(group)
}

/// Digits of a pre-release suffix ("1.2.0-rc1"): a count until a leading zero
/// or a long tail makes digit-by-digit the safer reading.
fn version_digits(digits: &str) -> String {
    if (digits.len() > 1 && digits.starts_with('0')) || digits.len() > 9 {
        digit_words(digits)
    } else {
        magnitude_reading(digits)
    }
}

// ── Suffixes ─────────────────────────────────────────────────────────────────

fn read_suffix(number: &Number, suffix: OrdinalSuffix, text: &str, span: &Span) -> Option<String> {
    let value = readable_value(number)?;
    if let Some(case) = decade_case(value, suffix, text, span.end) {
        if let Some(word) = morphology::decade(value, case) {
            return Some(word);
        }
    }

    let reading = match suffix {
        OrdinalSuffix::AmbiguousX => {
            let next = next_word(text, span.end);
            if lexicon::plural_ordinal_head(&next) {
                morphology::ordinal_plural(value, Case::Prepositional)
            } else {
                morphology::count(value, lexicon::noun_gender(&next), Case::Genitive)
            }
        }
        OrdinalSuffix::Instrumental => morphology::ordinal_plural(value, Case::Instrumental),
        OrdinalSuffix::Nominative => morphology::ordinal_plural(value, Case::Nominative),
        OrdinalSuffix::Genitive => morphology::ordinal_plural(value, Case::Genitive),
        OrdinalSuffix::Dative => morphology::ordinal_plural(value, Case::Dative),
        OrdinalSuffix::AmbiguousM => {
            let next = next_token(text, span.end);
            let prev = prev_word(text, span.start);
            let case = year_word_case(&next, &prev).unwrap_or_else(|| {
                if matches!(prev.as_str(), "в" | "во" | "на" | "по") {
                    Case::Prepositional
                } else {
                    lexicon::preposition_case(&prev).unwrap_or(Case::Instrumental)
                }
            });
            morphology::ordinal(value, Gender::Masculine, case)
        }
        OrdinalSuffix::MascPrepositional => {
            morphology::ordinal(value, Gender::Masculine, Case::Prepositional)
        }
        OrdinalSuffix::MascGenitive => {
            morphology::ordinal(value, Gender::Masculine, Case::Genitive)
        }
        OrdinalSuffix::MascDative => morphology::ordinal(value, Gender::Masculine, Case::Dative),
        // A bare "-й" is ambiguous: a masculine nominative ("5-й день")
        // or a feminine oblique form before a feminine head ("из 5-й главы").
        OrdinalSuffix::AmbiguousFeminine => {
            let next = next_word(text, span.end);
            let feminine = lexicon::ordinal_head(&next) == Some(Gender::Feminine)
                || lexicon::noun_gender(&next) == Gender::Feminine;
            if feminine {
                let prev = prev_word(text, span.start);
                let case = if matches!(prev.as_str(), "в" | "во" | "на") {
                    Case::Prepositional
                } else {
                    lexicon::context_case(&prev).unwrap_or(Case::Nominative)
                };
                morphology::ordinal(value, Gender::Feminine, case)
            } else {
                morphology::ordinal(value, Gender::Masculine, Case::Nominative)
            }
        }
        OrdinalSuffix::MascNominative => {
            morphology::ordinal(value, Gender::Masculine, Case::Nominative)
        }
        // "-ой" is never the masculine nominative of a numeral: it is the
        // feminine oblique form ("к 5-ой главе", "в 5-ой").
        OrdinalSuffix::FeminineOblique => {
            morphology::ordinal(value, Gender::Feminine, Case::Prepositional)
        }
        OrdinalSuffix::FeminineNominative => {
            morphology::ordinal(value, Gender::Feminine, Case::Nominative)
        }
        OrdinalSuffix::FeminineAccusative => {
            morphology::ordinal(value, Gender::Feminine, Case::Accusative)
        }
        OrdinalSuffix::NeuterNominative => {
            morphology::ordinal(value, Gender::Neuter, Case::Nominative)
        }
    };
    Some(reading)
}

/// A decade suffix (`1990-х`, `2000-е`) reads as a decade word unless the head
/// noun makes it an ordinal ("20-е место").
fn decade_case(value: u64, suffix: OrdinalSuffix, text: &str, end: usize) -> Option<Case> {
    let case = match suffix {
        OrdinalSuffix::AmbiguousX | OrdinalSuffix::Genitive => Case::Prepositional,
        OrdinalSuffix::NeuterNominative | OrdinalSuffix::Nominative => Case::Nominative,
        _ => return None,
    };
    if morphology::is_century_decade(value) {
        // The decade stem is the single source of truth: a value it cannot
        // name must fall through to a cardinal reading, never to a decade one.
        return morphology::is_decade(value).then_some(case);
    }
    if (20..=90).contains(&value) && morphology::is_decade(value) {
        let next = next_word(text, end);
        let year_head = next.is_empty()
            || matches!(
                next.as_str(),
                "год" | "года" | "году" | "годах" | "годы" | "годов" | "гг"
            );
        if year_head {
            return Some(case);
        }
    }
    None
}

// ── Dates, times, centuries ──────────────────────────────────────────────────

fn read_date(date: Date, text: &str, start: usize) -> String {
    let prev = prev_word(text, start);
    let case = if prev == "по" {
        Case::Accusative
    } else {
        lexicon::context_case(&prev).unwrap_or(Case::Nominative)
    };
    let day = morphology::ordinal(u64::from(date.day), Gender::Neuter, case);
    let month = MONTHS_GENITIVE[(date.month - 1) as usize];
    match date.year {
        Some(year) => format!(
            "{day} {month} {} года",
            morphology::ordinal(u64::from(year), Gender::Masculine, Case::Genitive)
        ),
        None => format!("{day} {month}"),
    }
}

fn read_year_month(year: u32, month: u32) -> String {
    format!(
        "{} {} года",
        MONTHS_NOMINATIVE[(month - 1) as usize],
        morphology::ordinal(u64::from(year), Gender::Masculine, Case::Genitive)
    )
}

fn read_roman(value: u32, abbreviated: bool, text: &str, start: usize, end: usize) -> String {
    let case = if abbreviated {
        // "XX в." — the case comes from the preposition before the numeral
        // ("в двадцатом веке"), not from the abbreviation itself.
        lexicon::calendar_case(&prev_word(text, start)).unwrap_or(Case::Nominative)
    } else {
        let word_pos = skip_spaces(text, end);
        let (_, word) = read_word(text, word_pos);
        match word.to_lowercase().as_str() {
            "веке" => Case::Prepositional,
            "века" | "веков" => Case::Genitive,
            _ => Case::Nominative,
        }
    };
    let mut reading = morphology::ordinal(u64::from(value), Gender::Masculine, case);
    if abbreviated {
        reading.push(' ');
        reading.push_str(match case {
            Case::Genitive => "века",
            Case::Dative => "веку",
            Case::Instrumental => "веком",
            Case::Prepositional => "веке",
            Case::Nominative | Case::Accusative => "век",
        });
    }
    reading
}

/// A day that opens a "с … по …" span whose right bound is a day and a month
/// reads as an ordinal genitive ("с первого по пятое мая").
fn is_day_range_left(value: u64, text: &str, span: &Span) -> bool {
    if !(1..=31).contains(&value) {
        return false;
    }
    let prev = prev_word(text, span.start);
    if !matches!(prev.as_str(), "с" | "со" | "от") {
        return false;
    }
    let next = next_word(text, span.end);
    if !matches!(next.as_str(), "по" | "до") {
        return false;
    }
    let after_next = word_end(text, span.end);
    let digits_start = skip_spaces(text, after_next);
    let (digits_end, digits) = read_digits(text, digits_start);
    if digits.is_empty() {
        return false;
    }
    let month_start = skip_spaces(text, digits_end);
    let (_, word) = read_word(text, month_start);
    lexicon::month_from_genitive(&word.to_lowercase()).is_some()
}

// ── Ranges ───────────────────────────────────────────────────────────────────

fn read_range(
    span: &Span,
    left: &Number,
    right: &Number,
    text: &str,
    units: bool,
) -> Option<String> {
    let (Some(l), Some(r)) = (readable_value(left), readable_value(right)) else {
        return None;
    };
    let plain_bounds = left.is_plain_integer() && right.is_plain_integer();
    let year_range = matches!(
        span.trailing.map(|trailing| trailing.kind),
        Some(TrailingKind::YearRange)
    );
    // "годы"/"гг." belongs to year bounds only; claiming a non-year range
    // would drop the noun.
    if year_range && !(plain_bounds && is_year(l) && is_year(r)) {
        return None;
    }

    if plain_bounds && is_year(l) && is_year(r) {
        if year_range {
            return Some(format!(
                "с {} по {} год",
                morphology::ordinal(l, Gender::Masculine, Case::Genitive),
                morphology::ordinal(r, Gender::Masculine, Case::Accusative)
            ));
        }
        return Some(format!(
            "от {} до {}",
            morphology::ordinal(l, Gender::Masculine, Case::Genitive),
            morphology::ordinal(r, Gender::Masculine, Case::Genitive)
        ));
    }

    if plain_bounds && span.trailing.is_none() && r == l + 1 {
        let gender = lexicon::noun_gender(&next_word(text, span.end));
        return Some(format!(
            "{}-{}",
            morphology::count(l, gender, Case::Nominative),
            morphology::count(r, gender, Case::Nominative)
        ));
    }

    if !units && owns_measurement(span.trailing) {
        return None;
    }
    let mut out = format!("от {} до {}", bound_genitive(left)?, bound_genitive(right)?);
    if let Some(trailing) = span.trailing {
        match trailing.kind {
            TrailingKind::Percent => out.push_str(" процентов"),
            TrailingKind::Unit(unit) => {
                out.push(' ');
                if unit.forms.fixed {
                    out.push_str(unit.forms.sg_nom);
                } else {
                    out.push_str(unit.forms.pl_gen);
                    if !unit.forms.tail.is_empty() {
                        out.push(' ');
                        out.push_str(unit.forms.tail);
                    }
                }
            }
            TrailingKind::Currency(currency) => {
                out.push(' ');
                out.push_str(currency.forms().2);
            }
            _ => {}
        }
    }
    Some(out)
}

fn bound_genitive(number: &Number) -> Option<String> {
    let value = readable_value(number)?;
    if let Some(frac) = &number.comma_frac {
        return Some(morphology::fraction(
            value,
            frac,
            Gender::Masculine,
            Case::Genitive,
        ));
    }
    if number.dot_groups.len() == 1 {
        return Some(morphology::fraction(
            value,
            &number.dot_groups[0],
            Gender::Masculine,
            Case::Genitive,
        ));
    }
    Some(morphology::count(value, Gender::Masculine, Case::Genitive))
}

// ── References and phones ────────────────────────────────────────────────────

fn read_reference(kind: Reference, number: &Number) -> Option<String> {
    let reading = if kind.ordinal() {
        morphology::ordinal(readable_value(number)?, Gender::Feminine, Case::Nominative)
    } else if kind.digit_by_digit() {
        digit_words(&number.int)
    } else {
        morphology::count(readable_value(number)?, Gender::Masculine, Case::Nominative)
    };
    Some(format!("{} {reading}", kind.word()))
}

fn read_phone(groups: &[String], labeled: bool) -> String {
    let mut parts: Vec<String> = Vec::new();
    if labeled {
        parts.push("телефон".to_string());
    }
    for group in groups {
        if group == "+" {
            parts.push("плюс".to_string());
            continue;
        }
        if (group.len() >= 3 && group.starts_with('0')) || group.len() > 4 {
            parts.push(digit_words(group));
            continue;
        }
        parts.push(morphology::count(
            group.parse().unwrap_or(0),
            Gender::Masculine,
            Case::Nominative,
        ));
    }
    parts.join(" ")
}

// ── Context helpers ──────────────────────────────────────────────────────────

/// The numeric value of the integer part, `None` when it does not fit `u64`.
fn value_of(number: &Number) -> Option<u64> {
    number.int.parse().ok()
}

/// The largest value the cardinal builder names; larger magnitudes fall back
/// to a digit-by-digit reading or stay unclaimed.
const MAX_CARDINAL: u64 = 999_999_999_999;

fn readable_value(number: &Number) -> Option<u64> {
    value_of(number).filter(|value| *value <= MAX_CARDINAL)
}

fn digit_words(digits: &str) -> String {
    digits
        .chars()
        .filter_map(|ch| {
            let word = morphology::digit_word(ch);
            (!word.is_empty()).then_some(word)
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Noun form for a count: 1 → singular nominative, 2–4 → singular genitive,
/// everything else → plural genitive.
fn decline_numbered(value: u64, forms: (&'static str, &'static str, &'static str)) -> &'static str {
    if (11..=14).contains(&(value % 100)) {
        return forms.2;
    }
    match value % 10 {
        1 => forms.0,
        2..=4 => forms.1,
        _ => forms.2,
    }
}

/// Noun form after a count in `case`.
///
/// A count ending in one takes the singular form, every other count the
/// plural. The genitive is the exception the plain 1/2–4/other rule cannot
/// express: it takes the singular genitive ("около одного килограмма") after
/// a count ending in one and the plural genitive otherwise ("около двух
/// килограммов"). The dative and prepositional forms are derived from the
/// nominative singular; a noun whose three forms coincide ("евро") does not
/// decline and keeps its only form.
fn noun_after_count(
    value: u64,
    forms: (&'static str, &'static str, &'static str),
    case: Case,
) -> String {
    let (nom, gen_sg, gen_pl) = forms;
    if nom == gen_sg && gen_sg == gen_pl {
        return nom.to_string();
    }
    match case {
        Case::Genitive => {
            let form = if ends_in_one(value) { gen_sg } else { gen_pl };
            form.to_string()
        }
        Case::Dative | Case::Prepositional => {
            let dative = case == Case::Dative;
            if ends_in_one(value) {
                oblique_singular(nom, dative)
            } else {
                oblique_plural(nom, dative)
            }
        }
        _ => decline_numbered(value, forms).to_string(),
    }
}

/// Dative or prepositional singular of a regular noun, derived from its
/// nominative form ("килограмм" → "килограмму"/"килограмме").
fn oblique_singular(nom: &str, dative: bool) -> String {
    if let Some(stem) = nom.strip_suffix('ь') {
        format!("{stem}{}", if dative { "ю" } else { "е" })
    } else if let Some(stem) = nom.strip_suffix('а').or_else(|| nom.strip_suffix('я')) {
        format!("{stem}е")
    } else if let Some(stem) = nom.strip_suffix('о') {
        format!("{stem}{}", if dative { "у" } else { "е" })
    } else {
        format!("{nom}{}", if dative { "у" } else { "е" })
    }
}

/// Dative or prepositional plural of a regular noun ("килограмм" →
/// "килограммам"/"килограммах").
fn oblique_plural(nom: &str, dative: bool) -> String {
    if let Some(stem) = nom.strip_suffix('ь').or_else(|| nom.strip_suffix('я')) {
        format!("{stem}{}", if dative { "ям" } else { "ях" })
    } else if let Some(stem) = nom.strip_suffix('а').or_else(|| nom.strip_suffix('о')) {
        format!("{stem}{}", if dative { "ам" } else { "ах" })
    } else if let Some(stem) = nom.strip_suffix('е') {
        format!("{stem}{}", if dative { "ям" } else { "ях" })
    } else {
        format!("{nom}{}", if dative { "ам" } else { "ах" })
    }
}

/// Case of the number's context: the shared preposition table, a genitive
/// trigger before it, or a forced case from an approximation sign.
fn case_at(text: &str, pos: usize, forced: Option<Case>) -> Case {
    forced
        .unwrap_or_else(|| lexicon::context_case(&prev_word(text, pos)).unwrap_or(Case::Nominative))
}

/// Case an ordinal takes from the head noun before it.
fn head_case(head: &str) -> Case {
    match head {
        "глава" | "том" | "раздел" | "квартал" | "этаж" | "часть" => {
            Case::Nominative
        }
        "главу" | "главам" => Case::Accusative,
        "главы" | "тома" | "раздела" | "части" => Case::Genitive,
        "главе" | "томе" | "разделе" | "главах" | "томах" | "частях" => {
            Case::Prepositional
        }
        "томом" | "разделом" | "частью" => Case::Instrumental,
        _ => Case::Nominative,
    }
}

fn append_tail(noun: &str, tail: &str) -> String {
    if tail.is_empty() {
        noun.to_string()
    } else {
        format!("{noun} {tail}")
    }
}

fn prev_word(text: &str, pos: usize) -> String {
    let before = &text[..pos];
    let trimmed = before.trim_end_matches([' ', '\u{a0}', '\t', '\n', '\r']);
    let word: String = trimmed
        .chars()
        .rev()
        .take_while(|c| c.is_alphabetic())
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    word.to_lowercase()
}

fn next_word(text: &str, pos: usize) -> String {
    let after = text[pos..].trim_start_matches([' ', '\u{a0}', '\t', '\n', '\r']);
    after
        .chars()
        .take_while(|c| c.is_alphabetic())
        .collect::<String>()
        .to_lowercase()
}

fn next_token(text: &str, pos: usize) -> String {
    let after = text[pos..].trim_start_matches([' ', '\u{a0}', '\t', '\n', '\r']);
    after
        .chars()
        .take_while(|c| !c.is_whitespace())
        .collect::<String>()
        .to_lowercase()
}

/// End of the word that starts after `pos` (after leading spaces).
fn word_end(text: &str, pos: usize) -> usize {
    let start = skip_spaces(text, pos);
    read_word(text, start).0
}

// Keep the hour/minute noun declension next to the time reading.
fn hour_noun(value: u64, case: Case) -> &'static str {
    noun_for_time(value, HOUR, case)
}

fn minute_noun(value: u64, case: Case) -> &'static str {
    noun_for_time(value, MINUTE, case)
}

fn second_noun(value: u64, case: Case) -> &'static str {
    noun_for_time(value, SECOND, case)
}

fn noun_for_time(
    value: u64,
    forms: (&'static str, &'static str, &'static str),
    case: Case,
) -> &'static str {
    if case == Case::Genitive {
        if ends_in_one(value) { forms.1 } else { forms.2 }
    } else {
        decline_numbered(value, forms)
    }
}

fn read_time(time: &Time, text: &str, span: &Span) -> String {
    let hours: u64 = time.hours.parse().unwrap_or(0);
    let minutes: u64 = time.minutes.parse().unwrap_or(0);
    let seconds: u64 = time.seconds.as_deref().unwrap_or("0").parse().unwrap_or(0);
    let prev = prev_word(text, span.start);
    let next = next_token(text, span.end);
    let left_range =
        matches!(prev.as_str(), "с" | "со" | "от") && matches!(next.as_str(), "до" | "по");
    let right_range = matches!(prev.as_str(), "до");
    let case = if left_range || right_range {
        Case::Genitive
    } else {
        Case::Nominative
    };
    // Inside "с 10:00 до 18:00" the left bound drops the hour noun: the
    // pattern is "с десяти до восемнадцати часов".
    if left_range && minutes == 0 && seconds == 0 {
        return morphology::count(hours, Gender::Masculine, Case::Genitive);
    }
    let mut out = format!(
        "{} {}",
        morphology::count(hours, Gender::Masculine, case),
        hour_noun(hours, case)
    );
    if minutes > 0 {
        out.push_str(&format!(
            " {} {}",
            morphology::count(minutes, Gender::Feminine, case),
            minute_noun(minutes, case)
        ));
    }
    if seconds > 0 {
        out.push_str(&format!(
            " {} {}",
            morphology::count(seconds, Gender::Feminine, case),
            second_noun(seconds, case)
        ));
    }
    out
}
