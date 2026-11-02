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
    use test_case::test_case;

    fn normalizer() -> SymbolNormalizer {
        SymbolNormalizer::new()
    }

    #[test_case("->" => "стрелка"; "right")]
    #[test_case("=>" => "толстая стрелка"; "fat")]
    #[test_case("<-" => "стрелка влево"; "left")]
    #[test_case("<->" => "двунаправленная стрелка"; "bidirectional")]
    fn arrow_operator(input: &str) -> &str {
        normalizer().normalize(input)
    }

    #[test_case(">=" => "больше или равно"; "gte")]
    #[test_case("<=" => "меньше или равно"; "lte")]
    #[test_case("!=" => "не равно"; "neq")]
    #[test_case("==" => "равно равно"; "eqeq")]
    #[test_case("===" => "строго равно"; "strict_eq")]
    #[test_case("!==" => "строго не равно"; "strict_neq")]
    #[test_case("<" => "меньше"; "lt")]
    #[test_case(">" => "больше"; "gt")]
    #[test_case("=" => "равно"; "eq")]
    fn comparison_operator(input: &str) -> &str {
        normalizer().normalize(input)
    }

    #[test_case("&&" => "и"; "and")]
    #[test_case("||" => "или"; "or")]
    #[test_case("!" => "восклицательный знак"; "bang")]
    #[test_case("??" => "нулевое слияние"; "nullish_coalescing")]
    #[test_case("?." => "опциональная цепочка"; "optional_chain")]
    #[test_case("?" => "вопросительный знак"; "question_mark")]
    fn logical_operator(input: &str) -> &str {
        normalizer().normalize(input)
    }

    #[test_case("+" => "плюс"; "plus")]
    #[test_case("-" => "минус"; "minus")]
    #[test_case("*" => "умножить"; "mul")]
    #[test_case("/" => "делить"; "div")]
    #[test_case("**" => "степень"; "pow")]
    #[test_case("//" => "целочисленное деление"; "floor_div")]
    #[test_case("%" => "процент"; "percent")]
    #[test_case("++" => "плюс плюс"; "inc")]
    #[test_case("--" => "минус минус"; "dec")]
    fn arithmetic_operator(input: &str) -> &str {
        normalizer().normalize(input)
    }

    #[test_case("+=" => "плюс равно"; "plus_assign")]
    #[test_case("-=" => "минус равно"; "minus_assign")]
    #[test_case("*=" => "умножить равно"; "mul_assign")]
    #[test_case("/=" => "делить равно"; "div_assign")]
    #[test_case(":=" => "присваивание"; "walrus")]
    fn assignment_operator(input: &str) -> &str {
        normalizer().normalize(input)
    }

    #[test_case("&" => "амперсанд"; "bitwise_and")]
    #[test_case("|" => "пайп"; "pipe")]
    #[test_case("^" => "каретка"; "caret")]
    #[test_case("~" => "тильда"; "tilde")]
    #[test_case("<<" => "сдвиг влево"; "shl")]
    #[test_case(">>" => "сдвиг вправо"; "shr")]
    fn bitwise_operator(input: &str) -> &str {
        normalizer().normalize(input)
    }

    #[test_case("::" => "двойное двоеточие"; "double_colon")]
    #[test_case("." => "точка"; "dot")]
    #[test_case("," => "запятая"; "comma")]
    #[test_case(":" => "двоеточие"; "colon")]
    #[test_case(";" => "точка с запятой"; "semicolon")]
    fn scope_operator(input: &str) -> &str {
        normalizer().normalize(input)
    }

    #[test_case("(" => "открывающая скобка"; "paren_open")]
    #[test_case(")" => "закрывающая скобка"; "paren_close")]
    #[test_case("[" => "открывающая квадратная скобка"; "bracket_open")]
    #[test_case("]" => "закрывающая квадратная скобка"; "bracket_close")]
    #[test_case("{" => "открывающая фигурная скобка"; "brace_open")]
    #[test_case("}" => "закрывающая фигурная скобка"; "brace_close")]
    fn bracket(input: &str) -> &str {
        normalizer().normalize(input)
    }

    #[test_case("..." => "троеточие"; "ellipsis")]
    #[test_case("_" => "нижнее подчёркивание"; "underscore")]
    #[test_case("\\" => "бэкслэш"; "backslash")]
    fn punctuation(input: &str) -> &str {
        normalizer().normalize(input)
    }

    #[test_case("@" => "собака"; "at")]
    #[test_case("#" => "решётка"; "hash")]
    #[test_case("$" => "доллар"; "dollar")]
    fn special_character(input: &str) -> &str {
        normalizer().normalize(input)
    }

    #[test_case("\"" => "кавычка"; "double_quote")]
    #[test_case("'" => "апостроф"; "single_quote")]
    #[test_case("`" => "обратная кавычка"; "backtick")]
    #[test_case("«" => "открывающая кавычка"; "lquote")]
    #[test_case("»" => "закрывающая кавычка"; "rquote")]
    fn quote(input: &str) -> &str {
        normalizer().normalize(input)
    }

    #[test_case("©" => "копирайт"; "copyright")]
    #[test_case("®" => "зарегистрировано"; "registered")]
    #[test_case("™" => "торговая марка"; "trademark")]
    #[test_case("°" => "градус"; "degree")]
    #[test_case("±" => "плюс минус"; "plus_minus")]
    fn unicode_symbol(input: &str) -> &str {
        normalizer().normalize(input)
    }

    #[test_case("α" => "альфа"; "alpha")]
    #[test_case("β" => "бета"; "beta")]
    #[test_case("γ" => "гамма"; "gamma")]
    #[test_case("δ" => "дельта"; "delta")]
    #[test_case("ε" => "эпсилон"; "epsilon")]
    #[test_case("λ" => "лямбда"; "lambda")]
    #[test_case("π" => "пи"; "pi")]
    #[test_case("σ" => "сигма"; "sigma")]
    #[test_case("τ" => "тау"; "tau")]
    #[test_case("φ" => "фи"; "phi")]
    #[test_case("ω" => "омега"; "omega")]
    fn greek_letter_lower(input: &str) -> &str {
        normalizer().normalize(input)
    }

    #[test_case("Α" => "альфа"; "alpha_upper")]
    #[test_case("Β" => "бета"; "beta_upper")]
    #[test_case("Γ" => "гамма"; "gamma_upper")]
    #[test_case("Δ" => "дельта"; "delta_upper")]
    #[test_case("Λ" => "лямбда"; "lambda_upper")]
    #[test_case("Π" => "пи"; "pi_upper")]
    #[test_case("Σ" => "сигма"; "sigma_upper")]
    #[test_case("Ω" => "омега"; "omega_upper")]
    fn greek_letter_upper(input: &str) -> &str {
        normalizer().normalize(input)
    }

    #[test_case("∞" => "бесконечность"; "infinity")]
    #[test_case("∈" => "принадлежит"; "in_set")]
    #[test_case("∉" => "не принадлежит"; "not_in")]
    #[test_case("∀" => "для всех"; "for_all")]
    #[test_case("∃" => "существует"; "exists")]
    #[test_case("∅" => "пустое множество"; "empty_set")]
    #[test_case("∩" => "пересечение"; "intersection")]
    #[test_case("∪" => "объединение"; "union")]
    #[test_case("⊂" => "подмножество"; "subset")]
    #[test_case("≠" => "не равно"; "not_equal_unicode")]
    #[test_case("≈" => "приблизительно равно"; "approx")]
    #[test_case("≤" => "меньше или равно"; "lte_unicode")]
    #[test_case("≥" => "больше или равно"; "gte_unicode")]
    #[test_case("×" => "умножить"; "times")]
    #[test_case("÷" => "разделить"; "divide_unicode")]
    #[test_case("√" => "корень"; "sqrt")]
    #[test_case("∑" => "сумма"; "sum")]
    #[test_case("∏" => "произведение"; "product")]
    fn math_symbol(input: &str) -> &str {
        normalizer().normalize(input)
    }

    #[test_case("→" => "стрелка"; "arrow_unicode_right")]
    #[test_case("←" => "стрелка влево"; "arrow_unicode_left")]
    #[test_case("↔" => "двунаправленная стрелка"; "arrow_unicode_bidir")]
    #[test_case("⇒" => "следует"; "double_arrow_right")]
    #[test_case("⇐" => "следует из"; "double_arrow_left")]
    #[test_case("⇔" => "эквивалентно"; "double_arrow_bidir")]
    fn arrow_symbol(input: &str) -> &str {
        normalizer().normalize(input)
    }

    #[test_case("§" => "§"; "unknown_symbol_passthrough")]
    #[test_case("hello" => "hello"; "unknown_word_passthrough")]
    fn passthrough(input: &str) -> &str {
        normalizer().normalize(input)
    }
}
