use std::collections::HashMap;

use once_cell::sync::Lazy;

use crate::pipeline::constants::{ARROW_SYMBOLS, GREEK_LETTERS, MATH_SYMBOLS};

/// Combined symbol lookup table: operators + Greek letters + math + arrow symbols.
/// Longer patterns are listed first so multi-char operators take precedence.
/// At runtime the HashMap is used for O(1) lookup.
static SYMBOLS: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut m: HashMap<&'static str, &'static str> = HashMap::new();

    // Multi-character operators (must be looked up before single-char)
    m.insert("<->", "двунаправленная стрелка");
    m.insert("===", "строго равно");
    m.insert("!==", "строго не равно");
    m.insert("...", "троеточие");
    m.insert("::", "двойное двоеточие");
    m.insert("->", "стрелка");
    m.insert("=>", "толстая стрелка");
    m.insert("<-", "стрелка влево");
    m.insert(">=", "больше или равно");
    m.insert("<=", "меньше или равно");
    m.insert("!=", "не равно");
    m.insert("==", "равно равно");
    m.insert("&&", "и");
    m.insert("||", "или");
    m.insert("??", "нулевое слияние");
    m.insert("?.", "опциональная цепочка");
    m.insert("**", "степень");
    m.insert("//", "целочисленное деление");
    m.insert("++", "плюс плюс");
    m.insert("--", "минус минус");
    m.insert("+=", "плюс равно");
    m.insert("-=", "минус равно");
    m.insert("*=", "умножить равно");
    m.insert("/=", "делить равно");
    m.insert(":=", "присваивание");
    m.insert("<<", "сдвиг влево");
    m.insert(">>", "сдвиг вправо");

    // Single-character comparison / arithmetic
    m.insert("<", "меньше");
    m.insert(">", "больше");
    m.insert("=", "равно");
    m.insert("+", "плюс");
    m.insert("-", "минус");
    m.insert("*", "умножить");
    m.insert("/", "делить");
    m.insert("%", "процент");
    m.insert("!", "восклицательный знак");
    m.insert("?", "вопросительный знак");

    // Bitwise
    m.insert("&", "амперсанд");
    m.insert("|", "пайп");
    m.insert("^", "каретка");
    m.insert("~", "тильда");

    // Punctuation
    m.insert(".", "точка");
    m.insert(",", "запятая");
    m.insert(":", "двоеточие");
    m.insert(";", "точка с запятой");
    m.insert("_", "нижнее подчёркивание");
    m.insert("\\", "бэкслэш");

    // Brackets
    m.insert("(", "открывающая скобка");
    m.insert(")", "закрывающая скобка");
    m.insert("[", "открывающая квадратная скобка");
    m.insert("]", "закрывающая квадратная скобка");
    m.insert("{", "открывающая фигурная скобка");
    m.insert("}", "закрывающая фигурная скобка");

    // Special characters
    m.insert("@", "собака");
    m.insert("#", "решётка");
    m.insert("$", "доллар");

    // Quotes
    m.insert("\"", "кавычка");
    m.insert("'", "апостроф");
    m.insert("`", "обратная кавычка");
    m.insert("«", "открывающая кавычка");
    m.insert("»", "закрывающая кавычка");

    // Unicode symbols
    m.insert("©", "копирайт");
    m.insert("®", "зарегистрировано");
    m.insert("™", "торговая марка");
    m.insert("°", "градус");
    m.insert("±", "плюс минус");

    // Greek letters
    for (k, v) in GREEK_LETTERS.iter() {
        m.insert(k, v);
    }

    // Math symbols
    for (k, v) in MATH_SYMBOLS.iter() {
        m.insert(k, v);
    }

    // Arrow symbols (Unicode arrows; ASCII arrows already covered above)
    for (k, v) in ARROW_SYMBOLS.iter() {
        m.insert(k, v);
    }

    m
});

pub struct SymbolNormalizer;

impl SymbolNormalizer {
    pub fn new() -> Self {
        Self
    }

    /// Convert a symbol / operator string to its speakable Russian equivalent.
    /// Returns the original string unchanged if the symbol is not in the table.
    pub fn normalize<'a>(&self, symbol: &'a str) -> &'a str {
        SYMBOLS.get(symbol).copied().unwrap_or(symbol)
    }
}

impl Default for SymbolNormalizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn normalizer() -> SymbolNormalizer {
        SymbolNormalizer::new()
    }

    // --- Arrow operators ---

    #[test]
    fn test_arrow_right() {
        assert_eq!(normalizer().normalize("->"), "стрелка");
    }

    #[test]
    fn test_arrow_fat() {
        assert_eq!(normalizer().normalize("=>"), "толстая стрелка");
    }

    #[test]
    fn test_arrow_left() {
        assert_eq!(normalizer().normalize("<-"), "стрелка влево");
    }

    #[test]
    fn test_arrow_bidirectional() {
        assert_eq!(normalizer().normalize("<->"), "двунаправленная стрелка");
    }

    // --- Comparison operators ---

    #[test]
    fn test_gte() {
        assert_eq!(normalizer().normalize(">="), "больше или равно");
    }

    #[test]
    fn test_lte() {
        assert_eq!(normalizer().normalize("<="), "меньше или равно");
    }

    #[test]
    fn test_neq() {
        assert_eq!(normalizer().normalize("!="), "не равно");
    }

    #[test]
    fn test_eqeq() {
        assert_eq!(normalizer().normalize("=="), "равно равно");
    }

    #[test]
    fn test_strict_eq() {
        assert_eq!(normalizer().normalize("==="), "строго равно");
    }

    #[test]
    fn test_strict_neq() {
        assert_eq!(normalizer().normalize("!=="), "строго не равно");
    }

    #[test]
    fn test_lt() {
        assert_eq!(normalizer().normalize("<"), "меньше");
    }

    #[test]
    fn test_gt() {
        assert_eq!(normalizer().normalize(">"), "больше");
    }

    #[test]
    fn test_eq() {
        assert_eq!(normalizer().normalize("="), "равно");
    }

    // --- Logical operators ---

    #[test]
    fn test_and() {
        assert_eq!(normalizer().normalize("&&"), "и");
    }

    #[test]
    fn test_or() {
        assert_eq!(normalizer().normalize("||"), "или");
    }

    #[test]
    fn test_bang() {
        assert_eq!(normalizer().normalize("!"), "восклицательный знак");
    }

    #[test]
    fn test_nullish_coalescing() {
        assert_eq!(normalizer().normalize("??"), "нулевое слияние");
    }

    #[test]
    fn test_optional_chain() {
        assert_eq!(normalizer().normalize("?."), "опциональная цепочка");
    }

    #[test]
    fn test_question_mark() {
        assert_eq!(normalizer().normalize("?"), "вопросительный знак");
    }

    // --- Arithmetic operators ---

    #[test]
    fn test_plus() {
        assert_eq!(normalizer().normalize("+"), "плюс");
    }

    #[test]
    fn test_minus() {
        assert_eq!(normalizer().normalize("-"), "минус");
    }

    #[test]
    fn test_mul() {
        assert_eq!(normalizer().normalize("*"), "умножить");
    }

    #[test]
    fn test_div() {
        assert_eq!(normalizer().normalize("/"), "делить");
    }

    #[test]
    fn test_pow() {
        assert_eq!(normalizer().normalize("**"), "степень");
    }

    #[test]
    fn test_floor_div() {
        assert_eq!(normalizer().normalize("//"), "целочисленное деление");
    }

    #[test]
    fn test_percent() {
        assert_eq!(normalizer().normalize("%"), "процент");
    }

    #[test]
    fn test_inc() {
        assert_eq!(normalizer().normalize("++"), "плюс плюс");
    }

    #[test]
    fn test_dec() {
        assert_eq!(normalizer().normalize("--"), "минус минус");
    }

    // --- Assignment operators ---

    #[test]
    fn test_plus_assign() {
        assert_eq!(normalizer().normalize("+="), "плюс равно");
    }

    #[test]
    fn test_minus_assign() {
        assert_eq!(normalizer().normalize("-="), "минус равно");
    }

    #[test]
    fn test_mul_assign() {
        assert_eq!(normalizer().normalize("*="), "умножить равно");
    }

    #[test]
    fn test_div_assign() {
        assert_eq!(normalizer().normalize("/="), "делить равно");
    }

    #[test]
    fn test_walrus() {
        assert_eq!(normalizer().normalize(":="), "присваивание");
    }

    // --- Bitwise operators ---

    #[test]
    fn test_bitwise_and() {
        assert_eq!(normalizer().normalize("&"), "амперсанд");
    }

    #[test]
    fn test_pipe() {
        assert_eq!(normalizer().normalize("|"), "пайп");
    }

    #[test]
    fn test_caret() {
        assert_eq!(normalizer().normalize("^"), "каретка");
    }

    #[test]
    fn test_tilde() {
        assert_eq!(normalizer().normalize("~"), "тильда");
    }

    #[test]
    fn test_shl() {
        assert_eq!(normalizer().normalize("<<"), "сдвиг влево");
    }

    #[test]
    fn test_shr() {
        assert_eq!(normalizer().normalize(">>"), "сдвиг вправо");
    }

    // --- Scope / access operators ---

    #[test]
    fn test_double_colon() {
        assert_eq!(normalizer().normalize("::"), "двойное двоеточие");
    }

    #[test]
    fn test_dot() {
        assert_eq!(normalizer().normalize("."), "точка");
    }

    #[test]
    fn test_comma() {
        assert_eq!(normalizer().normalize(","), "запятая");
    }

    #[test]
    fn test_colon() {
        assert_eq!(normalizer().normalize(":"), "двоеточие");
    }

    #[test]
    fn test_semicolon() {
        assert_eq!(normalizer().normalize(";"), "точка с запятой");
    }

    // --- Brackets ---

    #[test]
    fn test_paren_open() {
        assert_eq!(normalizer().normalize("("), "открывающая скобка");
    }

    #[test]
    fn test_paren_close() {
        assert_eq!(normalizer().normalize(")"), "закрывающая скобка");
    }

    #[test]
    fn test_bracket_open() {
        assert_eq!(normalizer().normalize("["), "открывающая квадратная скобка");
    }

    #[test]
    fn test_bracket_close() {
        assert_eq!(normalizer().normalize("]"), "закрывающая квадратная скобка");
    }

    #[test]
    fn test_brace_open() {
        assert_eq!(normalizer().normalize("{"), "открывающая фигурная скобка");
    }

    #[test]
    fn test_brace_close() {
        assert_eq!(normalizer().normalize("}"), "закрывающая фигурная скобка");
    }

    // --- Punctuation ---

    #[test]
    fn test_ellipsis() {
        assert_eq!(normalizer().normalize("..."), "троеточие");
    }

    #[test]
    fn test_underscore() {
        assert_eq!(normalizer().normalize("_"), "нижнее подчёркивание");
    }

    #[test]
    fn test_backslash() {
        assert_eq!(normalizer().normalize("\\"), "бэкслэш");
    }

    // --- Special characters ---

    #[test]
    fn test_at() {
        assert_eq!(normalizer().normalize("@"), "собака");
    }

    #[test]
    fn test_hash() {
        assert_eq!(normalizer().normalize("#"), "решётка");
    }

    #[test]
    fn test_dollar() {
        assert_eq!(normalizer().normalize("$"), "доллар");
    }

    // --- Quotes ---

    #[test]
    fn test_double_quote() {
        assert_eq!(normalizer().normalize("\""), "кавычка");
    }

    #[test]
    fn test_single_quote() {
        assert_eq!(normalizer().normalize("'"), "апостроф");
    }

    #[test]
    fn test_backtick() {
        assert_eq!(normalizer().normalize("`"), "обратная кавычка");
    }

    #[test]
    fn test_lquote() {
        assert_eq!(normalizer().normalize("«"), "открывающая кавычка");
    }

    #[test]
    fn test_rquote() {
        assert_eq!(normalizer().normalize("»"), "закрывающая кавычка");
    }

    // --- Unicode symbols ---

    #[test]
    fn test_copyright() {
        assert_eq!(normalizer().normalize("©"), "копирайт");
    }

    #[test]
    fn test_registered() {
        assert_eq!(normalizer().normalize("®"), "зарегистрировано");
    }

    #[test]
    fn test_trademark() {
        assert_eq!(normalizer().normalize("™"), "торговая марка");
    }

    #[test]
    fn test_degree() {
        assert_eq!(normalizer().normalize("°"), "градус");
    }

    #[test]
    fn test_plus_minus() {
        assert_eq!(normalizer().normalize("±"), "плюс минус");
    }

    // --- Greek letters (lowercase) ---

    #[test]
    fn test_alpha() {
        assert_eq!(normalizer().normalize("α"), "альфа");
    }

    #[test]
    fn test_beta() {
        assert_eq!(normalizer().normalize("β"), "бета");
    }

    #[test]
    fn test_gamma() {
        assert_eq!(normalizer().normalize("γ"), "гамма");
    }

    #[test]
    fn test_delta() {
        assert_eq!(normalizer().normalize("δ"), "дельта");
    }

    #[test]
    fn test_epsilon() {
        assert_eq!(normalizer().normalize("ε"), "эпсилон");
    }

    #[test]
    fn test_lambda() {
        assert_eq!(normalizer().normalize("λ"), "лямбда");
    }

    #[test]
    fn test_pi() {
        assert_eq!(normalizer().normalize("π"), "пи");
    }

    #[test]
    fn test_sigma() {
        assert_eq!(normalizer().normalize("σ"), "сигма");
    }

    #[test]
    fn test_tau() {
        assert_eq!(normalizer().normalize("τ"), "тау");
    }

    #[test]
    fn test_phi() {
        assert_eq!(normalizer().normalize("φ"), "фи");
    }

    #[test]
    fn test_omega() {
        assert_eq!(normalizer().normalize("ω"), "омега");
    }

    // --- Greek letters (uppercase) ---

    #[test]
    fn test_alpha_upper() {
        assert_eq!(normalizer().normalize("Α"), "альфа");
    }

    #[test]
    fn test_beta_upper() {
        assert_eq!(normalizer().normalize("Β"), "бета");
    }

    #[test]
    fn test_gamma_upper() {
        assert_eq!(normalizer().normalize("Γ"), "гамма");
    }

    #[test]
    fn test_delta_upper() {
        assert_eq!(normalizer().normalize("Δ"), "дельта");
    }

    #[test]
    fn test_lambda_upper() {
        assert_eq!(normalizer().normalize("Λ"), "лямбда");
    }

    #[test]
    fn test_pi_upper() {
        assert_eq!(normalizer().normalize("Π"), "пи");
    }

    #[test]
    fn test_sigma_upper() {
        assert_eq!(normalizer().normalize("Σ"), "сигма");
    }

    #[test]
    fn test_omega_upper() {
        assert_eq!(normalizer().normalize("Ω"), "омега");
    }

    // --- Math symbols ---

    #[test]
    fn test_infinity() {
        assert_eq!(normalizer().normalize("∞"), "бесконечность");
    }

    #[test]
    fn test_in() {
        assert_eq!(normalizer().normalize("∈"), "принадлежит");
    }

    #[test]
    fn test_not_in() {
        assert_eq!(normalizer().normalize("∉"), "не принадлежит");
    }

    #[test]
    fn test_for_all() {
        assert_eq!(normalizer().normalize("∀"), "для всех");
    }

    #[test]
    fn test_exists() {
        assert_eq!(normalizer().normalize("∃"), "существует");
    }

    #[test]
    fn test_empty_set() {
        assert_eq!(normalizer().normalize("∅"), "пустое множество");
    }

    #[test]
    fn test_intersection() {
        assert_eq!(normalizer().normalize("∩"), "пересечение");
    }

    #[test]
    fn test_union() {
        assert_eq!(normalizer().normalize("∪"), "объединение");
    }

    #[test]
    fn test_subset() {
        assert_eq!(normalizer().normalize("⊂"), "подмножество");
    }

    #[test]
    fn test_not_equal_unicode() {
        assert_eq!(normalizer().normalize("≠"), "не равно");
    }

    #[test]
    fn test_approx() {
        assert_eq!(normalizer().normalize("≈"), "приблизительно равно");
    }

    #[test]
    fn test_lte_unicode() {
        assert_eq!(normalizer().normalize("≤"), "меньше или равно");
    }

    #[test]
    fn test_gte_unicode() {
        assert_eq!(normalizer().normalize("≥"), "больше или равно");
    }

    #[test]
    fn test_times() {
        assert_eq!(normalizer().normalize("×"), "умножить");
    }

    #[test]
    fn test_divide_unicode() {
        assert_eq!(normalizer().normalize("÷"), "разделить");
    }

    #[test]
    fn test_sqrt() {
        assert_eq!(normalizer().normalize("√"), "корень");
    }

    #[test]
    fn test_sum() {
        assert_eq!(normalizer().normalize("∑"), "сумма");
    }

    #[test]
    fn test_product() {
        assert_eq!(normalizer().normalize("∏"), "произведение");
    }

    // --- Arrow symbols (Unicode) ---

    #[test]
    fn test_arrow_unicode_right() {
        assert_eq!(normalizer().normalize("→"), "стрелка");
    }

    #[test]
    fn test_arrow_unicode_left() {
        assert_eq!(normalizer().normalize("←"), "стрелка влево");
    }

    #[test]
    fn test_arrow_unicode_bidir() {
        assert_eq!(normalizer().normalize("↔"), "двунаправленная стрелка");
    }

    #[test]
    fn test_double_arrow_right() {
        assert_eq!(normalizer().normalize("⇒"), "следует");
    }

    #[test]
    fn test_double_arrow_left() {
        assert_eq!(normalizer().normalize("⇐"), "следует из");
    }

    #[test]
    fn test_double_arrow_bidir() {
        assert_eq!(normalizer().normalize("⇔"), "эквивалентно");
    }

    // --- Unknown symbol passthrough ---

    #[test]
    fn test_unknown_symbol_passthrough() {
        assert_eq!(normalizer().normalize("§"), "§");
    }

    #[test]
    fn test_unknown_word_passthrough() {
        assert_eq!(normalizer().normalize("hello"), "hello");
    }
}
