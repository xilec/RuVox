//! Candidate scanner and parser for the numeric span layer.
//!
//! The scanner walks the text once and, at every position that can start a
//! numeric construct, tries to consume the whole construct — digits, the
//! separators that belong to them, and the words that belong to the expression
//! (units, currencies, scale words, month names, reference prefixes). The
//! parser does not classify: it records the shape and leaves the reading to
//! [`super::read`], so a construct is either consumed whole or not at all.

use super::lexicon::{self, Currency, DEGREE_CELSIUS, Reference, Scale, UNIT_TOKENS, Unit};

// ── Span model ────────────────────────────────────────────────────────────────

/// A numeric sign the reading prepends ("минус", "плюс").
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Sign {
    Minus,
    Plus,
}

/// An approximation prefix the scanner consumes together with the number.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Approx {
    /// "~" → "около" and the genitive.
    Tilde,
    /// "≈" → "примерно".
    Almost,
    /// "±" → "плюс-минус".
    PlusMinus,
}

/// A mixed number's fractional tail.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum MixedFraction {
    Half,
    Quarter,
    ThreeQuarters,
}

/// The time part of a clock value.
#[derive(Clone, Debug)]
pub(crate) struct Time {
    pub hours: String,
    pub minutes: String,
    pub seconds: Option<String>,
}

/// A number literal with everything that was attached to it.
#[derive(Clone, Debug, Default)]
pub(crate) struct Number {
    pub sign: Option<Sign>,
    pub approx: Option<Approx>,
    /// Currency symbol before the number ("$5", "€ 20").
    pub currency_prefix: Option<Currency>,
    pub int: String,
    pub grouped: bool,
    pub dot_groups: Vec<String>,
    pub comma_frac: Option<String>,
    pub time: Option<Time>,
    pub slash_groups: Vec<String>,
    pub ordinal_suffix: Option<&'static str>,
    pub version_suffix: Option<&'static str>,
    pub leading_dot: bool,
    pub half: Option<MixedFraction>,
}

impl Number {
    /// One dot group is a decimal or a version, two or more a version.
    fn is_dot_decimal(&self) -> bool {
        self.dot_groups.len() == 1 && self.comma_frac.is_none() && self.time.is_none()
    }

    /// Whether the number is a bare integer with no other shape.
    pub(crate) fn is_plain_integer(&self) -> bool {
        self.dot_groups.is_empty()
            && self.comma_frac.is_none()
            && self.time.is_none()
            && self.slash_groups.is_empty()
            && self.ordinal_suffix.is_none()
            && self.version_suffix.is_none()
            && !self.leading_dot
            && self.half.is_none()
    }
}

/// A date whose day and month are known.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Date {
    pub day: u32,
    pub month: u32,
    pub year: Option<u32>,
}

/// The trailing construct the number owns.
#[derive(Clone, Copy, Debug)]
pub(crate) enum TrailingKind {
    Percent,
    Unit(Unit),
    Currency(Currency),
    Scale(Scale),
    /// "г." after a year → "года".
    YearAbbreviation,
    /// "годы"/"гг." after a year range → "год".
    YearRange,
    /// "5+" → "и больше".
    Plus,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Trailing {
    pub kind: TrailingKind,
    /// Currency after a scale word ("5 тыс. ₽").
    pub currency: Option<Currency>,
}

// The layer is a transient parse artifact: boxing the large variants would
// only add an allocation per span, while a span is consumed immediately.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug)]
pub(crate) enum Body {
    Number(Number),
    Date(Date),
    /// "2024-12" — a month and its year, which must not read as a range.
    YearMonth {
        year: u32,
        month: u32,
    },
    Range {
        left: Number,
        right: Number,
    },
    Reference {
        kind: Reference,
        number: Number,
    },
    Phone {
        groups: Vec<String>,
        labeled: bool,
    },
    /// "г. Москва" — the city abbreviation.
    City,
    /// A roman century ("XX век").
    Roman(u32),
}

/// One recognised candidate and the byte range it replaces.
#[derive(Clone, Debug)]
pub(crate) struct Span {
    pub start: usize,
    pub end: usize,
    pub body: Body,
    pub trailing: Option<Trailing>,
}

// ── Scanner ───────────────────────────────────────────────────────────────────

/// Scan `text` for numeric candidates.
///
/// `expand_units` gates the unit family at parse time: with units left to the
/// engine, a construct carrying a unit, percentage, currency or scale word is
/// not claimed, so its digits stay with the symbol.
pub(crate) fn scan(text: &str, expand_units: bool) -> Vec<Span> {
    let mut spans = Vec::new();
    let mut pos = 0;
    while pos < text.len() {
        let ch = text[pos..].chars().next().expect("a char at a boundary");
        let candidate = if ch.is_ascii_digit() {
            if digit_start_allowed(text, pos) {
                parse_at_digit(text, pos, None, expand_units)
            } else {
                None
            }
        } else {
            parse_at_symbol(text, pos, expand_units)
        };
        match candidate {
            Some(span) if span.end > pos => {
                pos = span.end;
                spans.push(span);
            }
            // A parser always advances; treating a zero-width span as a miss
            // keeps a future shape bug from stalling the scan.
            Some(_) | None => pos += ch.len_utf8(),
        }
    }
    spans
}

/// Whether a digit run at `pos` belongs to the numeric layer.
///
/// Fragments another phase owns are skipped here rather than being partially
/// consumed: a digit glued to a letter is an alphanumeric identifier token
/// ("x86", "H264"), a digit after `.`-after-letter is a dotted label
/// ("example.5"), and a digit after `/` or a letter-prefixed `-` is a path or
/// an identifier part ("HTTP/2", "UTF-8").
fn digit_start_allowed(text: &str, pos: usize) -> bool {
    let Some(prev) = text[..pos].chars().next_back() else {
        return true;
    };
    if prev.is_ascii_digit() || prev.is_alphabetic() || prev == '_' {
        return false;
    }
    if prev == '.' {
        return !text[..pos - 1]
            .chars()
            .next_back()
            .is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '.');
    }
    if prev == '/' || prev == '\\' {
        return false;
    }
    if prev == '-' {
        return !text[..pos - 1]
            .chars()
            .next_back()
            .is_some_and(|c| c.is_alphabetic() || c == '_');
    }
    true
}

fn is_word_start(text: &str, pos: usize) -> bool {
    text[..pos]
        .chars()
        .next_back()
        .is_none_or(|c| !c.is_alphanumeric() && c != '_')
}

fn parse_at_symbol(text: &str, pos: usize, expand_units: bool) -> Option<Span> {
    let ch = text[pos..].chars().next()?;
    match ch {
        '~' => parse_approx(text, pos, Approx::Tilde, expand_units),
        '≈' => parse_approx(text, pos, Approx::Almost, expand_units),
        '±' => parse_approx(text, pos, Approx::PlusMinus, expand_units),
        '$' | '€' | '£' | '₽' => parse_currency_prefix(text, pos, expand_units),
        '№' => parse_reference(text, pos, expand_units),
        '+' | '-' | '−' => parse_signed_or_phone(text, pos, expand_units),
        '.' => parse_leading_dot(text, pos, expand_units),
        'v' | 'V' => parse_version_prefix(text, pos, expand_units),
        'г' | 'Г' => {
            parse_city_prefix(text, pos).or_else(|| parse_reference(text, pos, expand_units))
        }
        c if c.is_alphabetic() && is_word_start(text, pos) => {
            parse_roman(text, pos).or_else(|| parse_reference(text, pos, expand_units))
        }
        _ => None,
    }
}

fn parse_approx(text: &str, pos: usize, approx: Approx, expand_units: bool) -> Option<Span> {
    let symbol_len = text[pos..].chars().next()?.len_utf8();
    let cursor = skip_spaces(text, pos + symbol_len);
    if !text[cursor..].starts_with(|c: char| c.is_ascii_digit()) {
        return None;
    }
    let span = parse_at_digit(text, cursor, None, expand_units)?;
    let Body::Number(mut number) = span.body else {
        return None;
    };
    number.approx = Some(approx);
    Some(Span {
        start: pos,
        end: span.end,
        body: Body::Number(number),
        trailing: span.trailing,
    })
}

fn parse_currency_prefix(text: &str, pos: usize, expand_units: bool) -> Option<Span> {
    let symbol = &text[pos..pos + text[pos..].chars().next()?.len_utf8()];
    let currency = Currency::from_symbol(symbol)?;
    let cursor = skip_spaces(text, pos + symbol.len());
    if !text[cursor..].starts_with(|c: char| c.is_ascii_digit()) {
        return None;
    }
    let mut span = parse_at_digit(text, cursor, None, expand_units)?;
    let Body::Number(ref mut number) = span.body else {
        return None;
    };
    number.currency_prefix = Some(currency);
    span.start = pos;
    Some(span)
}

fn parse_signed_or_phone(text: &str, pos: usize, expand_units: bool) -> Option<Span> {
    if let Some(phone) = parse_phone(text, pos, false) {
        let mut phone = phone;
        phone.start = pos;
        return Some(phone);
    }
    let ch = text[pos..].chars().next()?;
    let sign = match ch {
        '+' => Sign::Plus,
        '-' => Sign::Minus,
        '−' => Sign::Minus,
        _ => return None,
    };
    if ch != '−' && !sign_start_allowed(text, pos) {
        return None;
    }
    let cursor = pos + ch.len_utf8();
    if !text[cursor..].starts_with(|c: char| c.is_ascii_digit()) {
        return None;
    }
    let mut span = parse_at_digit(text, cursor, Some(sign), expand_units)?;
    span.start = pos;
    Some(span)
}

/// A sign may start a number at the beginning of the text or after an opening
/// bracket or operator — never after an ordinary word, so a command-line flag
/// ("head -50") keeps its hyphen.
fn sign_start_allowed(text: &str, pos: usize) -> bool {
    let before = text[..pos].trim_end_matches([' ', '\u{a0}', '\t', '\n']);
    match before.chars().next_back() {
        None => true,
        Some(c) => "([{«\"'=:>+,;".contains(c),
    }
}

fn parse_leading_dot(text: &str, pos: usize, expand_units: bool) -> Option<Span> {
    let after_dot = pos + 1;
    if !text[after_dot..].starts_with(|c: char| c.is_ascii_digit()) {
        return None;
    }
    // A dot after a letter is a dotted label ("example.5"), not a decimal.
    if text[..pos]
        .chars()
        .next_back()
        .is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '.')
    {
        return None;
    }
    let mut parser = NumParser::new(text, after_dot);
    let frac = parser.digits()?;
    let number = Number {
        int: "0".to_string(),
        leading_dot: true,
        comma_frac: Some(frac),
        ..Number::default()
    };
    let (trailing, end) = parse_trailing(text, parser.pos, &number, expand_units)?;
    Some(Span {
        start: pos,
        end,
        body: Body::Number(number),
        trailing,
    })
}

fn parse_version_prefix(text: &str, pos: usize, expand_units: bool) -> Option<Span> {
    let after = pos + 1;
    if !text[after..].starts_with(|c: char| c.is_ascii_digit()) {
        return None;
    }
    // Only a dotted version is claimed here; "v1" is an alphanumeric token the
    // mixed-token phase reads as a whole.
    let mut parser = NumParser::new(text, after);
    let int = parser.digits()?;
    if !parser.text[parser.pos..].starts_with('.') {
        return None;
    }
    let (number, number_end) = parse_number_tail(parser, int, None)?;
    let (trailing, end) = parse_trailing(text, number_end, &number, expand_units)?;
    if trailing.is_none() && text[end..].chars().next().is_some_and(char::is_alphabetic) {
        return None;
    }
    Some(Span {
        start: pos,
        end,
        body: Body::Number(number),
        trailing,
    })
}

fn parse_city_prefix(text: &str, pos: usize) -> Option<Span> {
    if !is_word_start(text, pos) {
        return None;
    }
    let unit_len = text[pos..].chars().next()?.len_utf8();
    if !text[pos + unit_len..].starts_with('.') {
        return None;
    }
    let after = skip_spaces(text, pos + unit_len + 1);
    match text[after..].chars().next() {
        Some(c) if c.is_uppercase() => Some(Span {
            start: pos,
            end: pos + unit_len + 1,
            body: Body::City,
            trailing: None,
        }),
        _ => None,
    }
}

fn parse_roman(text: &str, pos: usize) -> Option<Span> {
    let mut end = pos;
    for ch in text[pos..].chars() {
        if matches!(ch, 'I' | 'V' | 'X' | 'L' | 'C' | 'D' | 'M') {
            end += ch.len_utf8();
        } else {
            break;
        }
    }
    if end - pos < 2 {
        return None;
    }
    let word_pos = skip_spaces(text, end);
    let (_, word) = read_word(text, word_pos);
    if !matches!(word, "век" | "веке" | "века" | "веков" | "в.") {
        return None;
    }
    let value = roman_value(&text[pos..end])?;
    Some(Span {
        start: pos,
        end,
        body: Body::Roman(value),
        trailing: None,
    })
}

fn roman_value(roman: &str) -> Option<u32> {
    let mut total = 0u32;
    let mut previous = 0u32;
    for ch in roman.chars().rev() {
        let value = match ch {
            'I' => 1,
            'V' => 5,
            'X' => 10,
            'L' => 50,
            'C' => 100,
            'D' => 500,
            'M' => 1000,
            _ => return None,
        };
        if value < previous {
            total -= value;
        } else {
            total += value;
            previous = value;
        }
    }
    (1..=39).contains(&total).then_some(total)
}

fn parse_reference(text: &str, pos: usize, expand_units: bool) -> Option<Span> {
    if !is_word_start(text, pos) && !text[pos..].starts_with('№') {
        return None;
    }
    let (kind, prefix_len) = lexicon::match_reference(&text[pos..])?;
    let cursor = skip_spaces(text, pos + prefix_len);
    let colon = usize::from(text[cursor..].starts_with(':'));
    let after_colon = skip_spaces(text, cursor + colon);
    if kind == Reference::Phone {
        if let Some(phone) = parse_phone(text, after_colon, true) {
            let mut phone = phone;
            phone.start = pos;
            return Some(phone);
        }
        return None;
    }
    if !text[after_colon..].starts_with(|c: char| c.is_ascii_digit()) {
        return None;
    }
    let span = parse_at_digit(text, after_colon, None, expand_units)?;
    let Body::Number(number) = span.body else {
        return None;
    };
    Some(Span {
        start: pos,
        end: span.end,
        body: Body::Reference { kind, number },
        trailing: None,
    })
}

/// A phone number read in groups: digits separated by spaces, parentheses or
/// dashes ("+7 (999) 123-45-67").
fn parse_phone(text: &str, pos: usize, labeled: bool) -> Option<Span> {
    let mut cursor = pos;
    let mut sign = false;
    match text[cursor..].chars().next() {
        Some('+') => {
            sign = true;
            cursor += 1;
        }
        Some('8') if labeled => {}
        _ => return None,
    }
    let mut groups: Vec<String> = Vec::new();
    loop {
        let before = cursor;
        while let Some(ch) = text[cursor..].chars().next() {
            if matches!(ch, '(' | ' ' | '\u{a0}' | '-' | '\u{2013}') {
                cursor += ch.len_utf8();
            } else {
                break;
            }
        }
        let (end, digits) = read_digits(text, cursor);
        if digits.is_empty() || digits.len() > 4 {
            cursor = before;
            break;
        }
        cursor = end;
        groups.push(digits.to_string());
        if text[cursor..].starts_with(')') {
            cursor += 1;
        }
        if groups.len() >= 6 {
            break;
        }
        let probe = skip_spaces(text, cursor);
        if probe == cursor && !text[cursor..].starts_with('-') && !text[cursor..].starts_with('(') {
            break;
        }
    }
    if groups.len() < 3 {
        return None;
    }
    if sign {
        groups.insert(0, "+".to_string());
    }
    Some(Span {
        start: pos,
        end: cursor,
        body: Body::Phone { groups, labeled },
        trailing: None,
    })
}

// ── Digit candidates ──────────────────────────────────────────────────────────

/// Parse everything that can start with a digit at `pos`.
fn parse_at_digit(text: &str, pos: usize, sign: Option<Sign>, expand_units: bool) -> Option<Span> {
    if let Some(span) = parse_date(text, pos) {
        return Some(span);
    }
    if let Some(span) = parse_time(text, pos) {
        return Some(span);
    }
    if let Some(span) = parse_month_date(text, pos) {
        return Some(span);
    }
    parse_plain(text, pos, sign, expand_units)
}

fn parse_plain(text: &str, pos: usize, sign: Option<Sign>, expand_units: bool) -> Option<Span> {
    let mut parser = NumParser::new(text, pos);
    let int = parser.digits()?;
    let (number, number_end) = parse_number_tail(parser, int, sign)?;

    if let Some((right, right_end)) = parse_range_rhs(text, number_end) {
        let (trailing, end) = parse_range_trailing(text, right_end, &right, expand_units)?;
        if trailing.is_none() && text[end..].chars().next().is_some_and(char::is_alphabetic) {
            return None;
        }
        return Some(Span {
            start: pos,
            end,
            body: Body::Range {
                left: number,
                right,
            },
            trailing,
        });
    }

    let (trailing, end) = parse_trailing(text, number_end, &number, expand_units)?;
    // A digit glued to a letter is an alphanumeric token ("3d", "42x") that the
    // mixed-token phase reads as a whole; a known unit ("100MB") already
    // claimed the letters above.
    if trailing.is_none() && text[end..].chars().next().is_some_and(char::is_alphabetic) {
        return None;
    }
    Some(Span {
        start: pos,
        end,
        body: Body::Number(number),
        trailing,
    })
}

/// Continue a number after its integer part: digit groups, decimals, dotted
/// groups, a slash fraction, an ordinal suffix, a version suffix or a mixed
/// fraction tail.
fn parse_number_tail(
    mut parser: NumParser<'_>,
    int: String,
    sign: Option<Sign>,
) -> Option<(Number, usize)> {
    let mut number = Number {
        sign,
        int,
        ..Number::default()
    };

    loop {
        if let Some(group) = parser.space_group() {
            number.int.push_str(&group);
            number.grouped = true;
            continue;
        }
        if number.comma_frac.is_none() {
            if let Some(frac) = parser.comma_fraction() {
                number.comma_frac = Some(frac);
                break;
            }
        }
        if number.dot_groups.len() < 3 {
            if let Some(group) = parser.dot_group() {
                number.dot_groups.push(group);
                continue;
            }
        }
        if number.slash_groups.len() < 2 {
            if let Some(group) = parser.slash_group() {
                number.slash_groups.push(group);
                continue;
            }
        }
        break;
    }

    // A pre-release suffix belongs to a dotted version ("1.0.0-beta").
    if !number.dot_groups.is_empty() {
        if let Some((word, length)) = version_suffix_at(&parser.text[parser.pos..]) {
            number.version_suffix = Some(word);
            parser.pos += length;
            if let Some(group) = parser.dot_group() {
                number.dot_groups.push(group);
            }
        }
    }

    // An ordinal suffix is a dash plus a case ending ("5-й", "21-го").
    if number.dot_groups.is_empty() && number.comma_frac.is_none() && number.slash_groups.is_empty()
    {
        if let Some((suffix, length)) = ordinal_suffix_at(&parser.text[parser.pos..]) {
            number.ordinal_suffix = Some(suffix);
            parser.pos += length;
        }
    }

    if let Some((half, length)) = half_at(&parser.text[parser.pos..]) {
        number.half = Some(half);
        parser.pos += length;
    }

    let end = parser.pos;
    Some((number, end))
}

/// A dash between two numbers.
fn parse_range_rhs(text: &str, pos: usize) -> Option<(Number, usize)> {
    let dash_start = skip_spaces(text, pos);
    let dash = text[dash_start..].chars().next()?;
    if !matches!(dash, '-' | '–' | '—') {
        return None;
    }
    let after = skip_spaces(text, dash_start + dash.len_utf8());
    if !text[after..].starts_with(|c: char| c.is_ascii_digit()) {
        return None;
    }
    let mut parser = NumParser::new(text, after);
    let int = parser.digits()?;
    parse_number_tail(parser, int, None)
}

// ── Dates and times ──────────────────────────────────────────────────────────

fn parse_date(text: &str, pos: usize) -> Option<Span> {
    let mut parser = NumParser::new(text, pos);
    let first = parser.digits()?;
    let separator = parser.peek()?;
    if !matches!(separator, '.' | '-' | '/') {
        return None;
    }
    parser.pos += 1;
    let second = parser.digits()?;

    if parser.peek() == Some(separator) {
        let save = parser.pos;
        parser.pos += 1;
        if let Some(third) = parser.digits() {
            if let Some(date) = classify_date(&first, &second, &third) {
                let end = consume_optional_year_word(text, parser.pos);
                return Some(Span {
                    start: pos,
                    end,
                    body: Body::Date(date),
                    trailing: None,
                });
            }
        }
        parser.pos = save;
    }

    // "2024-12" is a month and its year, not a range.
    if separator == '-' && first.len() == 4 && (1..=2).contains(&second.len()) {
        let year: u32 = first.parse().ok()?;
        let month: u32 = second.parse().ok()?;
        if (1900..=2100).contains(&year) && (1..=12).contains(&month) {
            return Some(Span {
                start: pos,
                end: parser.pos,
                body: Body::YearMonth { year, month },
                trailing: None,
            });
        }
    }
    None
}

fn classify_date(first: &str, second: &str, third: &str) -> Option<Date> {
    let (day, month, year) = if first.len() == 4 {
        (
            third.parse::<u32>().ok()?,
            second.parse::<u32>().ok()?,
            first.parse::<u32>().ok()?,
        )
    } else {
        let year = match third.len() {
            4 => third.parse::<u32>().ok()?,
            2 => 2000 + third.parse::<u32>().ok()?,
            _ => return None,
        };
        (
            first.parse::<u32>().ok()?,
            second.parse::<u32>().ok()?,
            year,
        )
    };
    if !(1..=31).contains(&day) || !(1..=12).contains(&month) || !(1000..=2199).contains(&year) {
        return None;
    }
    Some(Date {
        day,
        month,
        year: Some(year),
    })
}

/// Consume a "года"/"г." word that follows a numeric date's year.
fn consume_optional_year_word(text: &str, pos: usize) -> usize {
    let start = skip_spaces(text, pos);
    let (end, token) = read_token(text, start);
    if matches!(token.to_lowercase().as_str(), "года" | "год" | "г.") {
        return end;
    }
    pos
}

fn parse_month_date(text: &str, pos: usize) -> Option<Span> {
    let (day_end, day) = read_digits(text, pos);
    if day.is_empty() || day.len() > 2 {
        return None;
    }
    let day: u32 = day.parse().ok()?;
    if !(1..=31).contains(&day) {
        return None;
    }
    let word_start = skip_spaces(text, day_end);
    let (word_end, word) = read_word(text, word_start);
    let month = lexicon::month_from_genitive(&word.to_lowercase())?;
    let mut end = word_end;
    // Optional year after the month.
    let mut year = None;
    let year_start = skip_spaces(text, end);
    let (year_end, year_digits) = read_digits(text, year_start);
    if year_digits.len() == 4 {
        let value: u32 = year_digits.parse().ok()?;
        if (1000..=2199).contains(&value) {
            year = Some(value);
            end = consume_optional_year_word(text, year_end);
        }
    }
    Some(Span {
        start: pos,
        end,
        body: Body::Date(Date { day, month, year }),
        trailing: None,
    })
}

fn parse_time(text: &str, pos: usize) -> Option<Span> {
    let (hours_end, hours) = read_digits(text, pos);
    if hours.is_empty() || hours.len() > 2 {
        return None;
    }
    if !text[hours_end..].starts_with(':') {
        return None;
    }
    let (minutes_end, minutes) = read_digits(text, hours_end + 1);
    if minutes.len() != 2 {
        return None;
    }
    let mut end = minutes_end;
    let mut seconds = None;
    if text[end..].starts_with(':') {
        let (seconds_end, value) = read_digits(text, end + 1);
        if value.len() == 2 {
            seconds = Some(value.to_string());
            end = seconds_end;
        }
    }
    let hours_value: u32 = hours.parse().ok()?;
    let minutes_value: u32 = minutes.parse().ok()?;
    let seconds_value: u32 = seconds.as_deref().unwrap_or("0").parse().ok()?;
    if hours_value > 23 || minutes_value > 59 || seconds_value > 59 {
        return None;
    }
    let time = Time {
        hours: hours.to_string(),
        minutes: minutes.to_string(),
        seconds,
    };
    Some(Span {
        start: pos,
        end,
        body: Body::Number(Number {
            int: hours.to_string(),
            time: Some(time),
            ..Number::default()
        }),
        trailing: None,
    })
}

// ── Trailing constructs ──────────────────────────────────────────────────────

/// Parse the construct attached after a number: a unit, percentage, currency,
/// scale word, year abbreviation or a plus sign. Returns the trailing construct
/// (if any) and the end of the text the span owns.
fn parse_trailing(
    text: &str,
    pos: usize,
    number: &Number,
    expand_units: bool,
) -> Option<(Option<Trailing>, usize)> {
    let start = skip_spaces(text, pos);

    if text[start..].starts_with('%') {
        if !expand_units {
            return None;
        }
        return Some((
            Some(Trailing {
                kind: TrailingKind::Percent,
                currency: None,
            }),
            start + 1,
        ));
    }

    if text[start..].starts_with('+')
        && !text[start + 1..].starts_with(|c: char| c.is_alphanumeric())
    {
        return Some((
            Some(Trailing {
                kind: TrailingKind::Plus,
                currency: None,
            }),
            start + 1,
        ));
    }

    if let Some(currency) = currency_at(&text[start..]) {
        if !expand_units {
            return None;
        }
        return Some((
            Some(Trailing {
                kind: TrailingKind::Currency(currency),
                currency: None,
            }),
            start + currency_symbol_len(&text[start..]),
        ));
    }

    // Scale words and money abbreviations ("млн", "тыс.", "руб.", "коп.").
    let (_token_end, token) = read_token(text, start);
    let token = token.trim_end_matches([',', ';', ':', '!', '?', ')', '»', '"']);
    let token_lower = token.to_lowercase();
    if let Some(scale) = Scale::from_token(&token_lower) {
        if !expand_units {
            return None;
        }
        let mut end = start + token.len();
        let mut currency = None;
        let after = skip_spaces(text, end);
        if let Some(currency_kind) = currency_at(&text[after..]) {
            currency = Some(currency_kind);
            end = after + currency_symbol_len(&text[after..]);
        }
        return Some((
            Some(Trailing {
                kind: TrailingKind::Scale(scale),
                currency,
            }),
            end,
        ));
    }
    if let Some(currency) = money_word(&token_lower) {
        if !expand_units {
            return None;
        }
        return Some((
            Some(Trailing {
                kind: TrailingKind::Currency(currency),
                currency: None,
            }),
            start + token.len(),
        ));
    }

    // "г." after a year is the year abbreviation; after any other number the
    // "г" is a gram unit and the dot stays as punctuation.
    if token_lower == "г." && is_year_value(&number.int) {
        return Some((
            Some(Trailing {
                kind: TrailingKind::YearAbbreviation,
                currency: None,
            }),
            start + token.len(),
        ));
    }

    if text[start..].starts_with('°') {
        if !expand_units {
            return None;
        }
        let after_degree = skip_spaces(text, start + '°'.len_utf8());
        let (unit_end, degree) = read_word(text, after_degree);
        if degree == "C" || degree == "С" {
            return Some((
                Some(Trailing {
                    kind: TrailingKind::Unit(Unit {
                        forms: DEGREE_CELSIUS,
                        size_unit: false,
                    }),
                    currency: None,
                }),
                unit_end,
            ));
        }
        return None;
    }

    if let Some((unit, length)) = unit_at(&text[start..]) {
        // With units left to the engine the whole construct belongs to the
        // unit symbol, so the number must not be claimed either.
        if !expand_units {
            return None;
        }
        // A dotted decimal keeps a physical unit in the text (the technical
        // register is "ноль точка пять кг"), but owns size units ("1.5em")
        // and percentages.
        if (number.is_dot_decimal() || number.leading_dot) && !unit.size_unit {
            return Some((None, pos));
        }
        return Some((
            Some(Trailing {
                kind: TrailingKind::Unit(unit),
                currency: None,
            }),
            start + length,
        ));
    }

    Some((None, pos))
}

/// Trailing construct of a range: a year-range word ("годы", "гг.") or a unit,
/// percentage or currency shared by both bounds.
fn parse_range_trailing(
    text: &str,
    pos: usize,
    right: &Number,
    expand_units: bool,
) -> Option<(Option<Trailing>, usize)> {
    let start = skip_spaces(text, pos);
    let (token_end, token) = read_token(text, start);
    let token_lower = token.to_lowercase();
    if matches!(
        token_lower.as_str(),
        "годы" | "гг." | "год" | "годах" | "годов"
    ) {
        return Some((
            Some(Trailing {
                kind: TrailingKind::YearRange,
                currency: None,
            }),
            token_end,
        ));
    }
    parse_trailing(text, pos, right, expand_units)
}

fn currency_at(text: &str) -> Option<Currency> {
    let ch = text.chars().next()?;
    Currency::from_symbol(ch.to_string().as_str())
}

fn currency_symbol_len(text: &str) -> usize {
    text.chars().next().map_or(0, char::len_utf8)
}

fn money_word(token: &str) -> Option<Currency> {
    match token {
        "руб." | "руб" => Some(Currency::Ruble),
        "коп." | "коп" => Some(Currency::Kopek),
        "долл." | "долл" => Some(Currency::Dollar),
        "евро" => Some(Currency::Euro),
        _ => None,
    }
}

fn unit_at(text: &str) -> Option<(Unit, usize)> {
    for token in UNIT_TOKENS {
        let Some(candidate) = text.get(..token.len()) else {
            continue;
        };
        if candidate.to_lowercase() != token {
            continue;
        }
        let after = text.get(token.len()..).and_then(|rest| rest.chars().next());
        if after.is_some_and(|c| c.is_alphanumeric() || c == '_') {
            continue;
        }
        return lexicon::lookup_unit(token).map(|unit| (unit, token.len()));
    }
    None
}

fn is_year_value(digits: &str) -> bool {
    digits.len() == 4
        && digits
            .parse::<u32>()
            .is_ok_and(|value| (1000..=2199).contains(&value))
}

// ── Suffix tables ────────────────────────────────────────────────────────────

fn ordinal_suffix_at(text: &str) -> Option<(&'static str, usize)> {
    const SUFFIXES: [&str; 23] = [
        "ыми", "его", "ому", "ая", "ое", "ые", "ых", "ым", "ую", "ом", "ем", "им", "ой", "ый",
        "ми", "го", "му", "м", "х", "й", "я", "е", "ю",
    ];
    let rest = text.strip_prefix('-')?;
    for suffix in SUFFIXES {
        if let Some(after) = rest.strip_prefix(suffix) {
            if after.starts_with(|c: char| c.is_alphabetic()) {
                continue;
            }
            return Some((suffix, 1 + suffix.len()));
        }
    }
    None
}

fn version_suffix_at(text: &str) -> Option<(&'static str, usize)> {
    let rest = text.strip_prefix('-')?;
    let letters: String = rest
        .chars()
        .take_while(|c| c.is_ascii_alphabetic())
        .collect();
    if letters.is_empty() {
        return None;
    }
    let word = lexicon::version_suffix_word(&letters)?;
    let digits: String = rest[letters.len()..]
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    Some((word, 1 + letters.len() + digits.len()))
}

fn half_at(text: &str) -> Option<(MixedFraction, usize)> {
    let ch = text.chars().next()?;
    let half = match ch {
        '½' => MixedFraction::Half,
        '¼' => MixedFraction::Quarter,
        '¾' => MixedFraction::ThreeQuarters,
        _ => return None,
    };
    Some((half, ch.len_utf8()))
}

// ── Small text helpers ───────────────────────────────────────────────────────

#[derive(Clone)]
struct NumParser<'a> {
    text: &'a str,
    pos: usize,
}

impl<'a> NumParser<'a> {
    fn new(text: &'a str, pos: usize) -> Self {
        Self { text, pos }
    }

    fn peek(&self) -> Option<char> {
        self.text[self.pos..].chars().next()
    }

    fn digits(&mut self) -> Option<String> {
        let start = self.pos;
        while self
            .text
            .as_bytes()
            .get(self.pos)
            .is_some_and(u8::is_ascii_digit)
        {
            self.pos += 1;
        }
        (self.pos > start).then(|| self.text[start..self.pos].to_string())
    }

    /// A space/NBSP-separated three-digit group of one number ("10 000").
    fn space_group(&mut self) -> Option<String> {
        let ch = self.peek()?;
        if ch != ' ' && ch != '\u{a0}' {
            return None;
        }
        let after = skip_spaces(self.text, self.pos);
        let digits: String = self.text[after..]
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if digits.len() != 3 || self.text[after + 3..].starts_with(|c: char| c.is_ascii_digit()) {
            return None;
        }
        self.pos = after + 3;
        Some(digits)
    }

    fn comma_fraction(&mut self) -> Option<String> {
        let start = self.pos + 1;
        if !self.text[self.pos..].starts_with(',') {
            return None;
        }
        let digits: String = self.text[start..]
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if digits.is_empty() {
            return None;
        }
        self.pos = start + digits.len();
        Some(digits)
    }

    fn dot_group(&mut self) -> Option<String> {
        let start = self.pos + 1;
        if !self.text[self.pos..].starts_with('.') {
            return None;
        }
        let digits: String = self.text[start..]
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if digits.is_empty() {
            return None;
        }
        self.pos = start + digits.len();
        Some(digits)
    }

    fn slash_group(&mut self) -> Option<String> {
        let start = self.pos + 1;
        if !self.text[self.pos..].starts_with('/') {
            return None;
        }
        let digits: String = self.text[start..]
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if digits.is_empty() {
            return None;
        }
        self.pos = start + digits.len();
        Some(digits)
    }
}

pub(crate) fn skip_spaces(text: &str, pos: usize) -> usize {
    let mut cursor = pos;
    while let Some(ch) = text[cursor..].chars().next() {
        if ch == ' ' || ch == '\u{a0}' || ch == '\t' {
            cursor += ch.len_utf8();
        } else {
            break;
        }
    }
    cursor
}

pub(crate) fn read_digits(text: &str, pos: usize) -> (usize, &str) {
    let mut cursor = pos;
    while text.as_bytes().get(cursor).is_some_and(u8::is_ascii_digit) {
        cursor += 1;
    }
    (cursor, &text[pos..cursor])
}

pub(crate) fn read_word(text: &str, pos: usize) -> (usize, &str) {
    let mut cursor = pos;
    for ch in text[pos..].chars() {
        if ch.is_alphabetic() {
            cursor += ch.len_utf8();
        } else {
            break;
        }
    }
    (cursor, &text[pos..cursor])
}

fn read_token(text: &str, pos: usize) -> (usize, &str) {
    let mut cursor = pos;
    for ch in text[pos..].chars() {
        if ch.is_whitespace() {
            break;
        }
        cursor += ch.len_utf8();
    }
    (cursor, &text[pos..cursor])
}
