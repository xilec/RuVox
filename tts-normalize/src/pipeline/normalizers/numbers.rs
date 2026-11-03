//! Numbers normalizer — converts integers to Russian words.
//!
//! Number-to-words logic is a manual port of the Python `num2words(n, lang="ru")`
//! output, since no Rust crate provides equivalent Russian-language support.

#[derive(Clone, Copy, PartialEq, Eq)]
enum Gender {
    Masculine,
    Feminine,
}

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
    use test_case::test_case;

    fn normalizer() -> NumberNormalizer {
        NumberNormalizer::new()
    }

    // ---- TestIntegers ----

    #[test_case("0" => "ноль"; "zero")]
    #[test_case("1" => "один"; "one")]
    #[test_case("5" => "пять"; "five")]
    #[test_case("10" => "десять"; "ten")]
    #[test_case("11" => "одиннадцать"; "eleven")]
    #[test_case("15" => "пятнадцать"; "fifteen")]
    #[test_case("20" => "двадцать"; "twenty")]
    #[test_case("21" => "двадцать один"; "twenty_one")]
    #[test_case("42" => "сорок два"; "forty_two")]
    #[test_case("99" => "девяносто девять"; "ninety_nine")]
    #[test_case("100" => "сто"; "hundred")]
    #[test_case("101" => "сто один"; "hundred_one")]
    #[test_case("123" => "сто двадцать три"; "hundred_twenty_three")]
    #[test_case("200" => "двести"; "two_hundred")]
    #[test_case("300" => "триста"; "three_hundred")]
    #[test_case("500" => "пятьсот"; "five_hundred")]
    #[test_case("999" => "девятьсот девяносто девять"; "nine_hundred_ninety_nine")]
    #[test_case("1000" => "одна тысяча"; "one_thousand")]
    #[test_case("1001" => "одна тысяча один"; "one_thousand_one")]
    #[test_case("1234" => "одна тысяча двести тридцать четыре"; "one_thousand_two_hundred_thirty_four")]
    #[test_case("10000" => "десять тысяч"; "ten_thousand")]
    #[test_case("100000" => "сто тысяч"; "hundred_thousand")]
    #[test_case("1000000" => "один миллион"; "one_million")]
    fn integer(input: &str) -> String {
        normalizer().normalize_number(input)
    }
}
