use super::english::EnglishNormalizer;
use super::numbers::NumberNormalizer;

// Protocol pronunciations
const PROTOCOLS: &[(&str, &str)] = &[
    ("https", "эйч ти ти пи эс"),
    ("http", "эйч ти ти пи"),
    ("ftp", "эф ти пи"),
    ("ssh", "эс эс эйч"),
    ("git", "гит"),
    ("file", "файл"),
    ("sftp", "эс эф ти пи"),
    ("ws", "веб сокет"),
    ("wss", "веб сокет секьюр"),
];

// Top-level domain pronunciations
const TLD_MAP: &[(&str, &str)] = &[
    ("com", "ком"),
    ("org", "орг"),
    ("net", "нет"),
    ("ru", "ру"),
    ("io", "ай оу"),
    ("dev", "дев"),
    ("app", "апп"),
    ("ai", "эй ай"),
    ("co", "ко"),
    ("me", "ми"),
    ("uk", "ю кей"),
    ("edu", "еду"),
    ("gov", "гов"),
    ("info", "инфо"),
    ("biz", "биз"),
];

// Windows drive letter pronunciations
const DRIVE_LETTERS: &[(&str, &str)] = &[
    ("c", "си"),
    ("d", "ди"),
    ("e", "и"),
    ("f", "эф"),
    ("g", "джи"),
    ("h", "эйч"),
];

fn lookup_tld(tld: &str) -> Option<&'static str> {
    let lower = tld.to_lowercase();
    TLD_MAP.iter().find(|(k, _)| *k == lower.as_str()).map(|(_, v)| *v)
}

fn lookup_protocol(scheme: &str) -> Option<&'static str> {
    let lower = scheme.to_lowercase();
    PROTOCOLS.iter().find(|(k, _)| *k == lower.as_str()).map(|(_, v)| *v)
}

fn lookup_drive(letter: &str) -> Option<&'static str> {
    let lower = letter.to_lowercase();
    DRIVE_LETTERS.iter().find(|(k, _)| *k == lower.as_str()).map(|(_, v)| *v)
}

/// Normalizes URLs, emails, IP addresses, and file paths to speakable Russian text.
///
/// When `english` is `None`, alphabetic segments in URLs and paths are kept verbatim
/// (matching Python behavior when `english_normalizer=None`). When provided, segments
/// are transliterated via the English normalizer before output. R9 integration will
/// always pass the normalizer; the `None` path exists only for legacy parity.
pub struct URLPathNormalizer<'a> {
    pub numbers: &'a NumberNormalizer,
    english: Option<&'a EnglishNormalizer>,
}

impl<'a> URLPathNormalizer<'a> {
    pub fn new(english: &'a EnglishNormalizer, numbers: &'a NumberNormalizer) -> Self {
        Self {
            numbers,
            english: Some(english),
        }
    }

    /// Create a normalizer that passes word segments through verbatim (no transliteration).
    ///
    /// Matches Python behavior when `english_normalizer=None` — used in tests and
    /// contexts where downstream processing will handle transliteration separately.
    pub fn new_without_english(numbers: &'a NumberNormalizer) -> Self {
        Self { numbers, english: None }
    }

    fn transliterate_word(&self, word: &str) -> String {
        if word.is_empty() {
            return word.to_string();
        }
        // When no EnglishNormalizer is provided, pass through verbatim.
        // This matches Python URLPathNormalizer(english_normalizer=None) behavior.
        match self.english {
            None => word.to_string(),
            Some(_en) => {
                if !word.is_ascii() || !word.chars().any(|c| c.is_alphabetic()) {
                    return word.to_string();
                }
                let lower = word.to_lowercase();
                // Check IT_TERMS first (e.g. "github" → "гитхаб").
                if let Some(v) = super::english::IT_TERMS.get(lower.as_str()) {
                    return v.to_string();
                }
                super::english::transliterate_simple(&lower)
            }
        }
    }

    fn transliterate_segment(&self, segment: &str) -> String {
        if segment.is_empty() {
            return segment.to_string();
        }
        if !segment.contains('-') {
            return self.transliterate_word(segment);
        }
        // Split by hyphens; numeric parts → number words, alphabetic → transliterate.
        let parts: Vec<String> = segment
            .split('-')
            .map(|part| {
                if !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()) {
                    self.numbers.normalize_number(part)
                } else {
                    self.transliterate_word(part)
                }
            })
            .collect();
        parts.join(" ")
    }

    pub fn normalize_url(&self, url: &str) -> String {
        if url.is_empty() {
            return url.to_string();
        }

        // Split off scheme (everything before "://").
        let (scheme, rest) = if let Some(pos) = url.find("://") {
            (&url[..pos], &url[pos + 3..])
        } else {
            return url.to_string();
        };

        let mut parts: Vec<String> = Vec::new();

        // Protocol
        if let Some(proto) = lookup_protocol(scheme) {
            parts.push(proto.to_string());
        } else {
            parts.push(scheme.to_string());
        }

        parts.push("двоеточие слэш слэш".to_string());

        // Split authority (host[:port]) from path/query/fragment.
        let (authority, path_query_fragment) = if let Some(pos) = rest.find('/') {
            (&rest[..pos], &rest[pos..])
        } else if let Some(pos) = rest.find('?') {
            (&rest[..pos], &rest[pos..])
        } else if let Some(pos) = rest.find('#') {
            (&rest[..pos], &rest[pos..])
        } else {
            (rest, "")
        };

        // Extract optional port from authority.
        let (host, port) = if let Some(colon_pos) = authority.rfind(':') {
            let maybe_port = &authority[colon_pos + 1..];
            if !maybe_port.is_empty() && maybe_port.chars().all(|c| c.is_ascii_digit()) {
                (&authority[..colon_pos], Some(maybe_port))
            } else {
                (authority, None)
            }
        } else {
            (authority, None)
        };

        // Domain parts with TLD handling.
        let domain_parts: Vec<&str> = host.split('.').collect();
        let domain_words: Vec<String> = domain_parts
            .iter()
            .enumerate()
            .map(|(i, part)| {
                if i == domain_parts.len() - 1 {
                    if let Some(tld) = lookup_tld(part) {
                        return tld.to_string();
                    }
                }
                if !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()) {
                    self.numbers.normalize_number(part)
                } else {
                    self.transliterate_word(part)
                }
            })
            .collect();
        parts.push(domain_words.join(" точка "));

        // Port
        if let Some(p) = port {
            parts.push("двоеточие".to_string());
            parts.push(self.numbers.normalize_number(p));
        }

        // Separate path from query/fragment.
        let (path_str, after_path) = if let Some(pos) = path_query_fragment.find('?') {
            (&path_query_fragment[..pos], &path_query_fragment[pos + 1..])
        } else if let Some(pos) = path_query_fragment.find('#') {
            (&path_query_fragment[..pos], &path_query_fragment[pos..])
        } else {
            (path_query_fragment, "")
        };

        // Separate query from fragment.
        let (query_str, fragment_str) = if let Some(stripped) = after_path.strip_prefix('#') {
            ("", stripped)
        } else if let Some(pos) = after_path.find('#') {
            (&after_path[..pos], &after_path[pos + 1..])
        } else {
            (after_path, "")
        };

        // Path segments
        if !path_str.is_empty() && path_str != "/" {
            for segment in path_str.trim_matches('/').split('/') {
                if segment.is_empty() {
                    continue;
                }
                parts.push("слэш".to_string());
                if segment.contains('.') {
                    let seg_parts: Vec<&str> = segment.split('.').collect();
                    let seg_words: Vec<String> = seg_parts
                        .iter()
                        .map(|sp| {
                            if !sp.is_empty() && sp.chars().all(|c| c.is_ascii_digit()) {
                                self.numbers.normalize_number(sp)
                            } else {
                                self.transliterate_segment(sp)
                            }
                        })
                        .collect();
                    parts.push(seg_words.join(" точка "));
                } else {
                    parts.push(self.transliterate_segment(segment));
                }
            }
        }

        // Query parameters (simplified — key=value pairs).
        if !query_str.is_empty() {
            parts.push("вопросительный знак".to_string());
            for qp in query_str.split('&') {
                if let Some(eq_pos) = qp.find('=') {
                    let key = &qp[..eq_pos];
                    let value = &qp[eq_pos + 1..];
                    parts.push(key.to_string());
                    parts.push("равно".to_string());
                    parts.push(value.to_string());
                }
            }
        }

        // Fragment
        if !fragment_str.is_empty() {
            parts.push("решётка".to_string());
            parts.push(fragment_str.to_string());
        }

        parts.join(" ")
    }

    fn normalize_identifier(&self, identifier: &str) -> String {
        let mut result: Vec<String> = Vec::new();
        let mut current_word = String::new();
        let mut chars = identifier.chars().peekable();

        while let Some(ch) = chars.next() {
            match ch {
                '.' => {
                    if !current_word.is_empty() {
                        result.push(current_word.clone());
                        current_word.clear();
                    }
                    result.push("точка".to_string());
                }
                '_' => {
                    if !current_word.is_empty() {
                        result.push(current_word.clone());
                        current_word.clear();
                    }
                    result.push("андерскор".to_string());
                }
                '-' => {
                    if !current_word.is_empty() {
                        result.push(current_word.clone());
                        current_word.clear();
                    }
                    result.push("дефис".to_string());
                }
                c if c.is_ascii_digit() => {
                    if !current_word.is_empty() {
                        result.push(current_word.clone());
                        current_word.clear();
                    }
                    let mut num_str = String::new();
                    num_str.push(c);
                    while let Some(&next) = chars.peek() {
                        if next.is_ascii_digit() {
                            num_str.push(chars.next().unwrap());
                        } else {
                            break;
                        }
                    }
                    result.push(self.numbers.normalize_number(&num_str));
                }
                other => {
                    current_word.push(other);
                }
            }
        }

        if !current_word.is_empty() {
            result.push(current_word);
        }

        result.join(" ")
    }

    pub fn normalize_email(&self, email: &str) -> String {
        if email.is_empty() || !email.contains('@') {
            return email.to_string();
        }

        let at_pos = email.rfind('@').unwrap();
        let local_part = &email[..at_pos];
        let domain = &email[at_pos + 1..];

        let mut parts: Vec<String> = Vec::new();

        parts.push(self.normalize_identifier(local_part));
        parts.push("собака".to_string());

        let domain_parts: Vec<&str> = domain.split('.').collect();
        let domain_words: Vec<String> = domain_parts
            .iter()
            .enumerate()
            .map(|(i, part)| {
                if i == domain_parts.len() - 1 {
                    if let Some(tld) = lookup_tld(part) {
                        return tld.to_string();
                    }
                }
                part.to_string()
            })
            .collect();
        parts.push(domain_words.join(" точка "));

        parts.join(" ")
    }

    pub fn normalize_ip(&self, ip: &str) -> String {
        if ip.is_empty() {
            return ip.to_string();
        }

        let octets: Vec<&str> = ip.split('.').collect();
        if octets.len() != 4 {
            return ip.to_string();
        }

        let parts: Vec<String> = octets
            .iter()
            .map(|octet| match octet.parse::<i64>() {
                Ok(n) => self.numbers.normalize_number(&n.to_string()),
                Err(_) => octet.to_string(),
            })
            .collect();

        parts.join(" точка ")
    }

    fn normalize_filename_part(&self, part: &str) -> String {
        if part.contains('-') {
            let subparts: Vec<&str> = part.split('-').collect();
            subparts.join(" дефис ")
        } else {
            part.to_string()
        }
    }

    pub fn normalize_filepath(&self, path: &str) -> String {
        if path.is_empty() {
            return path.to_string();
        }

        let mut parts: Vec<String> = Vec::new();

        let (segments, separator): (Vec<&str>, &str) = if path.contains('\\') {
            (path.split('\\').collect(), "бэкслэш")
        } else {
            (path.split('/').collect(), "слэш")
        };

        for (i, segment) in segments.iter().enumerate() {
            if i > 0 {
                parts.push(separator.to_string());
            }

            if segment.is_empty() {
                // Empty segment from leading slash or double slash — skip content but
                // the separator was already added, so path like "/home" renders as "слэш home".
                continue;
            }

            if *segment == "~" {
                parts.push("тильда".to_string());
            } else if *segment == "." {
                parts.push("точка".to_string());
            } else if *segment == ".." {
                parts.push("точка точка".to_string());
            } else if segment.len() == 2
                && segment.ends_with(':')
                && segment.starts_with(|c: char| c.is_ascii_alphabetic())
            {
                // Windows drive letter (e.g. C:)
                let drive = segment[..1].to_lowercase();
                if let Some(pronounced) = lookup_drive(&drive) {
                    parts.push(pronounced.to_string());
                } else {
                    parts.push(drive);
                }
                parts.push("двоеточие".to_string());
            } else if let Some(rest) = segment.strip_prefix('.') {
                // Hidden file/directory (starts with .)
                parts.push("точка".to_string());
                if rest.contains('.') {
                    // Has extension: split on last dot.
                    let dot_pos = rest.rfind('.').unwrap();
                    let name = &rest[..dot_pos];
                    let ext = &rest[dot_pos + 1..];
                    parts.push(self.normalize_filename_part(name));
                    parts.push("точка".to_string());
                    parts.push(ext.to_string());
                } else {
                    parts.push(rest.to_string());
                }
            } else if segment.contains('.') {
                // Filename with one or more extensions (e.g. test.spec.ts).
                let dot_parts: Vec<&str> = segment.split('.').collect();
                for (j, dp) in dot_parts.iter().enumerate() {
                    if j > 0 {
                        parts.push("точка".to_string());
                    }
                    parts.push(self.normalize_filename_part(dp));
                }
            } else {
                // Regular directory/file segment with no extension.
                parts.push(self.normalize_filename_part(segment));
            }
        }

        parts.join(" ")
    }
}

// ---- Tests ----

#[cfg(test)]
mod tests {
    use super::super::english::EnglishNormalizer;
    use super::super::numbers::NumberNormalizer;
    use super::*;

    fn mk_normalizer() -> (EnglishNormalizer, NumberNormalizer) {
        (EnglishNormalizer::new(), NumberNormalizer::new())
    }

    /// Create a URLPathNormalizer that matches Python test fixture behavior
    /// (no English transliteration — word segments pass through verbatim).
    fn norm_no_en(nn: &NumberNormalizer) -> URLPathNormalizer<'_> {
        URLPathNormalizer::new_without_english(nn)
    }

    fn norm<'a>(en: &'a EnglishNormalizer, nn: &'a NumberNormalizer) -> URLPathNormalizer<'a> {
        URLPathNormalizer::new(en, nn)
    }

    // ---- TestURLNormalization ----

    #[test]
    fn test_url_https_example_com() {
        let (_, nn) = mk_normalizer();
        let n = norm_no_en(&nn);
        assert_eq!(
            n.normalize_url("https://example.com"),
            "эйч ти ти пи эс двоеточие слэш слэш example точка ком"
        );
    }

    #[test]
    fn test_url_http_test_org() {
        let (_, nn) = mk_normalizer();
        let n = norm_no_en(&nn);
        assert_eq!(
            n.normalize_url("http://test.org"),
            "эйч ти ти пи двоеточие слэш слэш test точка орг"
        );
    }

    #[test]
    fn test_url_with_path() {
        let (_, nn) = mk_normalizer();
        let n = norm_no_en(&nn);
        assert_eq!(
            n.normalize_url("https://github.com/user/repo"),
            "эйч ти ти пи эс двоеточие слэш слэш github точка ком слэш user слэш repo"
        );
    }

    #[test]
    fn test_url_python_docs_version_path() {
        let (_, nn) = mk_normalizer();
        let n = norm_no_en(&nn);
        assert_eq!(
            n.normalize_url("https://docs.python.org/3.11/tutorial"),
            "эйч ти ти пи эс двоеточие слэш слэш docs точка python точка орг слэш три точка одиннадцать слэш tutorial"
        );
    }

    #[test]
    fn test_url_with_file_extension() {
        let (_, nn) = mk_normalizer();
        let n = norm_no_en(&nn);
        assert_eq!(
            n.normalize_url("https://example.com/file.html"),
            "эйч ти ти пи эс двоеточие слэш слэш example точка ком слэш file точка html"
        );
    }

    #[test]
    fn test_url_subdomain() {
        let (_, nn) = mk_normalizer();
        let n = norm_no_en(&nn);
        assert_eq!(
            n.normalize_url("https://api.github.com/repos"),
            "эйч ти ти пи эс двоеточие слэш слэш api точка github точка ком слэш repos"
        );
    }

    #[test]
    fn test_url_with_port_8080() {
        let (_, nn) = mk_normalizer();
        let n = norm_no_en(&nn);
        assert_eq!(
            n.normalize_url("http://localhost:8080"),
            "эйч ти ти пи двоеточие слэш слэш localhost двоеточие восемь тысяч восемьдесят"
        );
    }

    #[test]
    fn test_url_with_port_3000_and_path() {
        let (_, nn) = mk_normalizer();
        let n = norm_no_en(&nn);
        assert_eq!(
            n.normalize_url("http://localhost:3000/api"),
            "эйч ти ти пи двоеточие слэш слэш localhost двоеточие три тысячи слэш api"
        );
    }

    // ---- TestCommonTLDs ----

    #[test]
    fn test_tld_com() {
        let (_, nn) = mk_normalizer();
        let n = norm_no_en(&nn);
        assert!(n.normalize_url("https://example.com").contains("ком"));
    }

    #[test]
    fn test_tld_org() {
        let (_, nn) = mk_normalizer();
        let n = norm_no_en(&nn);
        assert!(n.normalize_url("https://example.org").contains("орг"));
    }

    #[test]
    fn test_tld_net() {
        let (_, nn) = mk_normalizer();
        let n = norm_no_en(&nn);
        assert!(n.normalize_url("https://example.net").contains("нет"));
    }

    #[test]
    fn test_tld_ru() {
        let (_, nn) = mk_normalizer();
        let n = norm_no_en(&nn);
        assert!(n.normalize_url("https://example.ru").contains("ру"));
    }

    #[test]
    fn test_tld_io() {
        let (_, nn) = mk_normalizer();
        let n = norm_no_en(&nn);
        assert!(n.normalize_url("https://example.io").contains("ай оу"));
    }

    #[test]
    fn test_tld_dev() {
        let (_, nn) = mk_normalizer();
        let n = norm_no_en(&nn);
        assert!(n.normalize_url("https://example.dev").contains("дев"));
    }

    #[test]
    fn test_tld_app() {
        let (_, nn) = mk_normalizer();
        let n = norm_no_en(&nn);
        assert!(n.normalize_url("https://example.app").contains("апп"));
    }

    #[test]
    fn test_tld_ai() {
        let (_, nn) = mk_normalizer();
        let n = norm_no_en(&nn);
        assert!(n.normalize_url("https://example.ai").contains("эй ай"));
    }

    #[test]
    fn test_tld_co() {
        let (_, nn) = mk_normalizer();
        let n = norm_no_en(&nn);
        assert!(n.normalize_url("https://example.co").contains("ко"));
    }

    #[test]
    fn test_tld_me() {
        let (_, nn) = mk_normalizer();
        let n = norm_no_en(&nn);
        assert!(n.normalize_url("https://example.me").contains("ми"));
    }

    // ---- TestProtocols ----

    #[test]
    fn test_protocol_https() {
        let (_, nn) = mk_normalizer();
        let n = norm_no_en(&nn);
        assert!(n.normalize_url("https://example.com").starts_with("эйч ти ти пи эс"));
    }

    #[test]
    fn test_protocol_http() {
        let (_, nn) = mk_normalizer();
        let n = norm_no_en(&nn);
        assert!(n.normalize_url("http://example.com").starts_with("эйч ти ти пи"));
    }

    #[test]
    fn test_protocol_ftp() {
        let (_, nn) = mk_normalizer();
        let n = norm_no_en(&nn);
        assert!(n.normalize_url("ftp://files.example.com").starts_with("эф ти пи"));
    }

    #[test]
    fn test_protocol_ssh() {
        let (_, nn) = mk_normalizer();
        let n = norm_no_en(&nn);
        assert!(n.normalize_url("ssh://server.example.com").starts_with("эс эс эйч"));
    }

    #[test]
    fn test_protocol_git() {
        let (_, nn) = mk_normalizer();
        let n = norm_no_en(&nn);
        assert!(n.normalize_url("git://github.com/repo.git").starts_with("гит"));
    }

    #[test]
    fn test_protocol_file() {
        let (_, nn) = mk_normalizer();
        let n = norm_no_en(&nn);
        assert!(n.normalize_url("file:///home/user/doc.txt").starts_with("файл"));
    }

    // ---- TestEmailNormalization ----

    #[test]
    fn test_email_simple() {
        let (_, nn) = mk_normalizer();
        let n = norm_no_en(&nn);
        assert_eq!(
            n.normalize_email("user@example.com"),
            "user собака example точка ком"
        );
    }

    #[test]
    fn test_email_ru_tld() {
        let (_, nn) = mk_normalizer();
        let n = norm_no_en(&nn);
        assert_eq!(
            n.normalize_email("test@mail.ru"),
            "test собака mail точка ру"
        );
    }

    #[test]
    fn test_email_dot_in_local() {
        let (_, nn) = mk_normalizer();
        let n = norm_no_en(&nn);
        assert_eq!(
            n.normalize_email("john.doe@company.org"),
            "john точка doe собака company точка орг"
        );
    }

    #[test]
    fn test_email_no_tld() {
        let (_, nn) = mk_normalizer();
        let n = norm_no_en(&nn);
        assert_eq!(
            n.normalize_email("admin@localhost"),
            "admin собака localhost"
        );
    }

    #[test]
    fn test_email_subdomain() {
        let (_, nn) = mk_normalizer();
        let n = norm_no_en(&nn);
        assert_eq!(
            n.normalize_email("support@sub.domain.com"),
            "support собака sub точка domain точка ком"
        );
    }

    #[test]
    fn test_email_with_numbers_and_underscore() {
        let (_, nn) = mk_normalizer();
        let n = norm_no_en(&nn);
        assert_eq!(
            n.normalize_email("name_123@test.io"),
            "name андерскор сто двадцать три собака test точка ай оу"
        );
    }

    #[test]
    fn test_email_with_hyphen() {
        let (_, nn) = mk_normalizer();
        let n = norm_no_en(&nn);
        assert_eq!(
            n.normalize_email("info-team@company.co"),
            "info дефис team собака company точка ко"
        );
    }

    // ---- TestIPAddressNormalization ----

    #[test]
    fn test_ip_192_168_1_1() {
        let (_, nn) = mk_normalizer();
        let n = norm_no_en(&nn);
        assert_eq!(
            n.normalize_ip("192.168.1.1"),
            "сто девяносто два точка сто шестьдесят восемь точка один точка один"
        );
    }

    #[test]
    fn test_ip_127_0_0_1() {
        let (_, nn) = mk_normalizer();
        let n = norm_no_en(&nn);
        assert_eq!(
            n.normalize_ip("127.0.0.1"),
            "сто двадцать семь точка ноль точка ноль точка один"
        );
    }

    #[test]
    fn test_ip_10_0_0_1() {
        let (_, nn) = mk_normalizer();
        let n = norm_no_en(&nn);
        assert_eq!(
            n.normalize_ip("10.0.0.1"),
            "десять точка ноль точка ноль точка один"
        );
    }

    #[test]
    fn test_ip_255_255_255_0() {
        let (_, nn) = mk_normalizer();
        let n = norm_no_en(&nn);
        assert_eq!(
            n.normalize_ip("255.255.255.0"),
            "двести пятьдесят пять точка двести пятьдесят пять точка двести пятьдесят пять точка ноль"
        );
    }

    #[test]
    fn test_ip_8_8_8_8() {
        let (_, nn) = mk_normalizer();
        let n = norm_no_en(&nn);
        assert_eq!(
            n.normalize_ip("8.8.8.8"),
            "восемь точка восемь точка восемь точка восемь"
        );
    }

    #[test]
    fn test_ip_172_16_0_1() {
        let (_, nn) = mk_normalizer();
        let n = norm_no_en(&nn);
        assert_eq!(
            n.normalize_ip("172.16.0.1"),
            "сто семьдесят два точка шестнадцать точка ноль точка один"
        );
    }

    // ---- TestFilePathNormalization ----

    #[test]
    fn test_path_unix_home_user_file_txt() {
        let (_, nn) = mk_normalizer();
        let n = norm_no_en(&nn);
        assert_eq!(
            n.normalize_filepath("/home/user/file.txt"),
            "слэш home слэш user слэш file точка txt"
        );
    }

    #[test]
    fn test_path_nginx_conf() {
        let (_, nn) = mk_normalizer();
        let n = norm_no_en(&nn);
        assert_eq!(
            n.normalize_filepath("/etc/nginx/nginx.conf"),
            "слэш etc слэш nginx слэш nginx точка conf"
        );
    }

    #[test]
    fn test_path_var_log_syslog() {
        let (_, nn) = mk_normalizer();
        let n = norm_no_en(&nn);
        assert_eq!(
            n.normalize_filepath("/var/log/syslog"),
            "слэш var слэш log слэш syslog"
        );
    }

    #[test]
    fn test_path_tilde_documents() {
        let (_, nn) = mk_normalizer();
        let n = norm_no_en(&nn);
        assert_eq!(
            n.normalize_filepath("~/Documents/report.pdf"),
            "тильда слэш Documents слэш report точка pdf"
        );
    }

    #[test]
    fn test_path_tilde_config_hidden() {
        let (_, nn) = mk_normalizer();
        let n = norm_no_en(&nn);
        assert_eq!(
            n.normalize_filepath("~/.config/settings.json"),
            "тильда слэш точка config слэш settings точка json"
        );
    }

    #[test]
    fn test_path_relative_dot_slash() {
        let (_, nn) = mk_normalizer();
        let n = norm_no_en(&nn);
        assert_eq!(
            n.normalize_filepath("./src/main.py"),
            "точка слэш src слэш main точка py"
        );
    }

    #[test]
    fn test_path_relative_parent() {
        let (_, nn) = mk_normalizer();
        let n = norm_no_en(&nn);
        assert_eq!(
            n.normalize_filepath("../config/app.yaml"),
            "точка точка слэш config слэш app точка yaml"
        );
    }

    #[test]
    fn test_path_windows_c() {
        let (_, nn) = mk_normalizer();
        let n = norm_no_en(&nn);
        assert_eq!(
            n.normalize_filepath("C:\\Users\\Admin\\file.txt"),
            "си двоеточие бэкслэш Users бэкслэш Admin бэкслэш file точка txt"
        );
    }

    #[test]
    fn test_path_windows_d() {
        let (_, nn) = mk_normalizer();
        let n = norm_no_en(&nn);
        assert_eq!(
            n.normalize_filepath("D:\\Projects\\code\\main.py"),
            "ди двоеточие бэкслэш Projects бэкслэш code бэкслэш main точка py"
        );
    }

    // ---- TestFileExtensions ----

    #[test]
    fn test_filename_main_py() {
        let (_, nn) = mk_normalizer();
        let n = norm_no_en(&nn);
        assert_eq!(n.normalize_filepath("main.py"), "main точка py");
    }

    #[test]
    fn test_filename_index_js() {
        let (_, nn) = mk_normalizer();
        let n = norm_no_en(&nn);
        assert_eq!(n.normalize_filepath("index.js"), "index точка js");
    }

    #[test]
    fn test_filename_styles_css() {
        let (_, nn) = mk_normalizer();
        let n = norm_no_en(&nn);
        assert_eq!(n.normalize_filepath("styles.css"), "styles точка css");
    }

    #[test]
    fn test_filename_config_yaml() {
        let (_, nn) = mk_normalizer();
        let n = norm_no_en(&nn);
        assert_eq!(n.normalize_filepath("config.yaml"), "config точка yaml");
    }

    #[test]
    fn test_filename_data_json() {
        let (_, nn) = mk_normalizer();
        let n = norm_no_en(&nn);
        assert_eq!(n.normalize_filepath("data.json"), "data точка json");
    }

    #[test]
    fn test_filename_readme_md() {
        let (_, nn) = mk_normalizer();
        let n = norm_no_en(&nn);
        assert_eq!(n.normalize_filepath("README.md"), "README точка md");
    }

    #[test]
    fn test_filename_dockerfile_no_ext() {
        let (_, nn) = mk_normalizer();
        let n = norm_no_en(&nn);
        assert_eq!(n.normalize_filepath("Dockerfile"), "Dockerfile");
    }

    #[test]
    fn test_filename_docker_compose_yml() {
        let (_, nn) = mk_normalizer();
        let n = norm_no_en(&nn);
        assert_eq!(
            n.normalize_filepath("docker-compose.yml"),
            "docker дефис compose точка yml"
        );
    }

    #[test]
    fn test_filename_gitignore() {
        let (_, nn) = mk_normalizer();
        let n = norm_no_en(&nn);
        assert_eq!(n.normalize_filepath(".gitignore"), "точка gitignore");
    }

    #[test]
    fn test_filename_dot_env() {
        let (_, nn) = mk_normalizer();
        let n = norm_no_en(&nn);
        assert_eq!(n.normalize_filepath(".env"), "точка env");
    }

    #[test]
    fn test_filename_test_spec_ts() {
        let (_, nn) = mk_normalizer();
        let n = norm_no_en(&nn);
        assert_eq!(
            n.normalize_filepath("test.spec.ts"),
            "test точка spec точка ts"
        );
    }

    // ---- TestComplexURLs ----

    #[test]
    fn test_complex_url_query_params() {
        let (_, nn) = mk_normalizer();
        let n = norm_no_en(&nn);
        let result = n.normalize_url("https://example.com/search?q=test");
        for part in &["example", "search", "q", "test"] {
            assert!(
                result.contains(part),
                "missing '{}' in '{}'",
                part,
                result
            );
        }
    }

    #[test]
    fn test_complex_url_fragment() {
        let (_, nn) = mk_normalizer();
        let n = norm_no_en(&nn);
        let result = n.normalize_url("https://docs.example.com/guide#installation");
        for part in &["docs", "guide", "installation"] {
            assert!(result.contains(part), "missing '{}' in '{}'", part, result);
        }
    }

    #[test]
    fn test_complex_url_multiple_path_segments() {
        let (_, nn) = mk_normalizer();
        let n = norm_no_en(&nn);
        let result = n.normalize_url("https://api.example.com/v1/users/123/posts");
        for part in &["api", "v1", "users", "posts"] {
            assert!(result.contains(part), "missing '{}' in '{}'", part, result);
        }
    }

    // ---- Test with EnglishNormalizer (transliteration enabled) ----

    #[test]
    fn test_url_with_english_normalizer_transliterates() {
        let (en, nn) = mk_normalizer();
        let n = norm(&en, &nn);
        let result = n.normalize_url("https://github.com/user/repo");
        // With English normalizer, host parts should be transliterated.
        // "github" → "гисуб" (transliteration), TLD "com" → "ком"
        assert!(result.starts_with("эйч ти ти пи эс"));
        assert!(result.contains("ком"));
    }
}
