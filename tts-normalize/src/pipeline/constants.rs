use std::collections::HashMap;

use once_cell::sync::Lazy;

pub static GREEK_LETTERS: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut m = HashMap::new();
    // Lowercase
    m.insert("α", "альфа");
    m.insert("β", "бета");
    m.insert("γ", "гамма");
    m.insert("δ", "дельта");
    m.insert("ε", "эпсилон");
    m.insert("ζ", "дзета");
    m.insert("η", "эта");
    m.insert("θ", "тета");
    m.insert("ι", "йота");
    m.insert("κ", "каппа");
    m.insert("λ", "лямбда");
    m.insert("μ", "мю");
    m.insert("ν", "ню");
    m.insert("ξ", "кси");
    m.insert("π", "пи");
    m.insert("ρ", "ро");
    m.insert("σ", "сигма");
    m.insert("τ", "тау");
    m.insert("υ", "ипсилон");
    m.insert("φ", "фи");
    m.insert("χ", "хи");
    m.insert("ψ", "пси");
    m.insert("ω", "омега");
    // Uppercase
    m.insert("Α", "альфа");
    m.insert("Β", "бета");
    m.insert("Γ", "гамма");
    m.insert("Δ", "дельта");
    m.insert("Ε", "эпсилон");
    m.insert("Ζ", "дзета");
    m.insert("Η", "эта");
    m.insert("Θ", "тета");
    m.insert("Ι", "йота");
    m.insert("Κ", "каппа");
    m.insert("Λ", "лямбда");
    m.insert("Μ", "мю");
    m.insert("Ν", "ню");
    m.insert("Ξ", "кси");
    m.insert("Π", "пи");
    m.insert("Ρ", "ро");
    m.insert("Σ", "сигма");
    m.insert("Τ", "тау");
    m.insert("Υ", "ипсилон");
    m.insert("Φ", "фи");
    m.insert("Χ", "хи");
    m.insert("Ψ", "пси");
    m.insert("Ω", "омега");
    m
});

pub static MATH_SYMBOLS: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert("∞", "бесконечность");
    m.insert("∈", "принадлежит");
    m.insert("∉", "не принадлежит");
    m.insert("∀", "для всех");
    m.insert("∃", "существует");
    m.insert("∅", "пустое множество");
    m.insert("∩", "пересечение");
    m.insert("∪", "объединение");
    m.insert("⊂", "подмножество");
    m.insert("≠", "не равно");
    m.insert("≈", "приблизительно равно");
    m.insert("≤", "меньше или равно");
    m.insert("≥", "больше или равно");
    m.insert("×", "умножить");
    m.insert("÷", "разделить");
    m.insert("√", "корень");
    m.insert("∑", "сумма");
    m.insert("∏", "произведение");
    m
});

pub static ARROW_SYMBOLS: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert("→", "стрелка");
    m.insert("←", "стрелка влево");
    m.insert("↔", "двунаправленная стрелка");
    m.insert("⇒", "следует");
    m.insert("⇐", "следует из");
    m.insert("⇔", "эквивалентно");
    m
});
