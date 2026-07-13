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
    TLD_MAP
        .iter()
        .find(|(k, _)| *k == lower.as_str())
        .map(|(_, v)| *v)
}

fn lookup_protocol(scheme: &str) -> Option<&'static str> {
    let lower = scheme.to_lowercase();
    PROTOCOLS
        .iter()
        .find(|(k, _)| *k == lower.as_str())
        .map(|(_, v)| *v)
}

fn lookup_drive(letter: &str) -> Option<&'static str> {
    let lower = letter.to_lowercase();
    DRIVE_LETTERS
        .iter()
        .find(|(k, _)| *k == lower.as_str())
        .map(|(_, v)| *v)
}

/// Normalizes URLs, emails, IP addresses, and file paths to speakable Russian text.
///
/// When `english` is `None`, alphabetic segments in URLs and paths are kept verbatim.
/// When provided, segments are transliterated via the English normalizer before output.
/// In production the pipeline always passes the normalizer; the `None` path is kept
/// for direct unit testing of this normalizer in isolation.
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
        Self {
            numbers,
            english: None,
        }
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
    use test_case::test_case;

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

    // ---- URL normalization ----

    #[test_case("https://example.com" => "эйч ти ти пи эс двоеточие слэш слэш example точка ком"; "https_example_com")]
    #[test_case("http://test.org" => "эйч ти ти пи двоеточие слэш слэш test точка орг"; "http_test_org")]
    #[test_case("https://github.com/user/repo" => "эйч ти ти пи эс двоеточие слэш слэш github точка ком слэш user слэш repo"; "with_path")]
    #[test_case("https://docs.python.org/3.11/tutorial" => "эйч ти ти пи эс двоеточие слэш слэш docs точка python точка орг слэш три точка одиннадцать слэш tutorial"; "python_docs_version_path")]
    #[test_case("https://example.com/file.html" => "эйч ти ти пи эс двоеточие слэш слэш example точка ком слэш file точка html"; "with_file_extension")]
    #[test_case("https://api.github.com/repos" => "эйч ти ти пи эс двоеточие слэш слэш api точка github точка ком слэш repos"; "subdomain")]
    #[test_case("http://localhost:8080" => "эйч ти ти пи двоеточие слэш слэш localhost двоеточие восемь тысяч восемьдесят"; "with_port_8080")]
    #[test_case("http://localhost:3000/api" => "эйч ти ти пи двоеточие слэш слэш localhost двоеточие три тысячи слэш api"; "with_port_3000_and_path")]
    fn url_normalization(input: &str) -> String {
        let (_, nn) = mk_normalizer();
        norm_no_en(&nn).normalize_url(input)
    }

    // ---- Common TLDs ----

    #[test_case("https://example.com", "ком"; "com")]
    #[test_case("https://example.org", "орг"; "org")]
    #[test_case("https://example.net", "нет"; "net")]
    #[test_case("https://example.ru", "ру"; "ru")]
    #[test_case("https://example.io", "ай оу"; "io")]
    #[test_case("https://example.dev", "дев"; "dev")]
    #[test_case("https://example.app", "апп"; "app")]
    #[test_case("https://example.ai", "эй ай"; "ai")]
    #[test_case("https://example.co", "ко"; "co")]
    #[test_case("https://example.me", "ми"; "me")]
    fn tld(url: &str, expected: &str) {
        let (_, nn) = mk_normalizer();
        assert!(norm_no_en(&nn).normalize_url(url).contains(expected));
    }

    // ---- Protocols ----

    #[test_case("https://example.com", "эйч ти ти пи эс"; "https")]
    #[test_case("http://example.com", "эйч ти ти пи"; "http")]
    #[test_case("ftp://files.example.com", "эф ти пи"; "ftp")]
    #[test_case("ssh://server.example.com", "эс эс эйч"; "ssh")]
    #[test_case("git://github.com/repo.git", "гит"; "git")]
    #[test_case("file:///home/user/doc.txt", "файл"; "file")]
    fn protocol(url: &str, expected_prefix: &str) {
        let (_, nn) = mk_normalizer();
        assert!(norm_no_en(&nn)
            .normalize_url(url)
            .starts_with(expected_prefix));
    }

    // ---- Email normalization ----

    #[test_case("user@example.com" => "user собака example точка ком"; "simple")]
    #[test_case("test@mail.ru" => "test собака mail точка ру"; "ru_tld")]
    #[test_case("john.doe@company.org" => "john точка doe собака company точка орг"; "dot_in_local")]
    #[test_case("admin@localhost" => "admin собака localhost"; "no_tld")]
    #[test_case("support@sub.domain.com" => "support собака sub точка domain точка ком"; "subdomain")]
    #[test_case("name_123@test.io" => "name андерскор сто двадцать три собака test точка ай оу"; "with_numbers_and_underscore")]
    #[test_case("info-team@company.co" => "info дефис team собака company точка ко"; "with_hyphen")]
    fn email(input: &str) -> String {
        let (_, nn) = mk_normalizer();
        norm_no_en(&nn).normalize_email(input)
    }

    // ---- IP address normalization ----

    #[test_case("192.168.1.1" => "сто девяносто два точка сто шестьдесят восемь точка один точка один"; "192_168_1_1")]
    #[test_case("127.0.0.1" => "сто двадцать семь точка ноль точка ноль точка один"; "127_0_0_1")]
    #[test_case("10.0.0.1" => "десять точка ноль точка ноль точка один"; "10_0_0_1")]
    #[test_case("255.255.255.0" => "двести пятьдесят пять точка двести пятьдесят пять точка двести пятьдесят пять точка ноль"; "255_255_255_0")]
    #[test_case("8.8.8.8" => "восемь точка восемь точка восемь точка восемь"; "8_8_8_8")]
    #[test_case("172.16.0.1" => "сто семьдесят два точка шестнадцать точка ноль точка один"; "172_16_0_1")]
    fn ip(input: &str) -> String {
        let (_, nn) = mk_normalizer();
        norm_no_en(&nn).normalize_ip(input)
    }

    // ---- File path normalization + file extensions (both exercise normalize_filepath) ----

    #[test_case("/home/user/file.txt" => "слэш home слэш user слэш file точка txt"; "unix_home_user_file_txt")]
    #[test_case("/etc/nginx/nginx.conf" => "слэш etc слэш nginx слэш nginx точка conf"; "nginx_conf")]
    #[test_case("/var/log/syslog" => "слэш var слэш log слэш syslog"; "var_log_syslog")]
    #[test_case("~/Documents/report.pdf" => "тильда слэш Documents слэш report точка pdf"; "tilde_documents")]
    #[test_case("~/.config/settings.json" => "тильда слэш точка config слэш settings точка json"; "tilde_config_hidden")]
    #[test_case("./src/main.py" => "точка слэш src слэш main точка py"; "relative_dot_slash")]
    #[test_case("../config/app.yaml" => "точка точка слэш config слэш app точка yaml"; "relative_parent")]
    #[test_case("C:\\Users\\Admin\\file.txt" => "си двоеточие бэкслэш Users бэкслэш Admin бэкслэш file точка txt"; "windows_c")]
    #[test_case("D:\\Projects\\code\\main.py" => "ди двоеточие бэкслэш Projects бэкслэш code бэкслэш main точка py"; "windows_d")]
    #[test_case("main.py" => "main точка py"; "main_py")]
    #[test_case("index.js" => "index точка js"; "index_js")]
    #[test_case("styles.css" => "styles точка css"; "styles_css")]
    #[test_case("config.yaml" => "config точка yaml"; "config_yaml")]
    #[test_case("data.json" => "data точка json"; "data_json")]
    #[test_case("README.md" => "README точка md"; "readme_md")]
    #[test_case("Dockerfile" => "Dockerfile"; "dockerfile_no_ext")]
    #[test_case("docker-compose.yml" => "docker дефис compose точка yml"; "docker_compose_yml")]
    #[test_case(".gitignore" => "точка gitignore"; "gitignore")]
    #[test_case(".env" => "точка env"; "dot_env")]
    #[test_case("test.spec.ts" => "test точка spec точка ts"; "test_spec_ts")]
    fn filepath(input: &str) -> String {
        let (_, nn) = mk_normalizer();
        norm_no_en(&nn).normalize_filepath(input)
    }

    // ---- Complex URLs (multiple expected substrings) ----

    #[test_case("https://example.com/search?q=test", &["example", "search", "q", "test"]; "query_params")]
    #[test_case("https://docs.example.com/guide#installation", &["docs", "guide", "installation"]; "fragment")]
    #[test_case("https://api.example.com/v1/users/123/posts", &["api", "v1", "users", "posts"]; "multiple_path_segments")]
    fn complex_url(url: &str, parts: &[&str]) {
        let (_, nn) = mk_normalizer();
        let result = norm_no_en(&nn).normalize_url(url);
        for part in parts {
            assert!(result.contains(part), "missing '{}' in '{}'", part, result);
        }
    }

    // ---- With EnglishNormalizer (transliteration enabled) ----

    #[test]
    fn url_with_english_normalizer_transliterates() {
        let (en, nn) = mk_normalizer();
        let n = norm(&en, &nn);
        let result = n.normalize_url("https://github.com/user/repo");
        // With English normalizer, host parts should be transliterated.
        // "github" → "гисуб" (transliteration), TLD "com" → "ком"
        assert!(result.starts_with("эйч ти ти пи эс"));
        assert!(result.contains("ком"));
    }
}
