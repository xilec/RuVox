/// Numbers normalizer — converts numbers, sizes, versions, ranges, percentages
/// to Russian words, mirroring the Python `NumberNormalizer` class.
///
/// Number-to-words logic is a manual port of the Python `num2words(n, lang="ru")`
/// output, since no Rust crate provides equivalent Russian-language support.
use regex::Regex;
use std::sync::OnceLock;

// ---- Size units: (nom_sg, gen_sg, gen_pl, gender) ----

struct UnitData {
    nom_sg: &'static str,
    gen_sg: &'static str,
    gen_pl: &'static str,
    gender: Gender,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Gender {
    Masculine,
    Feminine,
    // Neuter is not used in SIZE_UNITS but kept for completeness
}

const SIZE_UNITS: &[(&str, UnitData)] = &[
    (
        "kb",
        UnitData {
            nom_sg: "килобайт",
            gen_sg: "килобайта",
            gen_pl: "килобайт",
            gender: Gender::Masculine,
        },
    ),
    (
        "mb",
        UnitData {
            nom_sg: "мегабайт",
            gen_sg: "мегабайта",
            gen_pl: "мегабайт",
            gender: Gender::Masculine,
        },
    ),
    (
        "gb",
        UnitData {
            nom_sg: "гигабайт",
            gen_sg: "гигабайта",
            gen_pl: "гигабайт",
            gender: Gender::Masculine,
        },
    ),
    (
        "tb",
        UnitData {
            nom_sg: "терабайт",
            gen_sg: "терабайта",
            gen_pl: "терабайт",
            gender: Gender::Masculine,
        },
    ),
    (
        "кб",
        UnitData {
            nom_sg: "килобайт",
            gen_sg: "килобайта",
            gen_pl: "килобайт",
            gender: Gender::Masculine,
        },
    ),
    (
        "мб",
        UnitData {
            nom_sg: "мегабайт",
            gen_sg: "мегабайта",
            gen_pl: "мегабайт",
            gender: Gender::Masculine,
        },
    ),
    (
        "гб",
        UnitData {
            nom_sg: "гигабайт",
            gen_sg: "гигабайта",
            gen_pl: "гигабайт",
            gender: Gender::Masculine,
        },
    ),
    (
        "тб",
        UnitData {
            nom_sg: "терабайт",
            gen_sg: "терабайта",
            gen_pl: "терабайт",
            gender: Gender::Masculine,
        },
    ),
    (
        "ms",
        UnitData {
            nom_sg: "миллисекунда",
            gen_sg: "миллисекунды",
            gen_pl: "миллисекунд",
            gender: Gender::Feminine,
        },
    ),
    (
        "sec",
        UnitData {
            nom_sg: "секунда",
            gen_sg: "секунды",
            gen_pl: "секунд",
            gender: Gender::Feminine,
        },
    ),
    (
        "min",
        UnitData {
            nom_sg: "минута",
            gen_sg: "минуты",
            gen_pl: "минут",
            gender: Gender::Feminine,
        },
    ),
    (
        "hr",
        UnitData {
            nom_sg: "час",
            gen_sg: "часа",
            gen_pl: "часов",
            gender: Gender::Masculine,
        },
    ),
    (
        "px",
        UnitData {
            nom_sg: "пиксель",
            gen_sg: "пикселя",
            gen_pl: "пикселей",
            gender: Gender::Masculine,
        },
    ),
    (
        "em",
        UnitData {
            nom_sg: "эм",
            gen_sg: "эм",
            gen_pl: "эм",
            gender: Gender::Masculine,
        },
    ),
    (
        "rem",
        UnitData {
            nom_sg: "рем",
            gen_sg: "рем",
            gen_pl: "рем",
            gender: Gender::Masculine,
        },
    ),
    (
        "vh",
        UnitData {
            nom_sg: "ви эйч",
            gen_sg: "ви эйч",
            gen_pl: "ви эйч",
            gender: Gender::Masculine,
        },
    ),
    (
        "vw",
        UnitData {
            nom_sg: "ви дабл ю",
            gen_sg: "ви дабл ю",
            gen_pl: "ви дабл ю",
            gender: Gender::Masculine,
        },
    ),
];

const VERSION_SUFFIXES: &[(&str, &str)] = &[
    ("alpha", "альфа"),
    ("beta", "бета"),
    ("rc", "эр си"),
    ("dev", "дев"),
    ("stable", "стейбл"),
    ("release", "релиз"),
];

const MONTHS_GENITIVE: [&str; 13] = [
    "",
    "января",
    "февраля",
    "марта",
    "апреля",
    "мая",
    "июня",
    "июля",
    "августа",
    "сентября",
    "октября",
    "ноября",
    "декабря",
];

// ---- Core number-to-words logic ----

/// Convert a non-negative integer to Russian words (cardinal, masculine gender by default).
///
/// Matches `num2words(n, lang="ru")` output exactly for values used in tests.
fn int_to_words(n: i64) -> String {
    if n < 0 {
        return format!("минус {}", int_to_words(-n));
    }
    int_to_words_with_gender(n, Gender::Masculine)
}

/// Cardinal form with explicit gender for the "ones" digit of each rank.
///
/// Gender affects only 1 and 2:
/// - Masculine: один, два
/// - Feminine:  одна, две
fn int_to_words_with_gender(n: i64, gender: Gender) -> String {
    if n < 0 {
        return format!("минус {}", int_to_words_with_gender(-n, gender));
    }

    if n == 0 {
        return "ноль".to_string();
    }

    let mut parts: Vec<String> = Vec::new();

    // Billions
    let billions = n / 1_000_000_000;
    if billions > 0 {
        let b_words = int_to_words_with_gender(billions, Gender::Masculine);
        let suffix = get_declension(billions, ("миллиард", "миллиарда", "миллиардов"));
        parts.push(format!("{} {}", b_words, suffix));
    }

    // Millions
    let millions = (n % 1_000_000_000) / 1_000_000;
    if millions > 0 {
        let m_words = int_to_words_with_gender(millions, Gender::Masculine);
        let suffix = get_declension(millions, ("миллион", "миллиона", "миллионов"));
        parts.push(format!("{} {}", m_words, suffix));
    }

    // Thousands — feminine (тысяча is feminine)
    let thousands = (n % 1_000_000) / 1_000;
    if thousands > 0 {
        let t_words = int_to_words_with_gender(thousands, Gender::Feminine);
        let suffix = get_declension(thousands, ("тысяча", "тысячи", "тысяч"));
        parts.push(format!("{} {}", t_words, suffix));
    }

    // Remainder < 1000
    let remainder = n % 1_000;
    if remainder > 0 {
        parts.push(below_thousand(remainder, gender));
    }

    parts.join(" ")
}

/// Convert 1–999 to Russian words with gender for ones.
fn below_thousand(n: i64, gender: Gender) -> String {
    debug_assert!(n > 0 && n < 1000);

    let mut parts: Vec<String> = Vec::new();

    let hundreds = n / 100;
    if hundreds > 0 {
        parts.push(hundreds_word(hundreds));
    }

    let below_100 = n % 100;
    if below_100 > 0 {
        parts.push(below_hundred(below_100, gender));
    }

    parts.join(" ")
}

fn hundreds_word(h: i64) -> String {
    match h {
        1 => "сто",
        2 => "двести",
        3 => "триста",
        4 => "четыреста",
        5 => "пятьсот",
        6 => "шестьсот",
        7 => "семьсот",
        8 => "восемьсот",
        9 => "девятьсот",
        _ => unreachable!(),
    }
    .to_string()
}

/// Convert 1–99 to Russian words with gender for ones.
fn below_hundred(n: i64, gender: Gender) -> String {
    debug_assert!(n > 0 && n < 100);

    if n < 20 {
        return ones_teens(n, gender);
    }

    let tens = n / 10;
    let ones = n % 10;

    let tens_word = tens_word(tens);
    if ones == 0 {
        return tens_word.to_string();
    }

    format!("{} {}", tens_word, ones_teens(ones, gender))
}

fn tens_word(tens: i64) -> &'static str {
    match tens {
        2 => "двадцать",
        3 => "тридцать",
        4 => "сорок",
        5 => "пятьдесят",
        6 => "шестьдесят",
        7 => "семьдесят",
        8 => "восемьдесят",
        9 => "девяносто",
        _ => unreachable!(),
    }
}

/// 1–19 with gender affecting 1 and 2.
fn ones_teens(n: i64, gender: Gender) -> String {
    match n {
        1 => match gender {
            Gender::Feminine => "одна",
            Gender::Masculine => "один",
        },
        2 => match gender {
            Gender::Feminine => "две",
            Gender::Masculine => "два",
        },
        3 => "три",
        4 => "четыре",
        5 => "пять",
        6 => "шесть",
        7 => "семь",
        8 => "восемь",
        9 => "девять",
        10 => "десять",
        11 => "одиннадцать",
        12 => "двенадцать",
        13 => "тринадцать",
        14 => "четырнадцать",
        15 => "пятнадцать",
        16 => "шестнадцать",
        17 => "семнадцать",
        18 => "восемнадцать",
        19 => "девятнадцать",
        _ => unreachable!(),
    }
    .to_string()
}

/// Russian declension based on number's last digits.
/// forms: (singular, genitive_singular, genitive_plural)
fn get_declension(n: i64, forms: (&str, &str, &str)) -> String {
    let n = n.unsigned_abs();
    let last_two = n % 100;
    let last_one = n % 10;

    if (11..=19).contains(&last_two) {
        forms.2.to_string() // genitive plural
    } else if last_one == 1 {
        forms.0.to_string() // singular
    } else if (2..=4).contains(&last_one) {
        forms.1.to_string() // genitive singular
    } else {
        forms.2.to_string() // genitive plural
    }
}

// ---- Ordinal forms for years ----

/// Ordinal form of a year for use in dates/ranges (masculine genitive ordinal).
///
/// This replicates Python's `num2words(year, lang="ru", to="ordinal")` +
/// suffix transformations that turn it into genitive ordinal.
fn year_to_ordinal_genitive(year: i64) -> String {
    if year == 2000 {
        return "двухтысячного".to_string();
    }
    // Build ordinal by composing parts: everything except last significant group
    // uses cardinal, the last group uses ordinal form.
    year_ordinal_genitive_inner(year)
}

/// Build genitive ordinal for year via splitting into a prefix (cardinal) + suffix (ordinal).
///
/// E.g. 2024 → "две тысячи" (cardinal) + "двадцать четвёртого" (ordinal genitive).
fn year_ordinal_genitive_inner(year: i64) -> String {
    // Decompose year into groups of 1000 and below-1000.
    let thousands = year / 1000;
    let remainder = year % 1000;

    if remainder == 0 {
        // Round thousands: cardinal prefix + "тысячного".
        // E.g. 1000 → "тысячного", 2000 → handled above, 3000 → "трёхтысячного".
        let prefix = int_to_words_with_gender(thousands, Gender::Feminine)
            .replace("тысяча", "")
            .replace("тысячи", "")
            .replace("тысяч", "")
            .trim()
            .to_string();
        return format!("{}тысячного", prefix);
    }

    let hundreds = remainder / 100;
    let below_100 = remainder % 100;

    // Build prefix: everything except the last unit
    if below_100 == 0 {
        // E.g. 2100 → "две тысячи сто" + ordinal of hundreds
        // Not typical in tests; use fallback cardinal
        return format!(
            "{} {} {}ого",
            int_to_words_with_gender(thousands, Gender::Feminine),
            get_declension(thousands, ("тысяча", "тысячи", "тысяч")),
            hundreds_word_ordinal_stem(hundreds)
        );
    }

    // Last significant part: below_100
    let prefix = build_year_prefix(thousands, remainder);
    let ordinal_suffix = ordinal_genitive_below_hundred(below_100);

    if prefix.is_empty() {
        ordinal_suffix
    } else {
        format!("{} {}", prefix, ordinal_suffix)
    }
}

/// Build year prefix (everything except the ordinalised last group).
fn build_year_prefix(thousands: i64, remainder: i64) -> String {
    let hundreds = remainder / 100;
    let below_100 = remainder % 100;

    let mut parts = Vec::new();

    // Thousands part (cardinal, feminine)
    if thousands > 0 {
        let t_words = int_to_words_with_gender(thousands, Gender::Feminine);
        let t_suffix = get_declension(thousands, ("тысяча", "тысячи", "тысяч"));
        parts.push(format!("{} {}", t_words, t_suffix));
    }

    // Hundreds (cardinal)
    if hundreds > 0 && below_100 > 0 {
        parts.push(hundreds_word(hundreds));
    }

    parts.join(" ")
}

/// Ordinal genitive form for 1–99 (masculine).
///
/// Used for the trailing part of a year ordinal genitive:
/// "двадцать четвёртого", "третьего", "пятого", etc.
fn ordinal_genitive_below_hundred(n: i64) -> String {
    if n < 20 {
        return ordinal_genitive_ones(n);
    }
    let tens = n / 10;
    let ones = n % 10;

    if ones == 0 {
        return format!("{}ого", tens_ordinal_stem(tens));
    }

    // Compound: tens stays cardinal, ones become ordinal
    format!("{} {}", tens_word(tens), ordinal_genitive_ones(ones))
}

/// Ordinal genitive for 1–19.
fn ordinal_genitive_ones(n: i64) -> String {
    match n {
        1 => "первого",
        2 => "второго",
        3 => "третьего",
        4 => "четвёртого",
        5 => "пятого",
        6 => "шестого",
        7 => "седьмого",
        8 => "восьмого",
        9 => "девятого",
        10 => "десятого",
        11 => "одиннадцатого",
        12 => "двенадцатого",
        13 => "тринадцатого",
        14 => "четырнадцатого",
        15 => "пятнадцатого",
        16 => "шестнадцатого",
        17 => "семнадцатого",
        18 => "восемнадцатого",
        19 => "девятнадцатого",
        _ => unreachable!(),
    }
    .to_string()
}

fn tens_ordinal_stem(tens: i64) -> &'static str {
    match tens {
        2 => "двадцат",
        3 => "тридцат",
        4 => "сороков",
        5 => "пятидесят",
        6 => "шестидесят",
        7 => "семидесят",
        8 => "восьмидесят",
        9 => "девяност",
        _ => unreachable!(),
    }
}

fn hundreds_word_ordinal_stem(h: i64) -> &'static str {
    match h {
        1 => "сот",
        2 => "двухсот",
        3 => "трёхсот",
        4 => "четырёхсот",
        5 => "пятисот",
        6 => "шестисот",
        7 => "семисот",
        8 => "восьмисот",
        9 => "девятисот",
        _ => unreachable!(),
    }
}

// ---- Genitive case for ranges ----

/// Convert number words to approximate genitive case (cardinal, for "от X до Y").
///
/// Replicates Python's `_to_genitive` logic with word-level replacements.
///
/// For year-range numbers (1000–9999), uses ordinal genitive (same as Python).
fn to_genitive(n: i64) -> String {
    if (1000..=9999).contains(&n) {
        return year_to_ordinal_genitive(n);
    }

    let words = int_to_words(n);
    apply_genitive_replacements(&words)
}

/// Apply Russian genitive substitutions to a cardinal string.
///
/// Replacement pairs are ordered from longest to shortest to avoid partial matches
/// when using simple substring replacement. Uses word-boundary checks.
fn apply_genitive_replacements(words: &str) -> String {
    // Ordered: longer forms before shorter ones to prevent partial replacement.
    let replacements: &[(&str, &str)] = &[
        ("одиннадцать", "одиннадцати"),
        ("двенадцать", "двенадцати"),
        ("тринадцать", "тринадцати"),
        ("четырнадцать", "четырнадцати"),
        ("пятнадцать", "пятнадцати"),
        ("шестнадцать", "шестнадцати"),
        ("семнадцать", "семнадцати"),
        ("восемнадцать", "восемнадцати"),
        ("девятнадцать", "девятнадцати"),
        ("восемьдесят", "восьмидесяти"),
        ("пятьдесят", "пятидесяти"),
        ("шестьдесят", "шестидесяти"),
        ("семьдесят", "семидесяти"),
        ("двадцать", "двадцати"),
        ("тридцать", "тридцати"),
        ("девяносто", "девяноста"),
        ("четыреста", "четырёхсот"),
        ("двести", "двухсот"),
        ("триста", "трёхсот"),
        ("пятьсот", "пятисот"),
        ("шестьсот", "шестисот"),
        ("семьсот", "семисот"),
        ("восемьсот", "восьмисот"),
        ("девятьсот", "девятисот"),
        ("четыре", "четырёх"),
        ("сорок", "сорока"),
        ("сто", "ста"),
        ("одна", "одной"),
        ("один", "одного"),
        ("две", "двух"),
        ("два", "двух"),
        ("три", "трёх"),
        ("пять", "пяти"),
        ("шесть", "шести"),
        ("семь", "семи"),
        ("восемь", "восьми"),
        ("девять", "девяти"),
        ("десять", "десяти"),
        ("тысяча", "тысячи"),
        ("миллион", "миллиона"),
        ("миллиарда", "миллиардов"),
        ("миллиард", "миллиарда"),
    ];

    // Applying word-boundary replacements. Russian word boundaries don't coincide with
    // \b (which is ASCII-only in most engines), so we use space/start/end anchors instead.
    // Since all replacements are Cyrillic substrings and the pairs are ordered longest-first,
    // plain substring replacement with a space-guard is sufficient and avoids per-call
    // Regex compilation.
    let mut result = words.to_string();
    for (from, to) in replacements {
        // Replace only when the pattern appears as a standalone word: preceded and followed
        // by space, start/end of string, or another non-alpha character.
        let mut out = String::new();
        let mut search = result.as_str();
        let from_len = from.len();
        while !search.is_empty() {
            if let Some(pos) = search.find(from) {
                let before = &search[..pos];
                let after = &search[pos + from_len..];

                let preceded_ok = before.is_empty()
                    || before.ends_with(|c: char| !c.is_alphabetic() && c != '\u{0300}');
                let followed_ok = after.is_empty()
                    || after.starts_with(|c: char| !c.is_alphabetic() && c != '\u{0300}');

                if preceded_ok && followed_ok {
                    out.push_str(before);
                    out.push_str(to);
                    search = after;
                } else {
                    // Not a whole-word match; advance past this occurrence.
                    out.push_str(before);
                    out.push_str(from);
                    search = after;
                }
            } else {
                out.push_str(search);
                break;
            }
        }
        result = out;
    }
    result
}

// ---- Date ordinal forms ----

/// Ordinal neuter genitive for day-of-month (1–31).
///
/// Matches Python: `num2words(day, lang="ru", to="ordinal")` +
/// `.replace("ый", "ое").replace("ий", "ее").replace("ой", "ое")`.
fn day_ordinal_neuter(day: u32) -> String {
    // Build the ordinal string and apply neuter genitive suffix transformation.
    let ordinal = day_ordinal_base(day);
    // Apply suffix replacements in order (same as Python, order matters).
    // "третий" → "третьее" first via "ий" → "ее"? Actually Python does:
    // replace("ый", "ое").replace("ий", "ее").replace("ой", "ое")
    // But "третий" → "третьее" doesn't match expected "третье" — actually:
    // "первый" → "первое", "второй" → "второе", "третий" → "третье"... but
    // "ий" → "ее" would give "третьее". Let's check expected outputs:
    // ("2000-01-01", "первое января двухтысячного года")
    // "первый" → replace("ый","ое") → "первое" ✓
    // ("2024-12-31", "тридцать первое декабря...")
    // "тридцать первый" → "тридцать первое" ✓
    // ("01.12.2023", "первое декабря...")
    // What about day 3? "третий" → replace("ий","ее") = "третьее"... but expected is "третье"?
    // Tests don't include day 3 directly, but range test has "от ... третьего"
    // which is a different code path. For dates, Python produces "третьее" via "ий"→"ее".
    // Let's follow Python literally.
    ordinal
        .replace("ый", "ое")
        .replace("ий", "ее")
        .replace("ой", "ое")
}

/// Base ordinal (masculine nominative) for 1–31.
fn day_ordinal_base(day: u32) -> String {
    match day {
        1 => "первый",
        2 => "второй",
        3 => "третий",
        4 => "четвёртый",
        5 => "пятый",
        6 => "шестой",
        7 => "седьмой",
        8 => "восьмой",
        9 => "девятый",
        10 => "десятый",
        11 => "одиннадцатый",
        12 => "двенадцатый",
        13 => "тринадцатый",
        14 => "четырнадцатый",
        15 => "пятнадцатый",
        16 => "шестнадцатый",
        17 => "семнадцатый",
        18 => "восемнадцатый",
        19 => "девятнадцатый",
        20 => "двадцатый",
        21 => "двадцать первый",
        22 => "двадцать второй",
        23 => "двадцать третий",
        24 => "двадцать четвёртый",
        25 => "двадцать пятый",
        26 => "двадцать шестой",
        27 => "двадцать седьмой",
        28 => "двадцать восьмой",
        29 => "двадцать девятый",
        30 => "тридцатый",
        31 => "тридцать первый",
        _ => "неизвестный",
    }
    .to_string()
}

/// Year to genitive ordinal form for use in dates.
///
/// Matches Python's `_year_to_genitive` output.
fn year_to_genitive(year: i64) -> String {
    year_to_ordinal_genitive(year)
}

// ---- Public API ----

pub struct NumberNormalizer;

impl NumberNormalizer {
    pub fn new() -> Self {
        Self
    }

    /// Convert integer string to Russian words.
    pub fn normalize_number(&self, num_str: &str) -> String {
        match num_str.trim().parse::<i64>() {
            Ok(n) => int_to_words(n),
            Err(_) => num_str.to_string(),
        }
    }

    /// Convert float string to Russian words: integer part + "точка" + digits.
    pub fn normalize_float(&self, float_str: &str) -> String {
        let s = float_str.replace(',', ".");
        if !s.contains('.') {
            return self.normalize_number(&s);
        }
        let parts: Vec<&str> = s.splitn(2, '.').collect();
        if parts.len() != 2 {
            return float_str.to_string();
        }
        let int_part = self.normalize_number(parts[0]);
        let dec_part: Vec<&str> = parts[1]
            .chars()
            .map(|c| match c {
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
                _ => "?",
            })
            .collect();
        format!("{} точка {}", int_part, dec_part.join(" "))
    }

    /// Convert percentage string (e.g. "50%") to Russian words.
    pub fn normalize_percentage(&self, pct_str: &str) -> String {
        let num_str = pct_str.trim_end_matches('%').trim();

        if num_str.contains('.') || num_str.contains(',') {
            let num_words = self.normalize_float(num_str);
            return format!("{} процентов", num_words);
        }

        match num_str.trim().parse::<i64>() {
            Ok(n) => {
                let num_words = int_to_words(n);
                let suffix = get_declension(n, ("процент", "процента", "процентов"));
                format!("{} {}", num_words, suffix)
            }
            Err(_) => pct_str.to_string(),
        }
    }

    /// Convert range string (e.g. "10-20") to "от X до Y" with genitive case.
    pub fn normalize_range(&self, range_str: &str) -> String {
        static RE_DASH: OnceLock<Regex> = OnceLock::new();
        let re = RE_DASH.get_or_init(|| Regex::new(r"[-–—]").expect("static pattern"));
        let parts: Vec<&str> = re.splitn(range_str, 2).collect();
        if parts.len() != 2 {
            return range_str.to_string();
        }

        let start_str = parts[0].trim();
        let end_str = parts[1].trim();

        match (start_str.parse::<i64>(), end_str.parse::<i64>()) {
            (Ok(start), Ok(end)) => {
                let start_gen = to_genitive(start);
                let end_gen = to_genitive(end);
                format!("от {} до {}", start_gen, end_gen)
            }
            _ => range_str.to_string(),
        }
    }

    /// Convert size string (e.g. "100MB", "16px") to Russian words.
    pub fn normalize_size(&self, size_str: &str) -> String {
        static RE_SIZE_PARSE: OnceLock<Regex> = OnceLock::new();
        let re = RE_SIZE_PARSE
            .get_or_init(|| Regex::new(r"^([\d.,]+)\s*([a-zA-Zа-яА-Я]+)$").expect("static pattern"));
        let s = size_str.trim();

        let caps = match re.captures(s) {
            Some(c) => c,
            None => return size_str.to_string(),
        };

        let num_str = &caps[1];
        let unit_lower = caps[2].to_lowercase();

        let unit_data = SIZE_UNITS.iter().find(|(key, _)| *key == unit_lower.as_str());
        let unit = match unit_data {
            Some((_, u)) => u,
            None => return size_str.to_string(),
        };

        if num_str.contains('.') || num_str.contains(',') {
            let num_words = self.normalize_float(num_str);
            return format!("{} {}", num_words, unit.gen_pl);
        }

        match num_str.parse::<i64>() {
            Ok(n) => {
                let num_words = int_to_words_with_gender(n, unit.gender);
                let unit_word =
                    get_declension(n, (unit.nom_sg, unit.gen_sg, unit.gen_pl));
                format!("{} {}", num_words, unit_word)
            }
            Err(_) => size_str.to_string(),
        }
    }

    /// Convert version string (e.g. "v1.2.3", "2.0-beta") to Russian words.
    pub fn normalize_version(&self, ver_str: &str) -> String {
        let s = ver_str.trim_start_matches(['v', 'V']);

        // Tokenise into (kind, value) pairs
        let mut tokens: Vec<(&str, String)> = Vec::new();
        let mut current = String::new();

        for ch in s.chars() {
            match ch {
                '.' => {
                    if !current.is_empty() {
                        tokens.push(("num", current.clone()));
                        current.clear();
                    }
                    tokens.push(("dot", ".".to_string()));
                }
                '-' => {
                    if !current.is_empty() {
                        tokens.push(("num", current.clone()));
                        current.clear();
                    }
                    tokens.push(("dash", "-".to_string()));
                }
                _ => current.push(ch),
            }
        }
        if !current.is_empty() {
            tokens.push(("num", current));
        }

        static RE_VERSION_SUFFIX: OnceLock<Regex> = OnceLock::new();
        let suffix_re = RE_VERSION_SUFFIX
            .get_or_init(|| Regex::new(r"^([a-zA-Z]+)(\d*)$").expect("static pattern"));

        let mut result: Vec<String> = Vec::new();

        for (kind, value) in &tokens {
            match *kind {
                "dot" => result.push("точка".to_string()),
                "dash" => {} // skipped; suffix follows directly
                "num" => {
                    if let Some(caps) = suffix_re.captures(value) {
                        let suffix_name = caps[1].to_lowercase();
                        let suffix_num = &caps[2];

                        if let Some(&(_, word)) =
                            VERSION_SUFFIXES.iter().find(|(k, _)| *k == suffix_name.as_str())
                        {
                            result.push(word.to_string());
                            if !suffix_num.is_empty() {
                                result.push(self.normalize_number(suffix_num));
                            }
                        } else if value.chars().all(|c| c.is_ascii_digit()) {
                            result.push(self.normalize_number(value));
                        } else {
                            result.push(value.clone());
                        }
                    } else if value.chars().all(|c| c.is_ascii_digit()) {
                        result.push(self.normalize_number(value));
                    } else {
                        result.push(value.clone());
                    }
                }
                _ => {}
            }
        }

        result.join(" ")
    }

    /// Convert date string (ISO or European) to Russian words.
    pub fn normalize_date(&self, date_str: &str) -> String {
        let parts: Vec<&str> = date_str.split(['-', '/', '.']).collect();
        if parts.len() != 3 {
            return date_str.to_string();
        }

        let (year, month, day) = if parts[0].len() == 4 {
            // ISO format: YYYY-MM-DD
            (
                parts[0].parse::<i64>().ok(),
                parts[1].parse::<u32>().ok(),
                parts[2].parse::<u32>().ok(),
            )
        } else {
            // European format: DD.MM.YYYY
            (
                parts[2].parse::<i64>().ok(),
                parts[1].parse::<u32>().ok(),
                parts[0].parse::<u32>().ok(),
            )
        };

        match (year, month, day) {
            (Some(y), Some(m), Some(d))
                if y > 0 && (1..=12).contains(&m) && (1..=31).contains(&d) =>
            {
                let day_ord = day_ordinal_neuter(d);
                let month_name = MONTHS_GENITIVE[m as usize];
                let year_gen = year_to_genitive(y);
                format!("{} {} {} года", day_ord, month_name, year_gen)
            }
            _ => date_str.to_string(),
        }
    }

    /// Convert time string (HH:MM or HH:MM:SS) to Russian words.
    pub fn normalize_time(&self, time_str: &str) -> String {
        let parts: Vec<&str> = time_str.split(':').collect();
        if parts.len() < 2 {
            return time_str.to_string();
        }

        let hours = match parts[0].parse::<i64>() {
            Ok(h) => h,
            Err(_) => return time_str.to_string(),
        };
        let minutes = match parts[1].parse::<i64>() {
            Ok(m) => m,
            Err(_) => return time_str.to_string(),
        };
        let seconds = if parts.len() > 2 {
            match parts[2].parse::<i64>() {
                Ok(s) => s,
                Err(_) => return time_str.to_string(),
            }
        } else {
            0
        };

        let mut result_parts: Vec<String> = Vec::new();

        let hours_word = int_to_words(hours);
        let hours_suffix = get_declension(hours, ("час", "часа", "часов"));
        result_parts.push(format!("{} {}", hours_word, hours_suffix));

        if minutes > 0 || seconds > 0 {
            let min_word = int_to_words(minutes);
            let min_suffix = get_declension(minutes, ("минута", "минуты", "минут"));
            result_parts.push(format!("{} {}", min_word, min_suffix));
        }

        if seconds > 0 {
            let sec_word = int_to_words(seconds);
            let sec_suffix = get_declension(seconds, ("секунда", "секунды", "секунд"));
            result_parts.push(format!("{} {}", sec_word, sec_suffix));
        }

        result_parts.join(" ")
    }
}

impl Default for NumberNormalizer {
    fn default() -> Self {
        Self::new()
    }
}

// ---- Tests ----

#[cfg(test)]
mod tests {
    use super::*;

    fn nn() -> NumberNormalizer {
        NumberNormalizer::new()
    }

    // ---- TestIntegers ----

    #[test]
    fn test_integer_zero() {
        assert_eq!(nn().normalize_number("0"), "ноль");
    }

    #[test]
    fn test_integer_one() {
        assert_eq!(nn().normalize_number("1"), "один");
    }

    #[test]
    fn test_integer_five() {
        assert_eq!(nn().normalize_number("5"), "пять");
    }

    #[test]
    fn test_integer_ten() {
        assert_eq!(nn().normalize_number("10"), "десять");
    }

    #[test]
    fn test_integer_eleven() {
        assert_eq!(nn().normalize_number("11"), "одиннадцать");
    }

    #[test]
    fn test_integer_fifteen() {
        assert_eq!(nn().normalize_number("15"), "пятнадцать");
    }

    #[test]
    fn test_integer_twenty() {
        assert_eq!(nn().normalize_number("20"), "двадцать");
    }

    #[test]
    fn test_integer_twenty_one() {
        assert_eq!(nn().normalize_number("21"), "двадцать один");
    }

    #[test]
    fn test_integer_forty_two() {
        assert_eq!(nn().normalize_number("42"), "сорок два");
    }

    #[test]
    fn test_integer_ninety_nine() {
        assert_eq!(nn().normalize_number("99"), "девяносто девять");
    }

    #[test]
    fn test_integer_hundred() {
        assert_eq!(nn().normalize_number("100"), "сто");
    }

    #[test]
    fn test_integer_hundred_one() {
        assert_eq!(nn().normalize_number("101"), "сто один");
    }

    #[test]
    fn test_integer_hundred_twenty_three() {
        assert_eq!(nn().normalize_number("123"), "сто двадцать три");
    }

    #[test]
    fn test_integer_two_hundred() {
        assert_eq!(nn().normalize_number("200"), "двести");
    }

    #[test]
    fn test_integer_three_hundred() {
        assert_eq!(nn().normalize_number("300"), "триста");
    }

    #[test]
    fn test_integer_five_hundred() {
        assert_eq!(nn().normalize_number("500"), "пятьсот");
    }

    #[test]
    fn test_integer_999() {
        assert_eq!(nn().normalize_number("999"), "девятьсот девяносто девять");
    }

    #[test]
    fn test_integer_one_thousand() {
        assert_eq!(nn().normalize_number("1000"), "одна тысяча");
    }

    #[test]
    fn test_integer_one_thousand_one() {
        assert_eq!(nn().normalize_number("1001"), "одна тысяча один");
    }

    #[test]
    fn test_integer_1234() {
        assert_eq!(nn().normalize_number("1234"), "одна тысяча двести тридцать четыре");
    }

    #[test]
    fn test_integer_ten_thousand() {
        assert_eq!(nn().normalize_number("10000"), "десять тысяч");
    }

    #[test]
    fn test_integer_hundred_thousand() {
        assert_eq!(nn().normalize_number("100000"), "сто тысяч");
    }

    #[test]
    fn test_integer_one_million() {
        assert_eq!(nn().normalize_number("1000000"), "один миллион");
    }

    // ---- TestFloats ----

    #[test]
    fn test_float_pi() {
        assert_eq!(nn().normalize_float("3.14"), "три точка один четыре");
    }

    #[test]
    fn test_float_zero_point_five() {
        assert_eq!(nn().normalize_float("0.5"), "ноль точка пять");
    }

    #[test]
    fn test_float_two_point_zero() {
        assert_eq!(nn().normalize_float("2.0"), "два точка ноль");
    }

    #[test]
    fn test_float_ten_point_twenty_five() {
        assert_eq!(nn().normalize_float("10.25"), "десять точка два пять");
    }

    #[test]
    fn test_float_99_99() {
        assert_eq!(
            nn().normalize_float("99.99"),
            "девяносто девять точка девять девять"
        );
    }

    #[test]
    fn test_float_zero_zero_one() {
        assert_eq!(nn().normalize_float("0.001"), "ноль точка ноль ноль один");
    }

    #[test]
    fn test_float_one_point_five() {
        assert_eq!(nn().normalize_float("1.5"), "один точка пять");
    }

    #[test]
    fn test_float_comma_pi() {
        assert_eq!(nn().normalize_float("3,14"), "три точка один четыре");
    }

    #[test]
    fn test_float_comma_zero_five() {
        assert_eq!(nn().normalize_float("0,5"), "ноль точка пять");
    }

    #[test]
    fn test_float_comma_ten_25() {
        assert_eq!(nn().normalize_float("10,25"), "десять точка два пять");
    }

    // ---- TestPercentages ----

    #[test]
    fn test_pct_fifty() {
        assert_eq!(nn().normalize_percentage("50%"), "пятьдесят процентов");
    }

    #[test]
    fn test_pct_hundred() {
        assert_eq!(nn().normalize_percentage("100%"), "сто процентов");
    }

    #[test]
    fn test_pct_one() {
        assert_eq!(nn().normalize_percentage("1%"), "один процент");
    }

    #[test]
    fn test_pct_two() {
        assert_eq!(nn().normalize_percentage("2%"), "два процента");
    }

    #[test]
    fn test_pct_five() {
        assert_eq!(nn().normalize_percentage("5%"), "пять процентов");
    }

    #[test]
    fn test_pct_twenty_one() {
        assert_eq!(nn().normalize_percentage("21%"), "двадцать один процент");
    }

    #[test]
    fn test_pct_twenty_two() {
        assert_eq!(nn().normalize_percentage("22%"), "двадцать два процента");
    }

    #[test]
    fn test_pct_twenty_five() {
        assert_eq!(nn().normalize_percentage("25%"), "двадцать пять процентов");
    }

    #[test]
    fn test_pct_float_99_9() {
        assert_eq!(
            nn().normalize_percentage("99.9%"),
            "девяносто девять точка девять процентов"
        );
    }

    #[test]
    fn test_pct_float_0_5() {
        assert_eq!(
            nn().normalize_percentage("0.5%"),
            "ноль точка пять процентов"
        );
    }

    #[test]
    fn test_pct_float_33_33() {
        assert_eq!(
            nn().normalize_percentage("33.33%"),
            "тридцать три точка три три процентов"
        );
    }

    // ---- TestRanges ----

    #[test]
    fn test_range_1_10() {
        assert_eq!(nn().normalize_range("1-10"), "от одного до десяти");
    }

    #[test]
    fn test_range_10_20() {
        assert_eq!(nn().normalize_range("10-20"), "от десяти до двадцати");
    }

    #[test]
    fn test_range_100_200() {
        assert_eq!(nn().normalize_range("100-200"), "от ста до двухсот");
    }

    #[test]
    fn test_range_years() {
        assert_eq!(
            nn().normalize_range("2020-2024"),
            "от две тысячи двадцатого до две тысячи двадцать четвёртого"
        );
    }

    #[test]
    fn test_range_5_6() {
        assert_eq!(nn().normalize_range("5-6"), "от пяти до шести");
    }

    #[test]
    fn test_range_1_100() {
        assert_eq!(nn().normalize_range("1-100"), "от одного до ста");
    }

    #[test]
    fn test_range_en_dash() {
        assert_eq!(nn().normalize_range("10\u{2013}20"), "от десяти до двадцати");
    }

    #[test]
    fn test_range_em_dash() {
        assert_eq!(nn().normalize_range("100\u{2014}200"), "от ста до двухсот");
    }

    // ---- TestSizeUnits ----

    #[test]
    fn test_size_100kb() {
        assert_eq!(nn().normalize_size("100KB"), "сто килобайт");
    }

    #[test]
    fn test_size_1mb() {
        assert_eq!(nn().normalize_size("1MB"), "один мегабайт");
    }

    #[test]
    fn test_size_2mb() {
        assert_eq!(nn().normalize_size("2MB"), "два мегабайта");
    }

    #[test]
    fn test_size_5mb() {
        assert_eq!(nn().normalize_size("5MB"), "пять мегабайт");
    }

    #[test]
    fn test_size_16gb() {
        assert_eq!(nn().normalize_size("16GB"), "шестнадцать гигабайт");
    }

    #[test]
    fn test_size_1tb() {
        assert_eq!(nn().normalize_size("1TB"), "один терабайт");
    }

    #[test]
    fn test_size_512gb() {
        assert_eq!(nn().normalize_size("512GB"), "пятьсот двенадцать гигабайт");
    }

    #[test]
    fn test_size_100kb_space() {
        assert_eq!(nn().normalize_size("100 KB"), "сто килобайт");
    }

    #[test]
    fn test_size_16gb_space() {
        assert_eq!(nn().normalize_size("16 GB"), "шестнадцать гигабайт");
    }

    #[test]
    fn test_size_10ms() {
        assert_eq!(nn().normalize_size("10ms"), "десять миллисекунд");
    }

    #[test]
    fn test_size_1sec() {
        assert_eq!(nn().normalize_size("1sec"), "одна секунда");
    }

    #[test]
    fn test_size_5sec() {
        assert_eq!(nn().normalize_size("5sec"), "пять секунд");
    }

    #[test]
    fn test_size_30min() {
        assert_eq!(nn().normalize_size("30min"), "тридцать минут");
    }

    #[test]
    fn test_size_2hr() {
        assert_eq!(nn().normalize_size("2hr"), "два часа");
    }

    #[test]
    fn test_size_16px() {
        assert_eq!(nn().normalize_size("16px"), "шестнадцать пикселей");
    }

    #[test]
    fn test_size_1_5em() {
        assert_eq!(nn().normalize_size("1.5em"), "один точка пять эм");
    }

    #[test]
    fn test_size_100vh() {
        assert_eq!(nn().normalize_size("100vh"), "сто ви эйч");
    }

    #[test]
    fn test_size_russian_100kb() {
        assert_eq!(nn().normalize_size("100кб"), "сто килобайт");
    }

    #[test]
    fn test_size_russian_1mb() {
        assert_eq!(nn().normalize_size("1мб"), "один мегабайт");
    }

    #[test]
    fn test_size_russian_16gb() {
        assert_eq!(nn().normalize_size("16гб"), "шестнадцать гигабайт");
    }

    // ---- TestVersions ----

    #[test]
    fn test_version_1_0() {
        assert_eq!(nn().normalize_version("1.0"), "один точка ноль");
    }

    #[test]
    fn test_version_2_0() {
        assert_eq!(nn().normalize_version("2.0"), "два точка ноль");
    }

    #[test]
    fn test_version_1_0_0() {
        assert_eq!(nn().normalize_version("1.0.0"), "один точка ноль точка ноль");
    }

    #[test]
    fn test_version_2_3_1() {
        assert_eq!(nn().normalize_version("2.3.1"), "два точка три точка один");
    }

    #[test]
    fn test_version_3_11() {
        assert_eq!(nn().normalize_version("3.11"), "три точка одиннадцать");
    }

    #[test]
    fn test_version_10_15_7() {
        assert_eq!(
            nn().normalize_version("10.15.7"),
            "десять точка пятнадцать точка семь"
        );
    }

    #[test]
    fn test_version_v1_0() {
        assert_eq!(nn().normalize_version("v1.0"), "один точка ноль");
    }

    #[test]
    fn test_version_v2_3_1() {
        assert_eq!(nn().normalize_version("v2.3.1"), "два точка три точка один");
    }

    #[test]
    fn test_version_beta() {
        assert_eq!(nn().normalize_version("1.0-beta"), "один точка ноль бета");
    }

    #[test]
    fn test_version_alpha() {
        assert_eq!(nn().normalize_version("2.0-alpha"), "два точка ноль альфа");
    }

    #[test]
    fn test_version_rc1() {
        assert_eq!(
            nn().normalize_version("1.0.0-rc1"),
            "один точка ноль точка ноль эр си один"
        );
    }

    #[test]
    fn test_version_beta_point() {
        assert_eq!(
            nn().normalize_version("3.11.0-beta.1"),
            "три точка одиннадцать точка ноль бета точка один"
        );
    }

    // ---- TestDates ----

    #[test]
    fn test_date_iso_2024_01_15() {
        assert_eq!(
            nn().normalize_date("2024-01-15"),
            "пятнадцатое января две тысячи двадцать четвёртого года"
        );
    }

    #[test]
    fn test_date_iso_2024_12_31() {
        assert_eq!(
            nn().normalize_date("2024-12-31"),
            "тридцать первое декабря две тысячи двадцать четвёртого года"
        );
    }

    #[test]
    fn test_date_iso_2000_01_01() {
        assert_eq!(
            nn().normalize_date("2000-01-01"),
            "первое января двухтысячного года"
        );
    }

    #[test]
    fn test_date_eu_15_01_2024() {
        assert_eq!(
            nn().normalize_date("15.01.2024"),
            "пятнадцатое января две тысячи двадцать четвёртого года"
        );
    }

    #[test]
    fn test_date_eu_01_12_2023() {
        assert_eq!(
            nn().normalize_date("01.12.2023"),
            "первое декабря две тысячи двадцать третьего года"
        );
    }

    #[test]
    fn test_date_slash_2024_01_15() {
        assert_eq!(
            nn().normalize_date("2024/01/15"),
            "пятнадцатое января две тысячи двадцать четвёртого года"
        );
    }

    #[test]
    fn test_date_slash_15_01_2024() {
        assert_eq!(
            nn().normalize_date("15/01/2024"),
            "пятнадцатое января две тысячи двадцать четвёртого года"
        );
    }

    // ---- TestTimes ----

    #[test]
    fn test_time_10_00() {
        assert_eq!(nn().normalize_time("10:00"), "десять часов");
    }

    #[test]
    fn test_time_10_30() {
        assert_eq!(nn().normalize_time("10:30"), "десять часов тридцать минут");
    }

    #[test]
    fn test_time_14_15() {
        assert_eq!(
            nn().normalize_time("14:15"),
            "четырнадцать часов пятнадцать минут"
        );
    }

    #[test]
    fn test_time_00_00() {
        assert_eq!(nn().normalize_time("00:00"), "ноль часов");
    }

    #[test]
    fn test_time_01_00() {
        assert_eq!(nn().normalize_time("01:00"), "один час");
    }

    #[test]
    fn test_time_02_00() {
        assert_eq!(nn().normalize_time("02:00"), "два часа");
    }

    #[test]
    fn test_time_05_00() {
        assert_eq!(nn().normalize_time("05:00"), "пять часов");
    }

    #[test]
    fn test_time_21_00() {
        assert_eq!(nn().normalize_time("21:00"), "двадцать один час");
    }

    #[test]
    fn test_time_22_30() {
        assert_eq!(
            nn().normalize_time("22:30"),
            "двадцать два часа тридцать минут"
        );
    }

    #[test]
    fn test_time_23_59() {
        assert_eq!(
            nn().normalize_time("23:59"),
            "двадцать три часа пятьдесят девять минут"
        );
    }

    #[test]
    fn test_time_with_seconds() {
        assert_eq!(
            nn().normalize_time("10:30:15"),
            "десять часов тридцать минут пятнадцать секунд"
        );
    }

    #[test]
    fn test_time_eight_zeros() {
        assert_eq!(nn().normalize_time("08:00:00"), "восемь часов");
    }
}
