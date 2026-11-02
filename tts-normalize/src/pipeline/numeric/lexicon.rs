//! Word tables for the numeric span layer.
//!
//! Units, currencies, scale words, month names and the noun genders the reader
//! needs are lexical facts, not numeral morphology; keeping them here lets the
//! scanner and the reader stay about structure.

use crate::pipeline::numeric::morphology::Gender;

// ── Units ─────────────────────────────────────────────────────────────────────

/// One entry of the unit table.
#[derive(Clone, Copy, Debug)]
pub struct UnitForms {
    pub sg_nom: &'static str,
    pub sg_gen: &'static str,
    pub pl_gen: &'static str,
    pub gender: Gender,
    /// A symbol read as a fixed word ("em" → "эм") never declines.
    pub fixed: bool,
    /// Spoken tail the unit adds after its own form ("в час", "Цельсия").
    pub tail: &'static str,
}

const fn unit(
    sg_nom: &'static str,
    sg_gen: &'static str,
    pl_gen: &'static str,
    gender: Gender,
) -> UnitForms {
    UnitForms {
        sg_nom,
        sg_gen,
        pl_gen,
        gender,
        fixed: false,
        tail: "",
    }
}

const fn fixed_unit(word: &'static str, gender: Gender) -> UnitForms {
    UnitForms {
        sg_nom: word,
        sg_gen: word,
        pl_gen: word,
        gender,
        fixed: true,
        tail: "",
    }
}

/// A unit the reader can attach to a number.
#[derive(Clone, Copy, Debug)]
pub struct Unit {
    pub forms: UnitForms,
    /// Size units are the technical register: a decimal with a size unit keeps
    /// the "точка" reading ("1.5em" → "один точка пять эм"), unlike a physical
    /// unit, where a decimal closes into a fraction ("1,5 кг" → "полтора
    /// килограмма"). The scanner also uses this to decide whether a dot
    /// decimal owns its unit at all.
    pub size_unit: bool,
}

const fn natural(forms: UnitForms) -> Unit {
    Unit {
        forms,
        size_unit: false,
    }
}

const fn size(forms: UnitForms) -> Unit {
    Unit {
        forms,
        size_unit: true,
    }
}

const KILOGRAM: UnitForms = unit("килограмм", "килограмма", "килограммов", Gender::Masculine);
const KILOMETER: UnitForms = unit("километр", "километра", "километров", Gender::Masculine);
const CENTIMETER: UnitForms = unit("сантиметр", "сантиметра", "сантиметров", Gender::Masculine);
const MILLIMETER: UnitForms = unit("миллиметр", "миллиметра", "миллиметров", Gender::Masculine);
const METER: UnitForms = unit("метр", "метра", "метров", Gender::Masculine);
const LITER: UnitForms = unit("литр", "литра", "литров", Gender::Masculine);
const MILLILITER: UnitForms = unit("миллилитр", "миллилитра", "миллилитров", Gender::Masculine);
const GRAM: UnitForms = unit("грамм", "грамма", "граммов", Gender::Masculine);
const KILOBYTE: UnitForms = unit("килобайт", "килобайта", "килобайт", Gender::Masculine);
const MEGABYTE: UnitForms = unit("мегабайт", "мегабайта", "мегабайт", Gender::Masculine);
const GIGABYTE: UnitForms = unit("гигабайт", "гигабайта", "гигабайт", Gender::Masculine);
const TERABYTE: UnitForms = unit("терабайт", "терабайта", "терабайт", Gender::Masculine);
const MILLISECOND: UnitForms = unit(
    "миллисекунда",
    "миллисекунды",
    "миллисекунд",
    Gender::Feminine,
);
const SECOND: UnitForms = unit("секунда", "секунды", "секунд", Gender::Feminine);
const MINUTE: UnitForms = unit("минута", "минуты", "минут", Gender::Feminine);
const HOUR: UnitForms = unit("час", "часа", "часов", Gender::Masculine);
const PIXEL: UnitForms = unit("пиксель", "пикселя", "пикселей", Gender::Masculine);

/// Units with a fixed reading.
const EM: UnitForms = fixed_unit("эм", Gender::Masculine);
const REM: UnitForms = fixed_unit("рем", Gender::Masculine);
const VH: UnitForms = fixed_unit("ви эйч", Gender::Masculine);
const VW: UnitForms = fixed_unit("ви дабл ю", Gender::Masculine);

const CELSIUS: UnitForms = UnitForms {
    sg_nom: "градус",
    sg_gen: "градуса",
    pl_gen: "градусов",
    gender: Gender::Masculine,
    fixed: false,
    tail: "Цельсия",
};
const KM_PER_HOUR: UnitForms = UnitForms {
    sg_nom: "километр",
    sg_gen: "километра",
    pl_gen: "километров",
    gender: Gender::Masculine,
    fixed: false,
    tail: "в час",
};
const M_PER_SECOND: UnitForms = UnitForms {
    sg_nom: "метр",
    sg_gen: "метра",
    pl_gen: "метров",
    gender: Gender::Masculine,
    fixed: false,
    tail: "в секунду",
};

/// Look up a unit token, longest first. Case-insensitive for ASCII units,
/// exact for Cyrillic ones.
/// Unit spellings the scanner tries, longest first so a one-letter unit
/// cannot shadow a longer one ("м" vs "мл"); Cyrillic is exact-case.
pub const UNIT_TOKENS: [&str; 27] = [
    "км/ч", "м/с", "rem", "min", "sec", "кб", "мб", "гб", "тб", "kb", "mb", "gb", "tb", "ms", "hr",
    "px", "em", "vh", "vw", "кг", "км", "см", "мм", "мл", "м", "л", "г",
];

pub fn lookup_unit(token: &str) -> Option<Unit> {
    let lower = token.to_lowercase();
    let found = match lower.as_str() {
        "км/ч" => natural(KM_PER_HOUR),
        "м/с" => natural(M_PER_SECOND),
        "кг" => natural(KILOGRAM),
        "км" => natural(KILOMETER),
        "см" => natural(CENTIMETER),
        "мм" => natural(MILLIMETER),
        "м" => natural(METER),
        "л" => natural(LITER),
        "мл" => natural(MILLILITER),
        "г" => natural(GRAM),
        "мб" => size(MEGABYTE),
        "кб" => size(KILOBYTE),
        "гб" => size(GIGABYTE),
        "тб" => size(TERABYTE),
        "kb" => size(KILOBYTE),
        "mb" => size(MEGABYTE),
        "gb" => size(GIGABYTE),
        "tb" => size(TERABYTE),
        "ms" => size(MILLISECOND),
        "sec" => size(SECOND),
        "min" => size(MINUTE),
        "hr" => size(HOUR),
        "px" => size(PIXEL),
        "em" => size(EM),
        "rem" => size(REM),
        "vh" => size(VH),
        "vw" => size(VW),
        _ => return None,
    };
    Some(found)
}

/// Degree unit (`°C`, `5°C`, `20 °C`): the degree sign is part of the token.
pub const DEGREE_CELSIUS: UnitForms = CELSIUS;

// ── Currencies ────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Currency {
    Ruble,
    Dollar,
    Euro,
    Kopek,
    Pound,
}

impl Currency {
    /// Spoken noun forms by count.
    pub fn forms(self) -> (&'static str, &'static str, &'static str, Gender) {
        match self {
            Currency::Ruble => ("рубль", "рубля", "рублей", Gender::Masculine),
            Currency::Dollar => ("доллар", "доллара", "долларов", Gender::Masculine),
            // "евро" is indeclinable.
            Currency::Euro => ("евро", "евро", "евро", Gender::Masculine),
            Currency::Kopek => ("копейка", "копейки", "копеек", Gender::Feminine),
            Currency::Pound => ("фунт", "фунта", "фунтов", Gender::Masculine),
        }
    }

    pub fn from_symbol(symbol: &str) -> Option<Self> {
        match symbol {
            "₽" | "р." | "руб." | "руб" => Some(Currency::Ruble),
            "$" | "usd" | "долл." | "долл" => Some(Currency::Dollar),
            "€" | "eur" | "евро" => Some(Currency::Euro),
            "коп." | "коп" => Some(Currency::Kopek),
            "£" => Some(Currency::Pound),
            _ => None,
        }
    }
}

// ── Scale words ───────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Scale {
    Thousand,
    Million,
    Billion,
}

impl Scale {
    pub fn forms(self) -> (&'static str, &'static str, &'static str, Gender) {
        match self {
            Scale::Thousand => ("тысяча", "тысячи", "тысяч", Gender::Feminine),
            Scale::Million => ("миллион", "миллиона", "миллионов", Gender::Masculine),
            Scale::Billion => ("миллиард", "миллиарда", "миллиардов", Gender::Masculine),
        }
    }

    pub fn from_token(token: &str) -> Option<Self> {
        let token = token.trim_end_matches('.').to_lowercase();
        match token.as_str() {
            "тыс" => Some(Scale::Thousand),
            "млн" => Some(Scale::Million),
            "млрд" => Some(Scale::Billion),
            _ => None,
        }
    }
}

// ── Calendar words ────────────────────────────────────────────────────────────

/// Month names in the genitive, indexed by month number.
pub const MONTHS_GENITIVE: [&str; 12] = [
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

/// Month names in the nominative, indexed by month number.
pub const MONTHS_NOMINATIVE: [&str; 12] = [
    "январь",
    "февраль",
    "март",
    "апрель",
    "май",
    "июнь",
    "июль",
    "август",
    "сентябрь",
    "октябрь",
    "ноябрь",
    "декабрь",
];

/// Month number (1-12) for a genitive month name.
pub fn month_from_genitive(word: &str) -> Option<u32> {
    MONTHS_GENITIVE
        .iter()
        .position(|month| *month == word)
        .map(|index| index as u32 + 1)
}

/// Nominal month name (nominative) a year can follow: "декабрь 2024".
pub fn month_from_nominative(word: &str) -> Option<u32> {
    MONTHS_NOMINATIVE
        .iter()
        .position(|month| *month == word)
        .map(|index| index as u32 + 1)
}

/// Words that put a following year in the genitive with an implied "года"
/// ("лето 2024" → "лето две тысячи двадцать четвёртого года").
pub const YEAR_CONTEXT_WORDS: [&str; 8] = [
    "лето",
    "весна",
    "осень",
    "зима",
    "начало",
    "конец",
    "середина",
    "весной",
];

// ── Noun gender ───────────────────────────────────────────────────────────────

/// Gender of a noun form the reader has to agree with, when the ending alone
/// is not enough (plural and genitive forms like "книги", "рубля", "года").
const NOUN_GENDERS: [(&str, Gender); 72] = [
    ("сервер", Gender::Masculine),
    ("сервера", Gender::Masculine),
    ("серверы", Gender::Masculine),
    ("книга", Gender::Feminine),
    ("книги", Gender::Feminine),
    ("книг", Gender::Feminine),
    ("девочка", Gender::Feminine),
    ("девочки", Gender::Feminine),
    ("девочек", Gender::Feminine),
    ("неделя", Gender::Feminine),
    ("недели", Gender::Feminine),
    ("недель", Gender::Feminine),
    ("страница", Gender::Feminine),
    ("страницы", Gender::Feminine),
    ("страниц", Gender::Feminine),
    ("половина", Gender::Feminine),
    ("половины", Gender::Feminine),
    ("часть", Gender::Feminine),
    ("части", Gender::Feminine),
    ("глава", Gender::Feminine),
    ("главы", Gender::Feminine),
    ("попытка", Gender::Feminine),
    ("попытки", Gender::Feminine),
    ("очередь", Gender::Feminine),
    ("очереди", Gender::Feminine),
    ("тысяча", Gender::Feminine),
    ("тысячи", Gender::Feminine),
    ("минута", Gender::Feminine),
    ("минуты", Gender::Feminine),
    ("секунда", Gender::Feminine),
    ("секунды", Gender::Feminine),
    ("рубль", Gender::Masculine),
    ("рубля", Gender::Masculine),
    ("рублей", Gender::Masculine),
    ("год", Gender::Masculine),
    ("года", Gender::Masculine),
    ("году", Gender::Masculine),
    ("лет", Gender::Masculine),
    ("человек", Gender::Masculine),
    ("человека", Gender::Masculine),
    ("месяц", Gender::Masculine),
    ("месяца", Gender::Masculine),
    ("час", Gender::Masculine),
    ("часа", Gender::Masculine),
    ("день", Gender::Masculine),
    ("дня", Gender::Masculine),
    ("дней", Gender::Masculine),
    ("раз", Gender::Masculine),
    ("раза", Gender::Masculine),
    ("процент", Gender::Masculine),
    ("процента", Gender::Masculine),
    ("градус", Gender::Masculine),
    ("класс", Gender::Masculine),
    ("класса", Gender::Masculine),
    ("этаж", Gender::Masculine),
    ("этажа", Gender::Masculine),
    ("квартал", Gender::Masculine),
    ("квартала", Gender::Masculine),
    ("том", Gender::Masculine),
    ("тома", Gender::Masculine),
    // Genitive singular of neuter -о nouns also ends in -а, so the ending
    // heuristic would call these feminine; list the frequent ones.
    ("яблока", Gender::Neuter),
    ("молока", Gender::Neuter),
    ("озера", Gender::Neuter),
    ("окна", Gender::Neuter),
    ("письма", Gender::Neuter),
    ("мяса", Gender::Neuter),
    ("масла", Gender::Neuter),
    ("пива", Gender::Neuter),
    ("неба", Gender::Neuter),
    ("слова", Gender::Neuter),
    ("места", Gender::Neuter),
    ("дела", Gender::Neuter),
];

/// Gender of the noun a cardinal agrees with ("42 книги" → "сорок две книги").
///
/// The lexicon covers the ambiguous forms (plural -и/-ы, genitive -а/-я that
/// is masculine); the ending heuristic covers the rest of one-word nouns.
pub fn noun_gender(word: &str) -> Gender {
    let word = word
        .trim_matches(|c: char| !c.is_alphabetic())
        .to_lowercase();
    if word.is_empty() {
        return Gender::Masculine;
    }
    if let Some((_, gender)) = NOUN_GENDERS.iter().find(|(known, _)| *known == word) {
        return *gender;
    }
    if word.ends_with("мя") {
        return Gender::Neuter;
    }
    // Agent nouns in -тель are masculine ("посетитель"); only a handful of
    // ordinary nouns in -тель are feminine.
    if word.ends_with("тель") {
        const FEMININE: [&str; 5] = ["метель", "постель", "капель", "пастель", "обитель"];
        return if FEMININE.contains(&word.as_str()) {
            Gender::Feminine
        } else {
            Gender::Masculine
        };
    }
    if word.ends_with('а') || word.ends_with('я') || word.ends_with('ь') {
        return Gender::Feminine;
    }
    if word.ends_with('о') || word.ends_with('е') || word.ends_with('ё') {
        return Gender::Neuter;
    }
    Gender::Masculine
}

// ── Reference words ───────────────────────────────────────────────────────────

/// A reference prefix read together with the number that follows it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Reference {
    /// "№ 5" → "номер пять".
    NumberSign,
    /// "стр. 15" → "страница пятнадцать".
    Page,
    /// "табл. 3" → "таблица три".
    Table,
    /// "рис. 3" → "рисунок три".
    Figure,
    /// "гл. 2" → "глава вторая".
    Chapter,
    /// "тел. +7 …" → "телефон плюс семь …".
    Phone,
    /// "ИНН 7707083893" → digit by digit.
    TaxId,
    /// "арт. 123456" → digit by digit.
    Article,
}

impl Reference {
    /// Spoken word(s) the prefix turns into; a chapter keeps its own reading.
    pub fn word(self) -> &'static str {
        match self {
            Reference::NumberSign => "номер",
            Reference::Page => "страница",
            Reference::Table => "таблица",
            Reference::Figure => "рисунок",
            Reference::Chapter => "глава",
            Reference::Phone => "телефон",
            Reference::TaxId => "ИНН",
            Reference::Article => "артикул",
        }
    }

    /// Whether the number after the prefix is read digit by digit.
    pub fn digit_by_digit(self) -> bool {
        matches!(self, Reference::TaxId | Reference::Article)
    }

    /// Whether the number after the prefix is an ordinal.
    pub fn ordinal(self) -> bool {
        matches!(self, Reference::Chapter)
    }
}

/// Longest-first reference prefix match at the start of `text`.
/// Reference prefix match at the start of `text`. The prefix must end at a
/// token boundary, so "строка" and "глубина" are ordinary words.
pub fn match_reference(text: &str) -> Option<(Reference, usize)> {
    let first = text.chars().next()?;
    // "№" is a symbol prefix and its number may be glued ("№123").
    if first == '№' {
        return Some((Reference::NumberSign, first.len_utf8()));
    }

    const PREFIXES: [(&str, Reference); 7] = [
        ("стр", Reference::Page),
        ("табл", Reference::Table),
        ("рис", Reference::Figure),
        ("гл", Reference::Chapter),
        ("тел", Reference::Phone),
        ("инн", Reference::TaxId),
        ("арт", Reference::Article),
    ];
    let lower: String = text.chars().take(4).collect::<String>().to_lowercase();
    for (prefix, reference) in PREFIXES {
        if !lower.starts_with(prefix) {
            continue;
        }
        // Cyrillic prefixes keep their byte length through lowercasing.
        let mut consumed = prefix.len();
        if text[consumed..].starts_with('.') {
            consumed += 1;
        }
        let rest = &text[consumed..];
        let boundary = match reference {
            Reference::Phone | Reference::TaxId | Reference::Article => {
                rest.starts_with(|c: char| {
                    c.is_whitespace() || c.is_ascii_digit() || c == ':' || c == '+' || c == '('
                })
            }
            _ => rest.starts_with(|c: char| !c.is_alphabetic()),
        };
        if boundary {
            return Some((reference, consumed));
        }
    }
    None
}

/// Version pre-release suffix words.
pub fn version_suffix_word(word: &str) -> Option<&'static str> {
    match word.to_lowercase().as_str() {
        "alpha" => Some("альфа"),
        "beta" => Some("бета"),
        "rc" => Some("эр си"),
        "dev" => Some("дев"),
        "stable" => Some("стейбл"),
        "release" => Some("релиз"),
        _ => None,
    }
}

/// Words after which a cardinal bound reads in the genitive
/// ("от 5 до 10", "около 5 %", "более 5 %").
pub fn genitive_trigger(word: &str) -> bool {
    matches!(
        word,
        "от" | "до"
            | "с"
            | "со"
            | "из"
            | "около"
            | "более"
            | "менее"
            | "свыше"
            | "после"
            | "для"
            | "без"
            | "у"
            | "порядка"
            | "младше"
            | "старше"
    )
}

/// Head nouns that read a following number as an ordinal
/// ("глава 2" → "глава вторая").
pub fn ordinal_head(word: &str) -> Option<Gender> {
    match word {
        "глава" | "главы" | "главе" | "главу" | "главам" | "главах" => {
            Some(Gender::Feminine)
        }
        "часть" | "части" | "частью" | "частях" => Some(Gender::Feminine),
        "том" | "тома" | "томе" | "томам" | "томах" => Some(Gender::Masculine),
        "раздел" | "раздела" | "разделе" => Some(Gender::Masculine),
        "квартал" => Some(Gender::Masculine),
        "этаж" => Some(Gender::Masculine),
        _ => None,
    }
}

/// Heads whose "N-х" suffix is the ordinal plural ("в 5-х классах"),
/// unlike the default cardinal genitive ("в 2-х шагах").
pub fn plural_ordinal_head(word: &str) -> bool {
    word.starts_with("класс")
}
