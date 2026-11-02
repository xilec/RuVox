//! Russian numeral morphology for the numeric span layer.
//!
//! The span reader asks for a number's spoken form together with the case and
//! gender the surrounding sentence requires; this module is the only place that
//! knows how Russian numerals inflect. Cardinal, ordinal, decade and fractional
//! forms are built from one set of word tables so a reading cannot drift from
//! the rest of the pipeline.

/// Grammatical gender of the noun a numeral agrees with.
///
/// Only the nominative/accusative forms of 1 and 2 differ by gender
/// ("один/одна", "два/две"); every other case form is shared.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Gender {
    Masculine,
    Feminine,
    Neuter,
}

/// Grammatical case a numeral is read in.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Case {
    Nominative,
    Genitive,
    Dative,
    Accusative,
    Instrumental,
    Prepositional,
}

impl Case {
    /// Accusative of an inanimate numeral is its nominative form.
    fn nominal(self) -> bool {
        matches!(self, Case::Nominative | Case::Accusative)
    }
}

/// Number of an ordinal: singular (gender matters) or plural.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Number {
    Singular,
    Plural,
}

// ── Cardinal numerals ─────────────────────────────────────────────────────────

/// Cardinal numeral in words: `1234` → "одна тысяча двести тридцать четыре".
///
/// The leading "одна"/"один" of a lone scale word is kept; use [`count`] for
/// the natural reading of a plain count.
pub fn cardinal(n: u64, gender: Gender, case: Case) -> String {
    cardinal_inner(n, gender, case, false)
}

/// Cardinal numeral for a plain count, dropping the numeral word of a lone
/// scale component: "1000" → "тысяча", "1 500" → "тысяча пятьсот",
/// "1000000" → "миллион".
pub fn count(n: u64, gender: Gender, case: Case) -> String {
    cardinal_inner(n, gender, case, true)
}

fn cardinal_inner(n: u64, gender: Gender, case: Case, drop_leading_one: bool) -> String {
    if n == 0 {
        return "ноль".to_string();
    }

    let mut parts: Vec<String> = Vec::new();
    let scales: [(u64, ScaleWord); 3] = [
        (1_000_000_000, ScaleWord::Billion),
        (1_000_000, ScaleWord::Million),
        (1_000, ScaleWord::Thousand),
    ];
    // Only the highest scale component of value 1 drops its numeral word
    // ("1 500" → "тысяча пятьсот", but "1 001 000" keeps the inner "одна").
    let mut dropped = false;
    for (value, scale) in scales {
        let component = (n / value) % 1_000;
        if component == 0 {
            continue;
        }
        let keep_word = !(drop_leading_one && !dropped && component == 1);
        let word = scale_word(scale, component, case);
        if keep_word {
            let numeral = cardinal_inner(component, scale.gender(), case, false);
            parts.push(format!("{numeral} {word}"));
        } else {
            parts.push(word);
            dropped = true;
        }
    }

    let remainder = n % 1_000;
    if remainder > 0 {
        parts.push(below_thousand(remainder, gender, case));
    }
    parts.join(" ")
}

#[derive(Clone, Copy)]
enum ScaleWord {
    Thousand,
    Million,
    Billion,
}

impl ScaleWord {
    fn gender(self) -> Gender {
        match self {
            // "тысяча" is feminine; миллион/миллиард are masculine.
            ScaleWord::Thousand => Gender::Feminine,
            ScaleWord::Million | ScaleWord::Billion => Gender::Masculine,
        }
    }
}

fn scale_word(scale: ScaleWord, component: u64, case: Case) -> String {
    let last_two = component % 100;
    let last_one = component % 10;
    let singular = !(11..=19).contains(&last_two) && last_one == 1;
    let few = !(11..=19).contains(&last_two) && (2..=4).contains(&last_one);

    let (nom_sg, gen_sg, gen_pl, dat_sg, dat_pl, instr_sg, instr_pl, prep_sg, prep_pl) = match scale
    {
        ScaleWord::Thousand => (
            "тысяча",
            "тысячи",
            "тысяч",
            "тысяче",
            "тысячам",
            "тысячей",
            "тысячами",
            "тысяче",
            "тысячах",
        ),
        ScaleWord::Million => (
            "миллион",
            "миллиона",
            "миллионов",
            "миллиону",
            "миллионам",
            "миллионом",
            "миллионами",
            "миллионе",
            "миллионах",
        ),
        ScaleWord::Billion => (
            "миллиард",
            "миллиарда",
            "миллиардов",
            "миллиарду",
            "миллиардам",
            "миллиардом",
            "миллиардами",
            "миллиарде",
            "миллиардах",
        ),
    };

    let word = match case {
        Case::Nominative | Case::Accusative => {
            if singular {
                nom_sg
            } else if few {
                gen_sg
            } else {
                gen_pl
            }
        }
        Case::Genitive => {
            if singular {
                gen_sg
            } else {
                gen_pl
            }
        }
        Case::Dative => {
            if singular {
                dat_sg
            } else {
                dat_pl
            }
        }
        Case::Instrumental => {
            if singular {
                instr_sg
            } else {
                instr_pl
            }
        }
        Case::Prepositional => {
            if singular {
                prep_sg
            } else {
                prep_pl
            }
        }
    };
    word.to_string()
}

fn below_thousand(n: u64, gender: Gender, case: Case) -> String {
    debug_assert!(n > 0 && n < 1000);
    let mut parts: Vec<String> = Vec::new();
    let hundreds = n / 100;
    if hundreds > 0 {
        parts.push(hundreds_word(hundreds, case).to_string());
    }
    let below = n % 100;
    if below > 0 {
        parts.push(below_hundred(below, gender, case));
    }
    parts.join(" ")
}

fn below_hundred(n: u64, gender: Gender, case: Case) -> String {
    debug_assert!(n > 0 && n < 100);
    if n < 20 {
        return ones_word(n, gender, case).to_string();
    }
    let tens = n / 10;
    let ones = n % 10;
    let tens_word = tens_word(tens, case);
    if ones == 0 {
        return tens_word.to_string();
    }
    format!("{tens_word} {}", ones_word(ones, gender, case))
}

fn hundreds_word(h: u64, case: Case) -> &'static str {
    const NOM: [&str; 10] = [
        "",
        "сто",
        "двести",
        "триста",
        "четыреста",
        "пятьсот",
        "шестьсот",
        "семьсот",
        "восемьсот",
        "девятьсот",
    ];
    const GEN: [&str; 10] = [
        "",
        "ста",
        "двухсот",
        "трёхсот",
        "четырёхсот",
        "пятисот",
        "шестисот",
        "семисот",
        "восьмисот",
        "девятисот",
    ];
    const DAT: [&str; 10] = [
        "",
        "ста",
        "двумстам",
        "трёмстам",
        "четырёмстам",
        "пятистам",
        "шестистам",
        "семистам",
        "восьмистам",
        "девятистам",
    ];
    const INSTR: [&str; 10] = [
        "",
        "ста",
        "двумястами",
        "тремястами",
        "четырьмястами",
        "пятьюстами",
        "шестьюстами",
        "семьюстами",
        "восемьюстами",
        "девятьюстами",
    ];
    const PREP: [&str; 10] = [
        "",
        "ста",
        "двухстах",
        "трёхстах",
        "четырёхстах",
        "пятистах",
        "шестистах",
        "семистах",
        "восьмистах",
        "девятистах",
    ];
    let index = h as usize;
    match case {
        Case::Nominative | Case::Accusative => NOM[index],
        Case::Genitive => GEN[index],
        Case::Dative => DAT[index],
        Case::Instrumental => INSTR[index],
        Case::Prepositional => PREP[index],
    }
}

fn tens_word(tens: u64, case: Case) -> &'static str {
    const NOM: [&str; 10] = [
        "",
        "",
        "двадцать",
        "тридцать",
        "сорок",
        "пятьдесят",
        "шестьдесят",
        "семьдесят",
        "восемьдесят",
        "девяносто",
    ];
    const GEN: [&str; 10] = [
        "",
        "",
        "двадцати",
        "тридцати",
        "сорока",
        "пятидесяти",
        "шестидесяти",
        "семидесяти",
        "восьмидесяти",
        "девяноста",
    ];
    const INSTR: [&str; 10] = [
        "",
        "",
        "двадцатью",
        "тридцатью",
        "сорока",
        "пятьюдесятью",
        "шестьюдесятью",
        "семьюдесятью",
        "восемьюдесятью",
        "девяноста",
    ];
    let index = tens as usize;
    match case {
        Case::Nominative | Case::Accusative => NOM[index],
        Case::Genitive | Case::Dative | Case::Prepositional => GEN[index],
        Case::Instrumental => INSTR[index],
    }
}

fn ones_word(n: u64, gender: Gender, case: Case) -> &'static str {
    const MASC_NOM: [&str; 20] = [
        "ноль",
        "один",
        "два",
        "три",
        "четыре",
        "пять",
        "шесть",
        "семь",
        "восемь",
        "девять",
        "десять",
        "одиннадцать",
        "двенадцать",
        "тринадцать",
        "четырнадцать",
        "пятнадцать",
        "шестнадцать",
        "семнадцать",
        "восемнадцать",
        "девятнадцать",
    ];
    const FEM_NOM: [&str; 3] = ["ноль", "одна", "две"];
    const NEUT_NOM: [&str; 2] = ["ноль", "одно"];
    const GEN: [&str; 20] = [
        "ноля",
        "одного",
        "двух",
        "трёх",
        "четырёх",
        "пяти",
        "шести",
        "семи",
        "восьми",
        "девяти",
        "десяти",
        "одиннадцати",
        "двенадцати",
        "тринадцати",
        "четырнадцати",
        "пятнадцати",
        "шестнадцати",
        "семнадцати",
        "восемнадцати",
        "девятнадцати",
    ];
    const DAT: [&str; 20] = [
        "нолю",
        "одному",
        "двум",
        "трём",
        "четырём",
        "пяти",
        "шести",
        "семи",
        "восьми",
        "девяти",
        "десяти",
        "одиннадцати",
        "двенадцати",
        "тринадцати",
        "четырнадцати",
        "пятнадцати",
        "шестнадцати",
        "семнадцати",
        "восемнадцати",
        "девятнадцати",
    ];
    const INSTR: [&str; 20] = [
        "нолём",
        "одним",
        "двумя",
        "тремя",
        "четырьмя",
        "пятью",
        "шестью",
        "семью",
        "восемью",
        "девятью",
        "десятью",
        "одиннадцатью",
        "двенадцатью",
        "тринадцатью",
        "четырнадцатью",
        "пятнадцатью",
        "шестнадцатью",
        "семнадцатью",
        "восемнадцатью",
        "девятнадцатью",
    ];
    const PREP: [&str; 20] = [
        "ноле",
        "одном",
        "двух",
        "трёх",
        "четырёх",
        "пяти",
        "шести",
        "семи",
        "восьми",
        "девяти",
        "десяти",
        "одиннадцати",
        "двенадцати",
        "тринадцати",
        "четырнадцати",
        "пятнадцати",
        "шестнадцати",
        "семнадцати",
        "восемнадцати",
        "девятнадцати",
    ];
    let index = n as usize;
    match case {
        Case::Nominative | Case::Accusative => match (index, gender) {
            (1, Gender::Feminine) => FEM_NOM[1],
            (2, Gender::Feminine) => FEM_NOM[2],
            (1, Gender::Neuter) => NEUT_NOM[1],
            _ => MASC_NOM[index],
        },
        Case::Genitive => {
            if index == 1 && gender == Gender::Feminine {
                "одной"
            } else if index == 2 && gender == Gender::Feminine {
                "двух"
            } else {
                GEN[index]
            }
        }
        Case::Dative => {
            if index == 1 && gender == Gender::Feminine {
                "одной"
            } else {
                DAT[index]
            }
        }
        Case::Instrumental => {
            if index == 1 && gender == Gender::Feminine {
                "одной"
            } else {
                INSTR[index]
            }
        }
        Case::Prepositional => {
            if index == 1 && gender == Gender::Feminine {
                "одной"
            } else {
                PREP[index]
            }
        }
    }
}

/// One digit read by name, for codes and identifiers.
pub fn digit_word(digit: char) -> &'static str {
    match digit {
        '0' => "ноль",
        '1' => "один",
        '2' => "два",
        '3' => "три",
        '4' => "четыре",
        '5' => "пять",
        '6' => "шесть",
        '7' => "семь",
        '8' => "восемь",
        '9' => "девять",
        _ => "",
    }
}

// ── Ordinal numerals ──────────────────────────────────────────────────────────

/// Ordinal numeral: `22` → "двадцать второй" (masculine nominative).
pub fn ordinal(n: u64, gender: Gender, case: Case) -> String {
    ordinal_inner(n, gender, case, Number::Singular)
}

/// Plural ordinal: "в 5-х классах" → "пятых".
pub fn ordinal_plural(n: u64, case: Case) -> String {
    ordinal_inner(n, Gender::Masculine, case, Number::Plural)
}

fn ordinal_inner(n: u64, gender: Gender, case: Case, number: Number) -> String {
    if n == 0 {
        return ending("нулев", Class::Stressed, gender, case, number).to_string();
    }

    let thousands = n / 1_000;
    let remainder = n % 1_000;

    // Round thousands are one word: 2000 → "двухтысячный", 1000 → "тысячный".
    if thousands > 0 && remainder == 0 {
        let stem = format!("{}тысячн", combining_prefix(thousands));
        return ending(&stem, Class::Hard, gender, case, number);
    }

    let mut parts: Vec<String> = Vec::new();
    if thousands > 0 {
        parts.push(count(
            thousands * 1_000,
            Gender::Masculine,
            Case::Nominative,
        ));
    }

    let hundreds = remainder / 100;
    let below = remainder % 100;
    if below == 0 {
        parts.push(ending(
            hundreds_ordinal_stem(hundreds),
            Class::Hard,
            gender,
            case,
            number,
        ));
    } else {
        if hundreds > 0 {
            parts.push(hundreds_word(hundreds, Case::Nominative).to_string());
        }
        parts.push(ordinal_below_hundred(below, gender, case, number));
    }
    parts.join(" ")
}

fn ordinal_below_hundred(n: u64, gender: Gender, case: Case, number: Number) -> String {
    if n < 20 {
        let (stem, class) = ones_ordinal_stem(n);
        return ending(stem, class, gender, case, number);
    }
    let tens = n / 10;
    let ones = n % 10;
    if ones == 0 {
        return ending(
            tens_ordinal_stem(tens),
            // "сороковой" is the one stressed tens ordinal.
            if tens == 4 {
                Class::Stressed
            } else {
                Class::Hard
            },
            gender,
            case,
            number,
        );
    }
    format!(
        "{} {}",
        tens_word(tens, Case::Nominative),
        ordinal_below_hundred(ones, gender, case, number)
    )
}

fn ones_ordinal_stem(n: u64) -> (&'static str, Class) {
    match n {
        1 => ("перв", Class::Hard),
        2 => ("втор", Class::Stressed),
        3 => ("трет", Class::Soft),
        4 => ("четвёрт", Class::Hard),
        5 => ("пят", Class::Hard),
        6 => ("шест", Class::Stressed),
        7 => ("седьм", Class::Stressed),
        8 => ("восьм", Class::Stressed),
        9 => ("девят", Class::Hard),
        10 => ("десят", Class::Hard),
        11 => ("одиннадцат", Class::Hard),
        12 => ("двенадцат", Class::Hard),
        13 => ("тринадцат", Class::Hard),
        14 => ("четырнадцат", Class::Hard),
        15 => ("пятнадцат", Class::Hard),
        16 => ("шестнадцат", Class::Hard),
        17 => ("семнадцат", Class::Hard),
        18 => ("восемнадцат", Class::Hard),
        19 => ("девятнадцат", Class::Hard),
        _ => unreachable!("ordinal ones are 1-19"),
    }
}

fn tens_ordinal_stem(tens: u64) -> &'static str {
    match tens {
        2 => "двадцат",
        3 => "тридцат",
        4 => "сороков",
        5 => "пятидесят",
        6 => "шестидесят",
        7 => "семидесят",
        8 => "восьмидесят",
        9 => "девяност",
        _ => unreachable!("ordinal tens are 2-9"),
    }
}

fn hundreds_ordinal_stem(hundreds: u64) -> &'static str {
    match hundreds {
        1 => "сот",
        2 => "двухсот",
        3 => "трёхсот",
        4 => "четырёхсот",
        5 => "пятисот",
        6 => "шестисот",
        7 => "семисот",
        8 => "восьмисот",
        9 => "девятисот",
        _ => unreachable!("ordinal hundreds are 1-9"),
    }
}

/// Combining form of a count used inside a compound adjective
/// ("два" → "двух" in "двухтысячный").
fn combining_prefix(n: u64) -> String {
    match n {
        1 => String::new(),
        2 => "двух".to_string(),
        3 => "трёх".to_string(),
        4 => "четырёх".to_string(),
        _ => {
            // Everything from five on uses the genitive stem of the last word:
            // "пяти", "десяти", "двадцати", ...
            let genitive = cardinal(n, Gender::Masculine, Case::Genitive);
            genitive.replace(' ', "")
        }
    }
}

#[derive(Clone, Copy)]
enum Class {
    /// Unstressed hard stem: "пятый".
    Hard,
    /// Stressed hard stem: "второй", "сороковой".
    Stressed,
    /// Soft stem: "третий".
    Soft,
}

fn ending(stem: &str, class: Class, gender: Gender, case: Case, number: Number) -> String {
    let suffix = match (number, class, gender, case) {
        (Number::Plural, Class::Soft, _, Case::Nominative | Case::Accusative) => "ьи",
        (Number::Plural, Class::Soft, _, Case::Genitive | Case::Prepositional) => "ьих",
        (Number::Plural, Class::Soft, _, Case::Dative) => "ьим",
        (Number::Plural, Class::Soft, _, Case::Instrumental) => "ьими",
        (Number::Plural, _, _, Case::Nominative | Case::Accusative) => "ые",
        (Number::Plural, _, _, Case::Genitive | Case::Prepositional) => "ых",
        (Number::Plural, _, _, Case::Dative) => "ым",
        (Number::Plural, _, _, Case::Instrumental) => "ыми",
        (Number::Singular, Class::Soft, Gender::Feminine, Case::Accusative) => "ью",
        (Number::Singular, Class::Soft, Gender::Masculine, Case::Accusative) => "ий",
        (Number::Singular, Class::Soft, Gender::Neuter, Case::Accusative) => "ье",
        (Number::Singular, Class::Soft, _, Case::Nominative) => match gender {
            Gender::Feminine => "ья",
            Gender::Neuter => "ье",
            Gender::Masculine => "ий",
        },
        (Number::Singular, Class::Soft, _, Case::Genitive) => match gender {
            Gender::Feminine => "ьей",
            _ => "ьего",
        },
        (Number::Singular, Class::Soft, _, Case::Dative) => match gender {
            Gender::Feminine => "ьей",
            _ => "ьему",
        },
        (Number::Singular, Class::Soft, _, Case::Instrumental) => match gender {
            Gender::Feminine => "ьей",
            _ => "ьим",
        },
        (Number::Singular, Class::Soft, _, Case::Prepositional) => match gender {
            Gender::Feminine => "ьей",
            _ => "ьем",
        },
        (Number::Singular, Class::Stressed, Gender::Feminine, Case::Accusative) => "ую",
        (Number::Singular, Class::Stressed, _, Case::Nominative | Case::Accusative) => match gender
        {
            Gender::Feminine => "ая",
            Gender::Neuter => "ое",
            Gender::Masculine => "ой",
        },
        (Number::Singular, Class::Hard, Gender::Feminine, Case::Accusative) => "ую",
        (Number::Singular, Class::Hard, _, Case::Nominative | Case::Accusative) => match gender {
            Gender::Feminine => "ая",
            Gender::Neuter => "ое",
            Gender::Masculine => "ый",
        },
        (Number::Singular, _, _, Case::Genitive) => match gender {
            Gender::Feminine => "ой",
            _ => "ого",
        },
        (Number::Singular, _, _, Case::Dative) => match gender {
            Gender::Feminine => "ой",
            _ => "ому",
        },
        (Number::Singular, _, _, Case::Instrumental) => match gender {
            Gender::Feminine => "ой",
            _ => "ым",
        },
        (Number::Singular, _, _, Case::Prepositional) => match gender {
            Gender::Feminine => "ой",
            _ => "ом",
        },
    };
    format!("{stem}{suffix}")
}

// ── Decades ───────────────────────────────────────────────────────────────────

/// Decade word: `1990` → "девяностые", `2000` → "двухтысячные".
///
/// Returns `None` for a value that is not a decade start.
pub fn decade(value: u64, case: Case) -> Option<String> {
    let stem = decade_stem(value)?;
    Some(ending(
        stem,
        Class::Hard,
        Gender::Masculine,
        case,
        Number::Plural,
    ))
}

fn decade_stem(value: u64) -> Option<&'static str> {
    if value == 1900 {
        return Some("девятисот");
    }
    if value == 2000 {
        return Some("двухтысячн");
    }
    if (1900..=1999).contains(&value) && value % 10 == 0 {
        let tens = (value % 100) / 10;
        return Some(tens_ordinal_stem(tens));
    }
    if (20..=90).contains(&value) && value % 10 == 0 {
        let tens = value / 10;
        return Some(tens_ordinal_stem(tens));
    }
    None
}

// ── Fractional numerals ───────────────────────────────────────────────────────

/// Fractional numeral read as a natural quantity: "1,5" → "полтора",
/// "2,5" → "два с половиной", "0,25" → "ноль целых двадцать пять сотых".
///
/// `gender` selects "полтора"/"полторы" and the integer part's gender;
/// `case` is the case the quantity is used in (the genitive form of
/// "1,5" is "полутора").
pub fn fraction(int: u64, frac: &str, gender: Gender, case: Case) -> String {
    let digits: Vec<char> = frac.chars().collect();
    let denominator = match digits.len() {
        1 => "десятых",
        2 => "сотых",
        3 => "тысячных",
        4 => "десятитысячных",
        5 => "стотысячных",
        _ => "миллионных",
    };
    let numerator = cardinal(parse_digits(frac), Gender::Feminine, Case::Nominative);

    if int == 0 {
        return format!("ноль целых {numerator} {denominator}");
    }

    // "N с половиной" is the natural reading of a half; "полтора" covers 1.5.
    if frac == "5" {
        return match int {
            1 => match (case, gender) {
                (Case::Genitive, _) => "полутора".to_string(),
                (_, Gender::Feminine) => "полторы".to_string(),
                _ => "полтора".to_string(),
            },
            _ => format!(
                "{} с половиной",
                count(
                    int,
                    gender,
                    if case.nominal() {
                        Case::Nominative
                    } else {
                        case
                    }
                )
            ),
        };
    }
    if int == 1 {
        let whole = match case {
            Case::Genitive => "одной целой",
            Case::Dative => "одной целой",
            Case::Instrumental => "одной целой",
            Case::Prepositional => "одной целой",
            _ => "одна целая",
        };
        return format!("{whole} {numerator} {denominator}");
    }
    let whole_case = if case.nominal() {
        Case::Nominative
    } else {
        case
    };
    format!(
        "{} целых {numerator} {denominator}",
        cardinal(int, gender, whole_case)
    )
}

/// Simple fraction "1/2" → "одна вторая", "2/3" → "две третьих".
pub fn simple_fraction(numerator: u64, denominator: u64) -> Option<String> {
    let (stem, class) = match denominator {
        2 => ("втор", Class::Stressed),
        3 => ("трет", Class::Soft),
        4 => ("четвёрт", Class::Hard),
        5 => ("пят", Class::Hard),
        6 => ("шест", Class::Stressed),
        7 => ("седьм", Class::Stressed),
        8 => ("восьм", Class::Stressed),
        9 => ("девят", Class::Hard),
        10 => ("десят", Class::Hard),
        _ => return None,
    };
    // A single denominator is a feminine adjective ("одна вторая"), a count
    // of them a genitive plural ("две третьих", "три четвёртых").
    let (number, case) = if numerator == 1 {
        (Number::Singular, Case::Nominative)
    } else {
        (Number::Plural, Case::Genitive)
    };
    Some(format!(
        "{} {}",
        cardinal(numerator, Gender::Feminine, Case::Nominative),
        ending(stem, class, Gender::Feminine, case, number)
    ))
}

fn parse_digits(digits: &str) -> u64 {
    digits.parse().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(n: u64) -> String {
        cardinal(n, Gender::Masculine, Case::Nominative)
    }

    #[test]
    fn cardinal_basics() {
        assert_eq!(m(0), "ноль");
        assert_eq!(m(42), "сорок два");
        assert_eq!(m(123), "сто двадцать три");
        assert_eq!(m(1000), "одна тысяча");
        assert_eq!(m(1500), "одна тысяча пятьсот");
        assert_eq!(m(1_000_000), "один миллион");
        assert_eq!(cardinal(2, Gender::Feminine, Case::Nominative), "две");
    }

    #[test]
    fn count_drops_a_lone_scale_word() {
        assert_eq!(count(1000, Gender::Masculine, Case::Nominative), "тысяча");
        assert_eq!(
            count(1500, Gender::Masculine, Case::Nominative),
            "тысяча пятьсот"
        );
        assert_eq!(
            count(1_000_000, Gender::Masculine, Case::Nominative),
            "миллион"
        );
        assert_eq!(
            count(2000, Gender::Masculine, Case::Nominative),
            "две тысячи"
        );
        assert_eq!(
            count(10_500, Gender::Masculine, Case::Nominative),
            "десять тысяч пятьсот"
        );
    }

    #[test]
    fn cardinal_genitive() {
        assert_eq!(cardinal(5, Gender::Masculine, Case::Genitive), "пяти");
        assert_eq!(cardinal(10, Gender::Masculine, Case::Genitive), "десяти");
        assert_eq!(
            cardinal(18, Gender::Masculine, Case::Genitive),
            "восемнадцати"
        );
        assert_eq!(cardinal(2, Gender::Masculine, Case::Genitive), "двух");
        assert_eq!(cardinal(1, Gender::Masculine, Case::Genitive), "одного");
        assert_eq!(cardinal(100, Gender::Masculine, Case::Genitive), "ста");
        assert_eq!(cardinal(200, Gender::Masculine, Case::Genitive), "двухсот");
    }

    #[test]
    fn ordinals() {
        let o = |n| ordinal(n, Gender::Masculine, Case::Nominative);
        assert_eq!(o(1), "первый");
        assert_eq!(o(2), "второй");
        assert_eq!(o(3), "третий");
        assert_eq!(o(5), "пятый");
        assert_eq!(o(22), "двадцать второй");
        assert_eq!(o(100), "сотый");
        assert_eq!(o(1000), "тысячный");
        assert_eq!(o(2000), "двухтысячный");
        assert_eq!(o(2024), "две тысячи двадцать четвёртый");
        assert_eq!(o(1999), "тысяча девятьсот девяносто девятый");
        assert_eq!(o(1900), "тысяча девятисотый");
        assert_eq!(
            ordinal(2024, Gender::Masculine, Case::Prepositional),
            "две тысячи двадцать четвёртом"
        );
        assert_eq!(
            ordinal(1999, Gender::Masculine, Case::Genitive),
            "тысяча девятьсот девяносто девятого"
        );
        assert_eq!(
            ordinal(2030, Gender::Masculine, Case::Dative),
            "две тысячи тридцатому"
        );
        assert_eq!(
            ordinal(2000, Gender::Masculine, Case::Genitive),
            "двухтысячного"
        );
        assert_eq!(ordinal(3, Gender::Feminine, Case::Accusative), "третью");
        assert_eq!(ordinal(5, Gender::Neuter, Case::Nominative), "пятое");
        assert_eq!(ordinal_plural(5, Case::Prepositional), "пятых");
        assert_eq!(ordinal_plural(1, Case::Instrumental), "первыми");
    }

    #[test]
    fn decades() {
        assert_eq!(
            decade(1990, Case::Prepositional).as_deref(),
            Some("девяностых")
        );
        assert_eq!(
            decade(1990, Case::Nominative).as_deref(),
            Some("девяностые")
        );
        assert_eq!(
            decade(2000, Case::Prepositional).as_deref(),
            Some("двухтысячных")
        );
        assert_eq!(
            decade(30, Case::Prepositional).as_deref(),
            Some("тридцатых")
        );
        assert_eq!(decade(2024, Case::Nominative), None);
    }

    #[test]
    fn fractions() {
        assert_eq!(
            fraction(1, "5", Gender::Masculine, Case::Nominative),
            "полтора"
        );
        assert_eq!(
            fraction(1, "5", Gender::Feminine, Case::Nominative),
            "полторы"
        );
        assert_eq!(
            fraction(2, "5", Gender::Masculine, Case::Nominative),
            "два с половиной"
        );
        assert_eq!(
            fraction(0, "5", Gender::Masculine, Case::Nominative),
            "ноль целых пять десятых"
        );
        assert_eq!(
            fraction(1, "25", Gender::Masculine, Case::Nominative),
            "одна целая двадцать пять сотых"
        );
        assert_eq!(
            fraction(3, "75", Gender::Masculine, Case::Nominative),
            "три целых семьдесят пять сотых"
        );
        assert_eq!(
            fraction(1, "5", Gender::Masculine, Case::Genitive),
            "полутора"
        );
        assert_eq!(simple_fraction(1, 2).as_deref(), Some("одна вторая"));
        assert_eq!(simple_fraction(2, 3).as_deref(), Some("две третьих"));
        assert_eq!(simple_fraction(3, 4).as_deref(), Some("три четвёртых"));
    }
}
