//! Spoken readings for the spans the scanner recognises.
//!
//! One place turns a shape into words. Classification lives here as well: the
//! reading is chosen from the shape plus the words around it (a year needs a
//! year word, a day needs a month, a month word needs a year), so the layer has
//! a single classification step rather than competing regex phases.

use super::lexicon::{self, Currency, MONTHS_GENITIVE, MONTHS_NOMINATIVE, Reference, Scale, Unit};
use super::morphology::{self, Case, Gender};
use super::parse::{
    Approx, Body, Date, MixedFraction, Number, Sign, Span, Time, Trailing, TrailingKind,
    read_digits, read_word, skip_spaces,
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
            Body::Reference { kind, number } => Some(read_reference(*kind, number)),
            Body::Phone { groups, labeled } => Some(read_phone(groups, *labeled)),
            Body::City => Some("город".to_string()),
            Body::Roman(value) => Some(read_roman(*value, text, self.end)),
        }
    }
}

// ── Numbers ──────────────────────────────────────────────────────────────────

fn read_number(span: &Span, number: &Number, text: &str, units: bool) -> Option<String> {
    if !units && owns_measurement(span.trailing) {
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
        read_slash(number, span.trailing)
    } else if let Some(half) = number.half {
        read_mixed(number, half)
    } else {
        read_plain(number, span.trailing, text, span, forced)
    }?;

    if let Some(currency) = number.currency_prefix {
        let value = value_of(number);
        let (_, _, gen_pl, _) = currency.forms();
        reading.push(' ');
        reading.push_str(gen_pl);
        if value == 1 {
            // The prefix form is a count: "один доллар", not "один долларов".
            reading.truncate(reading.len() - gen_pl.len());
            reading.push_str(currency.forms().0);
        }
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

fn read_plain(
    number: &Number,
    trailing: Option<Trailing>,
    text: &str,
    span: &Span,
    forced: Option<Case>,
) -> Option<String> {
    let value = value_of(number);
    let case = case_at(text, span.start, forced);
    match trailing.map(|trailing| trailing.kind) {
        Some(TrailingKind::Percent) => Some(read_percent(value, case)),
        Some(TrailingKind::Unit(unit)) => Some(read_unit(value, unit, case)),
        Some(TrailingKind::Currency(currency)) => Some(read_currency(value, currency, case)),
        Some(TrailingKind::Scale(scale)) => Some(read_scale(
            value,
            scale,
            trailing.expect("a scale trailing").currency,
            case,
        )),
        Some(TrailingKind::Plus) => Some(format!(
            "{} и больше",
            morphology::count(
                value,
                lexicon::noun_gender(&next_word(text, span.end)),
                case
            )
        )),
        Some(TrailingKind::YearAbbreviation) => is_year(value).then(|| {
            format!(
                "{} года",
                morphology::ordinal(value, Gender::Masculine, Case::Genitive)
            )
        }),
        Some(TrailingKind::YearRange) => is_year(value).then(|| {
            format!(
                "{} год",
                morphology::ordinal(value, Gender::Masculine, Case::Accusative)
            )
        }),
        None => read_bare_number(number, text, span, forced),
    }
}

fn read_bare_number(
    number: &Number,
    text: &str,
    span: &Span,
    forced: Option<Case>,
) -> Option<String> {
    let value = value_of(number);
    let digits = number.int.as_str();

    if digits.len() >= 3 && digits.starts_with('0') {
        return Some(digit_words(digits));
    }

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
    let value = value_of(number);
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
    let noun = if case == Case::Genitive {
        if ends_in_one(value) {
            PERCENT.1
        } else {
            PERCENT.2
        }
    } else {
        decline_numbered(value, PERCENT)
    };
    format!(
        "{} {noun}",
        morphology::count(value, Gender::Masculine, case)
    )
}

fn read_unit(value: u64, unit: Unit, case: Case) -> String {
    let forms = unit.forms;
    if forms.fixed {
        return format!(
            "{} {}",
            morphology::count(value, forms.gender, Case::Nominative),
            forms.sg_nom
        );
    }
    let noun = if case == Case::Genitive {
        forms.pl_gen
    } else {
        decline_numbered(value, (forms.sg_nom, forms.sg_gen, forms.pl_gen))
    };
    let mut out = format!("{} {noun}", morphology::count(value, forms.gender, case));
    if !forms.tail.is_empty() {
        out.push(' ');
        out.push_str(forms.tail);
    }
    out
}

fn read_currency(value: u64, currency: Currency, case: Case) -> String {
    let (nom, gen_sg, gen_pl, gender) = currency.forms();
    let noun = if case == Case::Genitive {
        gen_pl
    } else {
        decline_numbered(value, (nom, gen_sg, gen_pl))
    };
    format!("{} {noun}", morphology::count(value, gender, case))
}

fn read_scale(value: u64, scale: Scale, currency: Option<Currency>, case: Case) -> String {
    let (nom, gen_sg, gen_pl, gender) = scale.forms();
    let mut out = if value == 1 {
        nom.to_string()
    } else {
        let noun = if case == Case::Genitive {
            gen_pl
        } else {
            decline_numbered(value, (nom, gen_sg, gen_pl))
        };
        format!("{} {noun}", morphology::count(value, gender, case))
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
    let int = value_of(number);
    let case = case_at(text, span.start, forced);
    // A scale word can carry a currency after it ("1,5 млн $").
    let extra_currency = match trailing {
        Some(Trailing {
            currency: Some(currency),
            ..
        }) => Some(currency),
        _ => None,
    };
    let (gender, noun) = match trailing.map(|trailing| trailing.kind) {
        Some(TrailingKind::Unit(unit)) => {
            if unit.forms.fixed {
                return Some(format!(
                    "{} {}",
                    morphology::fraction(int, frac, unit.forms.gender, case),
                    unit.forms.sg_nom
                ));
            }
            let noun = if case == Case::Genitive {
                unit.forms.pl_gen
            } else {
                unit.forms.sg_gen
            };
            (unit.forms.gender, Some(append_tail(noun, unit.forms.tail)))
        }
        Some(TrailingKind::Scale(scale)) => {
            let forms = scale.forms();
            let noun = if case == Case::Genitive {
                forms.2
            } else {
                forms.1
            };
            (forms.3, Some(noun.to_string()))
        }
        Some(TrailingKind::Currency(currency)) => {
            let forms = currency.forms();
            let noun = if case == Case::Genitive {
                forms.2
            } else {
                forms.1
            };
            (forms.3, Some(noun.to_string()))
        }
        Some(TrailingKind::Percent) => {
            let noun = if case == Case::Genitive {
                PERCENT.2
            } else {
                PERCENT.1
            };
            (Gender::Masculine, Some(noun.to_string()))
        }
        _ => {
            let next = next_word(text, span.end);
            (lexicon::noun_gender(&next), None)
        }
    };
    let mut out = morphology::fraction(int, frac, gender, case);
    if let Some(noun) = noun {
        out.push(' ');
        out.push_str(&noun);
    }
    if let Some(currency) = extra_currency {
        out.push(' ');
        out.push_str(currency.forms().2);
    }
    Some(out)
}

fn read_slash(number: &Number, trailing: Option<Trailing>) -> Option<String> {
    let numerator = value_of(number);
    let denominator: u64 = number.slash_groups.first()?.parse().ok()?;
    let mut out = morphology::simple_fraction(numerator, denominator)?;
    if let Some(Trailing {
        kind: TrailingKind::Percent,
        ..
    }) = trailing
    {
        out.push_str(" процента");
    }
    Some(out)
}

fn read_mixed(number: &Number, half: MixedFraction) -> Option<String> {
    let int = value_of(number);
    let reading = match half {
        MixedFraction::Half => morphology::fraction(int, "5", Gender::Feminine, Case::Nominative),
        MixedFraction::Quarter if int == 0 => "одна четвёртая".to_string(),
        MixedFraction::Quarter => format!(
            "{} с четвертью",
            morphology::count(int, Gender::Feminine, Case::Nominative)
        ),
        MixedFraction::ThreeQuarters if int == 0 => "три четверти".to_string(),
        MixedFraction::ThreeQuarters => format!(
            "{} и три четверти",
            morphology::count(int, Gender::Feminine, Case::Nominative)
        ),
    };
    Some(reading)
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

    let mut out = technical(number);
    if let Some(suffix) = number.version_suffix {
        out.push(' ');
        out.push_str(suffix);
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

fn technical(number: &Number) -> String {
    let mut parts = vec![morphology::count(
        value_of(number),
        Gender::Masculine,
        Case::Nominative,
    )];
    for group in &number.dot_groups {
        parts.push(morphology::count(
            group.parse().unwrap_or(0),
            Gender::Masculine,
            Case::Nominative,
        ));
    }
    parts.join(" точка ")
}

// ── Suffixes ─────────────────────────────────────────────────────────────────

fn read_suffix(number: &Number, suffix: &str, text: &str, span: &Span) -> Option<String> {
    let value = value_of(number);
    if let Some(case) = decade_case(value, suffix, text, span.end) {
        if let Some(word) = morphology::decade(value, case) {
            return Some(word);
        }
    }

    let reading = match suffix {
        "х" => {
            let next = next_word(text, span.end);
            if lexicon::plural_ordinal_head(&next) {
                morphology::ordinal_plural(value, Case::Prepositional)
            } else {
                morphology::count(value, lexicon::noun_gender(&next), Case::Genitive)
            }
        }
        "ми" | "ыми" | "ими" => morphology::ordinal_plural(value, Case::Instrumental),
        "ые" | "ие" => morphology::ordinal_plural(value, Case::Nominative),
        "ых" | "их" => morphology::ordinal_plural(value, Case::Genitive),
        "ым" | "им" => morphology::ordinal_plural(value, Case::Dative),
        "м" => {
            let next = next_token(text, span.end);
            let prev = prev_word(text, span.start);
            let case = year_word_case(&next, &prev).unwrap_or(
                if matches!(prev.as_str(), "в" | "во" | "на" | "о" | "об" | "при" | "по")
                {
                    Case::Prepositional
                } else {
                    Case::Instrumental
                },
            );
            morphology::ordinal(value, Gender::Masculine, case)
        }
        "ом" | "ем" => morphology::ordinal(value, Gender::Masculine, Case::Prepositional),
        "го" | "его" | "ого" => {
            morphology::ordinal(value, Gender::Masculine, Case::Genitive)
        }
        "му" | "ому" => morphology::ordinal(value, Gender::Masculine, Case::Dative),
        // A bare "-й" is ambiguous: a masculine nominative ("5-й день")
        // or a feminine oblique form before a feminine head ("из 5-й главы").
        "й" => {
            let next = next_word(text, span.end);
            let feminine = lexicon::ordinal_head(&next) == Some(Gender::Feminine)
                || lexicon::noun_gender(&next) == Gender::Feminine;
            if feminine {
                let prev = prev_word(text, span.start);
                let case = match prev.as_str() {
                    "в" | "во" | "на" | "о" | "об" | "при" => Case::Prepositional,
                    _ if lexicon::genitive_trigger(&prev) => Case::Genitive,
                    _ => Case::Nominative,
                };
                morphology::ordinal(value, Gender::Feminine, case)
            } else {
                morphology::ordinal(value, Gender::Masculine, Case::Nominative)
            }
        }
        "ый" | "ий" => morphology::ordinal(value, Gender::Masculine, Case::Nominative),
        // "-ой" is never the masculine nominative of a numeral: it is the
        // feminine oblique form ("к 5-ой главе", "в 5-ой").
        "ой" => morphology::ordinal(value, Gender::Feminine, Case::Prepositional),
        "я" | "ая" => morphology::ordinal(value, Gender::Feminine, Case::Nominative),
        "ю" | "ую" => morphology::ordinal(value, Gender::Feminine, Case::Accusative),
        "е" | "ое" => morphology::ordinal(value, Gender::Neuter, Case::Nominative),
        _ => morphology::ordinal(value, Gender::Masculine, Case::Nominative),
    };
    Some(reading)
}

/// A decade suffix (`1990-х`, `2000-е`) reads as a decade word unless the head
/// noun makes it an ordinal ("20-е место").
fn decade_case(value: u64, suffix: &str, text: &str, end: usize) -> Option<Case> {
    let case = match suffix {
        "х" | "ых" => Case::Prepositional,
        "е" | "ые" => Case::Nominative,
        _ => return None,
    };
    if (1900..=2099).contains(&value) && value % 10 == 0 {
        return Some(case);
    }
    if (20..=90).contains(&value) && value % 10 == 0 {
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
    let case = match prev.as_str() {
        "от" | "до" | "с" | "со" | "из" | "около" => Case::Genitive,
        "по" => Case::Accusative,
        _ => Case::Nominative,
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

fn read_roman(value: u32, text: &str, end: usize) -> String {
    let word_pos = skip_spaces(text, end);
    let (_, word) = read_word(text, word_pos);
    let case = match word {
        "веке" | "в." => Case::Prepositional,
        "века" | "веков" => Case::Genitive,
        _ => Case::Nominative,
    };
    morphology::ordinal(u64::from(value), Gender::Masculine, case)
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
    let l = value_of(left);
    let r = value_of(right);
    let plain_bounds = left.is_plain_integer() && right.is_plain_integer();

    if plain_bounds
        && is_year(l)
        && is_year(r)
        && (1900..=2100).contains(&l)
        && (1900..=2100).contains(&r)
    {
        if matches!(
            span.trailing.map(|trailing| trailing.kind),
            Some(TrailingKind::YearRange)
        ) {
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
    let mut out = format!("от {} до {}", bound_genitive(left), bound_genitive(right));
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

fn bound_genitive(number: &Number) -> String {
    let value = value_of(number);
    if let Some(frac) = &number.comma_frac {
        return morphology::fraction(value, frac, Gender::Masculine, Case::Genitive);
    }
    if number.dot_groups.len() == 1 {
        return morphology::fraction(
            value,
            &number.dot_groups[0],
            Gender::Masculine,
            Case::Genitive,
        );
    }
    morphology::count(value, Gender::Masculine, Case::Genitive)
}

// ── References and phones ────────────────────────────────────────────────────

fn read_reference(kind: Reference, number: &Number) -> String {
    let value = value_of(number);
    let reading = if kind.ordinal() {
        morphology::ordinal(value, Gender::Feminine, Case::Nominative)
    } else if kind.digit_by_digit() {
        digit_words(&number.int)
    } else {
        morphology::count(value, Gender::Masculine, Case::Nominative)
    };
    format!("{} {reading}", kind.word())
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
        if group.len() >= 3 && group.starts_with('0') {
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

fn value_of(number: &Number) -> u64 {
    number.int.parse().unwrap_or(0)
}

fn is_year(value: u64) -> bool {
    (1000..=2199).contains(&value)
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

fn ends_in_one(value: u64) -> bool {
    value % 10 == 1 && value % 100 != 11
}

/// Case of the number's context: a genitive trigger before it or a forced
/// case from an approximation sign.
fn case_at(text: &str, pos: usize, forced: Option<Case>) -> Case {
    forced.unwrap_or_else(|| {
        if lexicon::genitive_trigger(&prev_word(text, pos)) {
            Case::Genitive
        } else {
            Case::Nominative
        }
    })
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

fn append_tail(noun: &'static str, tail: &'static str) -> String {
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
