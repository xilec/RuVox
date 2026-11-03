//! Derivation of an entry title from its text (issue #353).
//!
//! A new entry gets a title seeded from the first usable line of its
//! `original_text`, so the queue and the viewer header have an identity
//! without the user typing anything. Legacy entries (`title: None`) run the
//! same algorithm in the UI, so both look identical until edited.
//!
//! `src/lib/entryTitle.ts` mirrors this module. Keep the shared case table in
//! both test suites in sync — the frontend must derive exactly what the
//! backend seeded, otherwise a legacy entry would render a different title
//! from an identical freshly added one.

/// Maximum title length in Unicode scalar values.
pub const TITLE_MAX_CHARS: usize = 200;

/// Derive a display title from `text`.
///
/// Uses the first non-empty line that yields something after stripping
/// Markdown structure, collapses internal whitespace to single spaces, trims,
/// and caps the result at [`TITLE_MAX_CHARS`] characters. Returns `None` when
/// the text has nothing usable (empty, whitespace-only, or only markup marks).
pub fn derive_title(text: &str) -> Option<String> {
    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let collapsed = collapse_whitespace(strip_markdown_prefix(line));
        let trimmed = collapsed.trim();
        if trimmed.is_empty() {
            continue;
        }
        let capped: String = trimmed.chars().take(TITLE_MAX_CHARS).collect();
        return Some(capped.trim_end().to_string());
    }
    None
}

/// Collapse every run of Unicode whitespace into a single space.
fn collapse_whitespace(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut pending_space = false;
    for ch in text.chars() {
        if ch.is_whitespace() {
            pending_space = true;
            continue;
        }
        if pending_space && !out.is_empty() {
            out.push(' ');
        }
        pending_space = false;
        out.push(ch);
    }
    out
}

/// Strip repeated leading Markdown structure markers (`##`, `-`, `1.`, ...).
fn strip_markdown_prefix(mut line: &str) -> &str {
    loop {
        let next = strip_one_prefix(line);
        if next.len() == line.len() {
            return line;
        }
        line = next;
    }
}

/// Strip one leading heading marker or list bullet. Markers must be followed
/// by whitespace (matching Markdown), so `#hashtag` and `-5` are left alone.
fn strip_one_prefix(line: &str) -> &str {
    let hashes = line.chars().take_while(|&c| c == '#').count();
    if (1..=6).contains(&hashes) {
        let rest = &line[hashes..];
        if rest.is_empty() || rest.starts_with(char::is_whitespace) {
            return rest.trim_start();
        }
    }
    if let Some(rest) = strip_list_bullet(line) {
        return rest.trim_start();
    }
    line
}

fn strip_list_bullet(line: &str) -> Option<&str> {
    let first = line.chars().next()?;
    if matches!(first, '-' | '*' | '+') {
        let rest = &line[first.len_utf8()..];
        return (rest.is_empty() || rest.starts_with(char::is_whitespace)).then_some(rest);
    }
    if first.is_ascii_digit() {
        let digits_end = line
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(line.len());
        // A sane ordered-list marker; guard against treating a long number
        // as a bullet.
        if digits_end <= 9 {
            let marker = &line[digits_end..];
            if marker.starts_with('.') || marker.starts_with(')') {
                let rest = &marker[1..];
                if rest.is_empty() || rest.starts_with(char::is_whitespace) {
                    return Some(rest);
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Shared case table — mirrored verbatim in `src/lib/entryTitle.test.ts`.
    /// Change both at once; the parity is the point.
    const CASES: &[(&str, Option<&str>)] = &[
        ("Привет мир", Some("Привет мир")),
        ("## Установка\nТекст", Some("Установка")),
        ("# Заголовок с  пробелами", Some("Заголовок с пробелами")),
        ("- Первый пункт\n- Второй", Some("Первый пункт")),
        ("* Звёздный пункт", Some("Звёздный пункт")),
        ("+ Плюсовой пункт", Some("Плюсовой пункт")),
        ("1. Первый шаг", Some("Первый шаг")),
        ("12) Двенадцатый шаг", Some("Двенадцатый шаг")),
        (
            "   \n\n  # Заголовок после пустых строк  \n",
            Some("Заголовок после пустых строк"),
        ),
        ("#\n# Настоящий заголовок", Some("Настоящий заголовок")),
        ("#hashtag", Some("#hashtag")),
        ("-5 градусов", Some("-5 градусов")),
        ("", None),
        ("   \n\t\n ", None),
        ("#", None),
        ("- ", None),
        ("\u{2003}\n", None),
    ];

    #[test]
    fn derive_matches_shared_cases() {
        for (input, expected) in CASES {
            assert_eq!(
                derive_title(input).as_deref(),
                *expected,
                "derive_title({input:?})"
            );
        }
    }

    #[test]
    fn derives_first_non_empty_line() {
        assert_eq!(
            derive_title("\n\n  Вторая строка\nТретья").as_deref(),
            Some("Вторая строка")
        );
    }

    #[test]
    fn caps_at_max_chars() {
        let long = "ж".repeat(TITLE_MAX_CHARS + 50);
        let title = derive_title(&long).unwrap();
        assert_eq!(title.chars().count(), TITLE_MAX_CHARS);
    }

    #[test]
    fn cap_does_not_leave_a_trailing_space() {
        // The 200th character is the space before the tail word.
        let text = format!("{}{}", "ж".repeat(TITLE_MAX_CHARS - 1), " хвост");
        let title = derive_title(&text).unwrap();
        assert_eq!(title.chars().count(), TITLE_MAX_CHARS - 1);
        assert!(!title.ends_with(' '));
    }

    #[test]
    fn strips_nested_markers() {
        assert_eq!(derive_title("- ## Вложенный").as_deref(), Some("Вложенный"));
    }
}
