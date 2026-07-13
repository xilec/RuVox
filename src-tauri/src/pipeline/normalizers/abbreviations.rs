use std::collections::HashMap;
use std::sync::LazyLock;

static LETTER_MAP: LazyLock<HashMap<char, &'static str>> = LazyLock::new(|| {
    [
        ('a', "эй"),
        ('b', "би"),
        ('c', "си"),
        ('d', "ди"),
        ('e', "и"),
        ('f', "эф"),
        ('g', "джи"),
        ('h', "эйч"),
        ('i', "ай"),
        ('j', "джей"),
        ('k', "кей"),
        ('l', "эл"),
        ('m', "эм"),
        ('n', "эн"),
        ('o', "о"),
        ('p', "пи"),
        ('q', "кью"),
        ('r', "ар"),
        ('s', "эс"),
        ('t', "ти"),
        ('u', "ю"),
        ('v', "ви"),
        ('w', "дабл ю"),
        ('x', "экс"),
        ('y', "уай"),
        ('z', "зед"),
    ]
    .into_iter()
    .collect()
});

static AS_WORD: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    [
        // Data formats
        ("json", "джейсон"),
        ("yaml", "ямл"),
        ("toml", "томл"),
        // Protocols/Standards
        ("rest", "рест"),
        ("ajax", "эйджакс"),
        ("crud", "крад"),
        ("cors", "корс"),
        ("oauth", "о ауз"),
        // Image formats
        ("gif", "гиф"),
        ("jpeg", "джейпег"),
        // Memory
        ("ram", "рам"),
        ("rom", "ром"),
        // Network
        ("lan", "лан"),
        ("wan", "ван"),
        // Architecture
        ("spa", "спа"),
        ("dom", "дом"),
        // Other
        ("gui", "гуи"),
        ("imap", "ай мап"),
        ("pop", "поп"),
        // DevOps
        ("devops", "девопс"),
    ]
    .into_iter()
    .collect()
});

static SPECIAL_CASES: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    [
        ("ios", "ай оу эс"),
        ("macos", "мак оу эс"),
        ("graphql", "граф кью эл"),
        ("iot", "ай о ти"),
    ]
    .into_iter()
    .collect()
});

pub struct AbbreviationNormalizer;

impl AbbreviationNormalizer {
    pub fn new() -> Self {
        AbbreviationNormalizer
    }

    pub fn normalize(&self, abbrev: &str) -> String {
        if abbrev.is_empty() {
            return abbrev.to_string();
        }

        let lower = abbrev.to_lowercase();

        if let Some(&pronunciation) = SPECIAL_CASES.get(lower.as_str()) {
            return pronunciation.to_string();
        }

        if let Some(&pronunciation) = AS_WORD.get(lower.as_str()) {
            return pronunciation.to_string();
        }

        if abbrev.len() == 1 {
            let ch = lower.chars().next().unwrap();
            return LETTER_MAP
                .get(&ch)
                .map(|s| s.to_string())
                .unwrap_or_else(|| abbrev.to_string());
        }

        if abbrev.chars().all(|c| c.is_ascii_alphabetic()) {
            return self.spell_out(abbrev);
        }

        self.handle_mixed(abbrev)
    }

    fn spell_out(&self, abbrev: &str) -> String {
        abbrev
            .to_lowercase()
            .chars()
            .map(|c| {
                LETTER_MAP
                    .get(&c)
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| c.to_string())
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn handle_mixed(&self, abbrev: &str) -> String {
        abbrev
            .to_lowercase()
            .chars()
            .map(|c| {
                if let Some(&pronunciation) = LETTER_MAP.get(&c) {
                    pronunciation.to_string()
                } else {
                    c.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}

impl Default for AbbreviationNormalizer {
    fn default() -> Self {
        Self::new()
    }
}

pub fn letter_map() -> &'static HashMap<char, &'static str> {
    &LETTER_MAP
}

pub fn as_word() -> &'static HashMap<&'static str, &'static str> {
    &AS_WORD
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    fn normalizer() -> AbbreviationNormalizer {
        AbbreviationNormalizer::new()
    }

    #[test_case("JSON" => "джейсон"; "json")]
    #[test_case("YAML" => "ямл"; "yaml")]
    #[test_case("TOML" => "томл"; "toml")]
    #[test_case("REST" => "рест"; "rest")]
    #[test_case("AJAX" => "эйджакс"; "ajax")]
    #[test_case("CRUD" => "крад"; "crud")]
    #[test_case("CORS" => "корс"; "cors")]
    #[test_case("OAuth" => "о ауз"; "oauth")]
    #[test_case("GIF" => "гиф"; "gif")]
    #[test_case("JPEG" => "джейпег"; "jpeg")]
    #[test_case("PNG" => "пи эн джи"; "png_spelled_out")]
    #[test_case("RAM" => "рам"; "ram")]
    #[test_case("ROM" => "ром"; "rom")]
    #[test_case("LAN" => "лан"; "lan")]
    #[test_case("WAN" => "ван"; "wan")]
    #[test_case("SPA" => "спа"; "spa")]
    #[test_case("DOM" => "дом"; "dom")]
    #[test_case("GUI" => "гуи"; "gui")]
    #[test_case("IMAP" => "ай мап"; "imap")]
    #[test_case("POP" => "поп"; "pop")]
    fn as_word_lookup(input: &str) -> String {
        normalizer().normalize(input)
    }

    #[test_case("HTTP" => "эйч ти ти пи"; "http")]
    #[test_case("HTTPS" => "эйч ти ти пи эс"; "https")]
    #[test_case("HTML" => "эйч ти эм эл"; "html")]
    #[test_case("CSS" => "си эс эс"; "css")]
    #[test_case("XML" => "экс эм эл"; "xml")]
    #[test_case("URL" => "ю ар эл"; "url")]
    #[test_case("URI" => "ю ар ай"; "uri")]
    #[test_case("API" => "эй пи ай"; "api")]
    #[test_case("SDK" => "эс ди кей"; "sdk")]
    #[test_case("CLI" => "си эл ай"; "cli")]
    #[test_case("IDE" => "ай ди и"; "ide")]
    #[test_case("SSL" => "эс эс эл"; "ssl")]
    #[test_case("TLS" => "ти эл эс"; "tls")]
    #[test_case("SSH" => "эс эс эйч"; "ssh")]
    #[test_case("VPN" => "ви пи эн"; "vpn")]
    #[test_case("JWT" => "джей дабл ю ти"; "jwt")]
    #[test_case("XSS" => "экс эс эс"; "xss")]
    #[test_case("CSRF" => "си эс ар эф"; "csrf")]
    #[test_case("TCP" => "ти си пи"; "tcp")]
    #[test_case("UDP" => "ю ди пи"; "udp")]
    #[test_case("FTP" => "эф ти пи"; "ftp")]
    #[test_case("DNS" => "ди эн эс"; "dns")]
    #[test_case("SMTP" => "эс эм ти пи"; "smtp")]
    #[test_case("IP" => "ай пи"; "ip")]
    #[test_case("CPU" => "си пи ю"; "cpu")]
    #[test_case("GPU" => "джи пи ю"; "gpu")]
    #[test_case("SSD" => "эс эс ди"; "ssd")]
    #[test_case("HDD" => "эйч ди ди"; "hdd")]
    #[test_case("USB" => "ю эс би"; "usb")]
    #[test_case("HDMI" => "эйч ди эм ай"; "hdmi")]
    #[test_case("UI" => "ю ай"; "ui")]
    #[test_case("UX" => "ю экс"; "ux")]
    #[test_case("CI" => "си ай"; "ci")]
    #[test_case("CD" => "си ди"; "cd")]
    #[test_case("AI" => "эй ай"; "ai")]
    #[test_case("ML" => "эм эл"; "ml")]
    #[test_case("NLP" => "эн эл пи"; "nlp")]
    #[test_case("CV" => "си ви"; "cv")]
    #[test_case("SQL" => "эс кью эл"; "sql")]
    #[test_case("ORM" => "о ар эм"; "orm")]
    #[test_case("MVC" => "эм ви си"; "mvc")]
    #[test_case("MVP" => "эм ви пи"; "mvp")]
    #[test_case("IoT" => "ай о ти"; "iot_special_case")]
    #[test_case("SSR" => "эс эс ар"; "ssr")]
    #[test_case("SSG" => "эс эс джи"; "ssg")]
    #[test_case("CSR" => "си эс ар"; "csr")]
    #[test_case("PWA" => "пи дабл ю эй"; "pwa")]
    #[test_case("SVG" => "эс ви джи"; "svg")]
    fn spelled_out(input: &str) -> String {
        normalizer().normalize(input)
    }

    #[test_case("A" => "эй"; "a")]
    #[test_case("B" => "би"; "b")]
    #[test_case("C" => "си"; "c")]
    #[test_case("D" => "ди"; "d")]
    #[test_case("E" => "и"; "e")]
    #[test_case("F" => "эф"; "f")]
    #[test_case("G" => "джи"; "g")]
    #[test_case("H" => "эйч"; "h")]
    #[test_case("I" => "ай"; "i")]
    #[test_case("J" => "джей"; "j")]
    #[test_case("K" => "кей"; "k")]
    #[test_case("L" => "эл"; "l")]
    #[test_case("M" => "эм"; "m")]
    #[test_case("N" => "эн"; "n")]
    #[test_case("O" => "о"; "o")]
    #[test_case("P" => "пи"; "p")]
    #[test_case("Q" => "кью"; "q")]
    #[test_case("R" => "ар"; "r")]
    #[test_case("S" => "эс"; "s")]
    #[test_case("T" => "ти"; "t")]
    #[test_case("U" => "ю"; "u")]
    #[test_case("V" => "ви"; "v")]
    #[test_case("W" => "дабл ю"; "w")]
    #[test_case("X" => "экс"; "x")]
    #[test_case("Y" => "уай"; "y")]
    #[test_case("Z" => "зед"; "z")]
    fn letter(input: &str) -> String {
        normalizer().normalize(input)
    }

    #[test_case("json" => "джейсон"; "json_lowercase")]
    #[test_case("Json" => "джейсон"; "json_mixed_case")]
    #[test_case("api" => "эй пи ай"; "api_lowercase")]
    #[test_case("Api" => "эй пи ай"; "api_mixed_case")]
    fn case_insensitive(input: &str) -> String {
        normalizer().normalize(input)
    }

    #[test_case("XYZ" => "экс уай зед"; "xyz")]
    #[test_case("ABC" => "эй би си"; "abc")]
    #[test_case("QRS" => "кью ар эс"; "qrs")]
    #[test_case("WXYZ" => "дабл ю экс уай зед"; "wxyz")]
    fn unknown_abbreviation(input: &str) -> String {
        normalizer().normalize(input)
    }

    #[test_case("iOS" => "ай оу эс"; "ios")]
    #[test_case("macOS" => "мак оу эс"; "macos")]
    #[test_case("DevOps" => "девопс"; "devops")]
    #[test_case("GraphQL" => "граф кью эл"; "graphql")]
    fn special_case(input: &str) -> String {
        normalizer().normalize(input)
    }

    #[test_case("" => ""; "empty_string")]
    fn edge_case(input: &str) -> String {
        normalizer().normalize(input)
    }

    // --- Public accessors ---

    #[test]
    fn test_letter_map_accessible() {
        let map = letter_map();
        assert_eq!(map.get(&'a'), Some(&"эй"));
        assert_eq!(map.get(&'z'), Some(&"зед"));
        assert_eq!(map.len(), 26);
    }

    #[test]
    fn test_as_word_accessible() {
        let map = as_word();
        assert_eq!(map.get("json"), Some(&"джейсон"));
        assert_eq!(map.get("api"), None); // api is spelled out, not in AS_WORD
    }
}
