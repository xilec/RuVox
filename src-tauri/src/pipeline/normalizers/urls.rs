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

/// Whether the label is a known TLD — used by the pipeline's scheme-less URL
/// detection to keep filenames and versions from matching.
pub(crate) fn is_known_tld(label: &str) -> bool {
    lookup_tld(label).is_some()
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

fn hex_val(b: u8) -> u8 {
    match b {
        b'0'..=b'9' => b - b'0',
        b'a'..=b'f' => b - b'a' + 10,
        b'A'..=b'F' => b - b'A' + 10,
        _ => unreachable!("callers check is_ascii_hexdigit first"),
    }
}

/// Punctuation a percent-decoded URL component may yield. Readings follow
/// the URL context ('/' is "слэш", not "делить"); '.', '-', '_' are absent
/// because the regular chunk reading already handles them.
const DECODED_PUNCT: &[(char, &str)] = &[
    ('/', "слэш"),
    ('?', "вопросительный знак"),
    ('#', "решётка"),
    ('&', "амперсанд"),
    ('=', "равно"),
    (':', "двоеточие"),
    ('@', "собака"),
    ('(', "открывающая скобка"),
    (')', "закрывающая скобка"),
    (',', "запятая"),
    (';', "точка с запятой"),
    ('\'', "апостроф"),
    ('"', "кавычка"),
    ('<', "меньше"),
    ('>', "больше"),
    ('*', "умножить"),
    ('|', "пайп"),
    ('\\', "бэкслэш"),
    ('$', "доллар"),
    ('!', "восклицательный знак"),
    ('~', "тильда"),
    ('^', "каретка"),
    ('[', "открывающая квадратная скобка"),
    (']', "закрывающая квадратная скобка"),
    ('{', "открывающая фигурная скобка"),
    ('}', "закрывающая фигурная скобка"),
    ('`', "обратная кавычка"),
];

/// Decode percent-encoded bytes (`%XX`) in a single URL component. A maximal
/// run of `%XX` triples decodes as one UTF-8 string; a run that is not valid
/// UTF-8 is kept verbatim so the caller reads each leftover '%' as "процент".
/// A '%' without two hex digits after it is kept as-is. When `plus_as_space`
/// is set (query components, form-urlencoded), '+' becomes a space;
/// otherwise it is kept for the caller to read as "плюс".
fn percent_decode(input: &str, plus_as_space: bool) -> String {
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(input.len());
    let mut i = 0;
    while i < bytes.len() {
        let is_triple = bytes[i] == b'%'
            && i + 2 < bytes.len()
            && bytes[i + 1].is_ascii_hexdigit()
            && bytes[i + 2].is_ascii_hexdigit();
        if is_triple {
            let mut run: Vec<u8> = Vec::new();
            while i + 2 < bytes.len()
                && bytes[i] == b'%'
                && bytes[i + 1].is_ascii_hexdigit()
                && bytes[i + 2].is_ascii_hexdigit()
            {
                run.push(hex_val(bytes[i + 1]) * 16 + hex_val(bytes[i + 2]));
                i += 3;
            }
            match std::str::from_utf8(&run) {
                Ok(s) => out.push_str(s),
                Err(_) => {
                    for byte in run {
                        out.push('%');
                        out.push(char::from_digit((byte >> 4) as u32, 16).expect("nibble < 16"));
                        out.push(char::from_digit((byte & 0xF) as u32, 16).expect("nibble < 16"));
                    }
                }
            }
        } else if bytes[i] == b'+' && plus_as_space {
            out.push(' ');
            i += 1;
        } else {
            let ch = input[i..].chars().next().expect("i is on a char boundary");
            out.push(ch);
            i += ch.len_utf8();
        }
    }
    out
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
                if !word.is_ascii() {
                    // Non-ASCII (e.g. Cyrillic) segments are already readable;
                    // only wiki-style underscores need to become spaces.
                    return word.replace('_', " ");
                }
                if !word.chars().any(|c| c.is_alphabetic()) {
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

    /// Transliterate a separator-free piece, splitting embedded digit runs
    /// into number words ("v1" → "ви один"). Digits must not leak into the
    /// output: later pipeline phases skip already-replaced regions, so a
    /// digit left here would reach the TTS engine as-is.
    fn transliterate_runs(&self, part: &str) -> String {
        let mut runs: Vec<String> = Vec::new();
        let mut run = String::new();
        let mut run_is_digit = false;
        for c in part.chars() {
            let is_digit = c.is_ascii_digit();
            if !run.is_empty() && is_digit != run_is_digit {
                runs.push(std::mem::take(&mut run));
            }
            run.push(c);
            run_is_digit = is_digit;
        }
        if !run.is_empty() {
            runs.push(run);
        }

        runs.iter()
            .map(|r| {
                if r.bytes().all(|b| b.is_ascii_digit()) {
                    self.numbers.normalize_number(r)
                } else {
                    self.transliterate_word(r)
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn transliterate_segment(&self, segment: &str) -> String {
        if segment.is_empty() {
            return segment.to_string();
        }
        // Split on hyphens and underscores (wiki-style spaces); each piece is
        // then split into digit runs (number words) and alphabetic runs
        // (transliteration) by transliterate_runs.
        segment
            .split(['-', '_'])
            .filter(|part| !part.is_empty())
            .map(|part| self.transliterate_runs(part))
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Render a URL component (path segment, query key/value, fragment):
    /// percent-decode it, then read the decoded text. In query components
    /// `+` means a space (form-urlencoded); elsewhere a literal `+` is read
    /// as "плюс". Decoding happens after the structural splits, so a decoded
    /// "%2F" never becomes a path separator.
    fn read_component(&self, component: &str, plus_as_space: bool) -> String {
        self.read_decoded_text(&percent_decode(component, plus_as_space))
    }

    /// Read already-decoded component text: leftover '%' (invalid sequences)
    /// is read as "процент", '+' as "плюс", whitespace separates words,
    /// decoded punctuation is read via `DECODED_PUNCT`; other chunks keep
    /// the historical dot / hyphen / digit-run reading.
    fn read_decoded_text(&self, text: &str) -> String {
        let mut words: Vec<String> = Vec::new();
        let mut chunk = String::new();
        for ch in text.chars() {
            let marker = match ch {
                '%' => Some("процент"),
                '+' => Some("плюс"),
                // Decoded whitespace is a word separator; other control
                // characters (%00-%1F, %7F) are dropped — they carry no
                // reading and must not reach TTS.
                c if c.is_whitespace() || c.is_control() => Some(""),
                c => DECODED_PUNCT.iter().find(|(p, _)| *p == c).map(|(_, w)| *w),
            };
            match marker {
                Some(word) => {
                    if !chunk.is_empty() {
                        words.push(self.read_chunk(std::mem::take(&mut chunk).as_str()));
                    }
                    if !word.is_empty() {
                        words.push(word.to_string());
                    }
                }
                None => chunk.push(ch),
            }
        }
        if !chunk.is_empty() {
            words.push(self.read_chunk(&chunk));
        }
        words
            .into_iter()
            .filter(|w| !w.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Read a chunk free of '%' / '+' / whitespace: dotted pieces are joined
    /// with "точка" (all-digit pieces as number words), the rest goes
    /// through transliterate_segment.
    fn read_chunk(&self, chunk: &str) -> String {
        if chunk.contains('.') {
            chunk
                .split('.')
                .map(|sp| {
                    if !sp.is_empty() && sp.chars().all(|c| c.is_ascii_digit()) {
                        self.numbers.normalize_number(sp)
                    } else {
                        self.transliterate_segment(sp)
                    }
                })
                .collect::<Vec<_>>()
                .join(" точка ")
        } else {
            self.transliterate_segment(chunk)
        }
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
        parts.extend(self.render_host_and_tail(rest));

        parts.join(" ")
    }

    /// Normalize a scheme-less URL ("www.example.com", "example.com/path").
    ///
    /// Rendered exactly like `normalize_url` minus the protocol prefix, so
    /// transliteration, digit runs, and punctuation wording stay identical.
    pub fn normalize_schemeless(&self, url: &str) -> String {
        self.render_host_and_tail(url).join(" ")
    }

    /// Render "host[:port][/path][?query][#fragment]" (everything after the
    /// scheme) into speakable parts.
    fn render_host_and_tail(&self, rest: &str) -> Vec<String> {
        let mut parts: Vec<String> = Vec::new();

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

        // Domain parts with TLD handling. Labels are percent-decoded; any
        // label the decoder changed (or one carrying a raw '%' / '+' from an
        // invalid sequence) is read as component text — TLD lookup no longer
        // applies to it.
        let domain_parts: Vec<&str> = host.split('.').collect();
        let last = domain_parts.len() - 1;
        let domain_words: Vec<String> = domain_parts
            .iter()
            .enumerate()
            .map(|(i, part)| {
                let decoded = percent_decode(part, false);
                if decoded != *part || decoded.contains(['%', '+']) {
                    return self.read_decoded_text(&decoded);
                }
                if i == last {
                    if let Some(tld) = lookup_tld(&decoded) {
                        return tld.to_string();
                    }
                }
                // transliterate_runs also splits digit runs ("s3" → "эс три").
                self.transliterate_runs(&decoded)
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
                parts.push(self.read_component(segment, false));
            }
        }

        // Query parameters (simplified — key=value pairs). "+" means a
        // space here (form-urlencoded).
        if !query_str.is_empty() {
            parts.push("вопросительный знак".to_string());
            for qp in query_str.split('&') {
                if let Some(eq_pos) = qp.find('=') {
                    let key = &qp[..eq_pos];
                    let value = &qp[eq_pos + 1..];
                    parts.push(self.read_component(key, true));
                    parts.push("равно".to_string());
                    parts.push(self.read_component(value, true));
                }
            }
        }

        // Fragment
        if !fragment_str.is_empty() {
            parts.push("решётка".to_string());
            parts.push(self.read_component(fragment_str, false));
        }

        parts
    }

    fn normalize_identifier(&self, identifier: &str) -> String {
        let mut result: Vec<String> = Vec::new();
        let mut current_word = String::new();
        let mut chars = identifier.chars().peekable();

        while let Some(ch) = chars.next() {
            match ch {
                '.' => {
                    if !current_word.is_empty() {
                        result.push(self.transliterate_word(&current_word));
                        current_word.clear();
                    }
                    result.push("точка".to_string());
                }
                '_' => {
                    if !current_word.is_empty() {
                        result.push(self.transliterate_word(&current_word));
                        current_word.clear();
                    }
                    result.push("андерскор".to_string());
                }
                '-' => {
                    if !current_word.is_empty() {
                        result.push(self.transliterate_word(&current_word));
                        current_word.clear();
                    }
                    result.push("дефис".to_string());
                }
                '+' => {
                    if !current_word.is_empty() {
                        result.push(self.transliterate_word(&current_word));
                        current_word.clear();
                    }
                    result.push("плюс".to_string());
                }
                '%' => {
                    // Leftover from an invalid percent sequence.
                    if !current_word.is_empty() {
                        result.push(self.transliterate_word(&current_word));
                        current_word.clear();
                    }
                    result.push("процент".to_string());
                }
                c if c.is_whitespace() => {
                    // Decoded "%20": a plain word separator.
                    if !current_word.is_empty() {
                        result.push(self.transliterate_word(&current_word));
                        current_word.clear();
                    }
                }
                c if c.is_ascii_digit() => {
                    if !current_word.is_empty() {
                        result.push(self.transliterate_word(&current_word));
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
                    if let Some((_, word)) = DECODED_PUNCT.iter().find(|(p, _)| *p == other) {
                        // Decoded punctuation (e.g. "%28" → '(') is read, not leaked.
                        if !current_word.is_empty() {
                            result.push(self.transliterate_word(&current_word));
                            current_word.clear();
                        }
                        result.push(word.to_string());
                    } else if other.is_control() {
                        // Decoded control character (%00-%1F, %7F): dropped,
                        // it carries no reading and must not reach TTS.
                        if !current_word.is_empty() {
                            result.push(self.transliterate_word(&current_word));
                            current_word.clear();
                        }
                    } else {
                        current_word.push(other);
                    }
                }
            }
        }

        if !current_word.is_empty() {
            result.push(self.transliterate_word(&current_word));
        }

        result.join(" ")
    }

    pub fn normalize_email(&self, email: &str) -> String {
        if email.is_empty() || !email.contains('@') {
            return email.to_string();
        }

        let at_pos = email.rfind('@').unwrap();
        let local_part = percent_decode(&email[..at_pos], false);
        let domain = &email[at_pos + 1..];

        let mut parts: Vec<String> = Vec::new();

        parts.push(self.normalize_identifier(&local_part));
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
                // transliterate_runs also splits digit runs ("mail.123.com").
                self.transliterate_runs(part)
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
        // Hyphens are read as "дефис", underscores become spaces (as in URL
        // segments); each piece is transliterated with digit runs split into
        // number words.
        part.split('-')
            .map(|p| {
                p.split('_')
                    .filter(|s| !s.is_empty())
                    .map(|s| self.transliterate_runs(s))
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" дефис ")
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
                    parts.push(self.transliterate_runs(ext));
                } else {
                    parts.push(self.normalize_filename_part(rest));
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

    #[test_case("https://example.com" => "эйч ти ти пи эс двоеточие слэш слэш example точка ком"; "com")]
    #[test_case("https://example.org" => "эйч ти ти пи эс двоеточие слэш слэш example точка орг"; "org")]
    #[test_case("https://example.net" => "эйч ти ти пи эс двоеточие слэш слэш example точка нет"; "net")]
    #[test_case("https://example.ru" => "эйч ти ти пи эс двоеточие слэш слэш example точка ру"; "ru")]
    #[test_case("https://example.io" => "эйч ти ти пи эс двоеточие слэш слэш example точка ай оу"; "io")]
    #[test_case("https://example.dev" => "эйч ти ти пи эс двоеточие слэш слэш example точка дев"; "dev")]
    #[test_case("https://example.app" => "эйч ти ти пи эс двоеточие слэш слэш example точка апп"; "app")]
    #[test_case("https://example.ai" => "эйч ти ти пи эс двоеточие слэш слэш example точка эй ай"; "ai")]
    #[test_case("https://example.co" => "эйч ти ти пи эс двоеточие слэш слэш example точка ко"; "co")]
    #[test_case("https://example.me" => "эйч ти ти пи эс двоеточие слэш слэш example точка ми"; "me")]
    fn tld(url: &str) -> String {
        let (_, nn) = mk_normalizer();
        norm_no_en(&nn).normalize_url(url)
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

    // ---- Percent-decoding and '+' (change normalize-url-encoding) ----

    #[test_case("hello%20world", false => "hello world"; "encoded_space")]
    #[test_case("%D1%84%D0%B0%D0%B9%D0%BB", false => "файл"; "cyrillic_utf8_run")]
    #[test_case("a%2Bb", false => "a+b"; "encoded_plus_stays_literal")]
    #[test_case("a+b", true => "a b"; "plus_as_space_in_query")]
    #[test_case("a+b", false => "a+b"; "plus_kept_outside_query")]
    #[test_case("100%25", false => "100%"; "encoded_percent_sign")]
    #[test_case("done%2", false => "done%2"; "truncated_sequence_kept")]
    #[test_case("%ZZ", false => "%ZZ"; "non_hex_kept")]
    #[test_case("%FF%FE", false => "%ff%fe"; "invalid_utf8_run_kept")]
    fn percent_decode_cases(input: &str, plus_as_space: bool) -> String {
        percent_decode(input, plus_as_space)
    }

    #[test_case("https://example.com/hello%20world" => "эйч ти ти пи эс двоеточие слэш слэш экзампл точка ком слэш хеллоу ворлд"; "encoded_space_in_path")]
    #[test_case("https://example.com/%D1%84%D0%B0%D0%B9%D0%BB" => "эйч ти ти пи эс двоеточие слэш слэш экзампл точка ком слэш файл"; "encoded_cyrillic_name")]
    #[test_case("https://example.com/search?q=hello+world" => "эйч ти ти пи эс двоеточие слэш слэш экзампл точка ком слэш сирч вопросительный знак к равно хеллоу ворлд"; "plus_in_query_is_space")]
    #[test_case("https://example.com/a+b" => "эйч ти ти пи эс двоеточие слэш слэш экзампл точка ком слэш а плюс б"; "plus_in_path_is_word")]
    #[test_case("https://example.com/100%25done%2" => "эйч ти ти пи эс двоеточие слэш слэш экзампл точка ком слэш сто процент доне процент два"; "encoded_percent_and_truncated")]
    #[test_case("https://example.com/x%2Fy" => "эйч ти ти пи эс двоеточие слэш слэш экзампл точка ком слэш кс слэш и"; "decoded_slash_is_not_path_separator")]
    #[test_case("https://example.com/%FF%FE" => "эйч ти ти пи эс двоеточие слэш слэш экзампл точка ком слэш процент фф процент фе"; "invalid_utf8_run")]
    #[test_case("https://example.com/x#a+b" => "эйч ти ти пи эс двоеточие слэш слэш экзампл точка ком слэш кс решётка а плюс б"; "plus_in_fragment_is_word")]
    #[test_case("https://exa%2Emple.com/x" => "эйч ти ти пи эс двоеточие слэш слэш екса точка мпле точка ком слэш кс"; "encoded_dot_in_host_label")]
    fn url_percent_decoding(input: &str) -> String {
        let (en, nn) = mk_normalizer();
        norm(&en, &nn).normalize_url(input)
    }

    #[test_case("user+tag@example.com" => "юзер плюс таг собака экзампл точка ком"; "plus_in_local_part")]
    #[test_case("user%20name@example.com" => "юзер наме собака экзампл точка ком"; "encoded_space_in_local_part")]
    #[test_case("user%28tag@example.com" => "юзер открывающая скобка таг собака экзампл точка ком"; "decoded_punctuation_in_local_part")]
    fn email_percent_decoding(input: &str) -> String {
        let (en, nn) = mk_normalizer();
        norm(&en, &nn).normalize_email(input)
    }

    #[test]
    fn url_decoding_leaves_no_special_chars() {
        let (en, nn) = mk_normalizer();
        let n = norm(&en, &nn);
        for input in [
            "https://example.com/hello%20world",
            "https://example.com/%D1%84%D0%B0%D0%B9%D0%BB",
            "https://example.com/search?q=hello+world&lang=ru",
            "https://example.com/100%25done%2",
            "https://example.com/%FF%FE",
            "https://example.com/x%2Fy?q=a%3Db%26c",
            "https://example.com/a%28b%29",
            "https://example.com/a%0Ab%00c",
        ] {
            let out = n.normalize_url(input);
            assert!(
                !out.contains(['%', '+', '/', '?', '#', '&', '=', '(', ')'])
                    && !out.chars().any(|c| c.is_control()),
                "special char leak in {input:?}: {out:?}"
            );
        }
        let out = n.normalize_email("user+tag@example.com");
        assert!(!out.contains(['%', '+']), "special char leak: {out:?}");
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

    // ---- Complex URLs (query, fragment, multi-segment paths) ----
    //
    // In no-English mode alphabetic words pass through verbatim, but digit
    // runs still become number words everywhere: later pipeline phases skip
    // already-replaced regions, so anything left here would never be picked
    // up downstream.

    #[test_case("https://example.com/search?q=test" => "эйч ти ти пи эс двоеточие слэш слэш example точка ком слэш search вопросительный знак q равно test"; "query_params")]
    #[test_case("https://docs.example.com/guide#installation" => "эйч ти ти пи эс двоеточие слэш слэш docs точка example точка ком слэш guide решётка installation"; "fragment")]
    #[test_case("https://api.example.com/v1/users/123/posts" => "эйч ти ти пи эс двоеточие слэш слэш api точка example точка ком слэш v один слэш users слэш сто двадцать три слэш posts"; "multiple_path_segments")]
    fn complex_url(url: &str) -> String {
        let (_, nn) = mk_normalizer();
        norm_no_en(&nn).normalize_url(url)
    }

    // ---- With EnglishNormalizer (transliteration enabled) ----
    //
    // This is the production path: the pipeline always builds the normalizer
    // with `EnglishNormalizer`, and later phases skip the replaced URL region,
    // so the output here must already be fully speakable (no Latin, no digits).

    #[test]
    fn url_with_english_normalizer_transliterates() {
        let (en, nn) = mk_normalizer();
        let n = norm(&en, &nn);
        let result = n.normalize_url("https://github.com/user/repo");
        // With the English normalizer enabled every English segment is
        // transliterated: github → гитхаб, user → юзер, repo → репо.
        assert_eq!(
            result,
            "эйч ти ти пи эс двоеточие слэш слэш гитхаб точка ком слэш юзер слэш репо"
        );
    }

    #[test_case("https://example.com/search?q=test&lang=ru" => "эйч ти ти пи эс двоеточие слэш слэш экзампл точка ком слэш сирч вопросительный знак к равно тест ланг равно ру"; "query_key_value_transliterated")]
    #[test_case("https://example.com/#section-2" => "эйч ти ти пи эс двоеточие слэш слэш экзампл точка ком решётка секшн два"; "fragment_transliterated")]
    #[test_case("https://api.example.com/v1/users/123/posts" => "эйч ти ти пи эс двоеточие слэш слэш эй пи ай точка экзампл точка ком слэш в один слэш юзерс слэш сто двадцать три слэш постс"; "digit_runs_in_path_segments")]
    #[test_case("https://ru.wikipedia.org/wiki/Заглавная_страница" => "эйч ти ти пи эс двоеточие слэш слэш ру точка википедиа точка орг слэш вики слэш Заглавная страница"; "cyrillic_segment_underscore_to_space")]
    #[test_case("https://s3.amazonaws.com" => "эйч ти ти пи эс двоеточие слэш слэш с три точка амазонос точка ком"; "digit_run_in_host_label")]
    fn url_production_path(url: &str) -> String {
        let (en, nn) = mk_normalizer();
        norm(&en, &nn).normalize_url(url)
    }

    #[test_case("john.doe@company.org" => "джохн точка дое собака компани точка орг"; "email_local_and_domain")]
    #[test_case("name_123@test.io" => "наме андерскор сто двадцать три собака тест точка ай оу"; "email_underscore_and_digits")]
    #[test_case("john2doe@test.com" => "джохн два дое собака тест точка ком"; "email_digit_run_in_word")]
    #[test_case("user@mail.123.com" => "юзер собака мэйл точка сто двадцать три точка ком"; "email_digit_only_domain_label")]
    fn email_production_path(email: &str) -> String {
        let (en, nn) = mk_normalizer();
        norm(&en, &nn).normalize_email(email)
    }

    #[test_case("/home/user/file.txt" => "слэш хоум слэш юзер слэш файл точка тэкст"; "unix_path")]
    #[test_case("docker-compose.yml" => "докер дефис компосе точка имл"; "hyphenated_filename")]
    #[test_case("~/my_file.txt" => "тильда слэш ми файл точка тэкст"; "underscore_in_filename")]
    fn filepath_production_path(path: &str) -> String {
        let (en, nn) = mk_normalizer();
        norm(&en, &nn).normalize_filepath(path)
    }

    // ---- Scheme-less URLs (normalize_schemeless) ----

    #[test_case("www.example.com" => "ввв точка экзампл точка ком"; "www_prefixed")]
    #[test_case("example.com" => "экзампл точка ком"; "bare_domain")]
    #[test_case("docs.python.org/3/tutorial" => "докс точка пайтон точка орг слэш три слэш тьюториал"; "bare_domain_with_path")]
    #[test_case("example.com/search?q=test&lang=ru" => "экзампл точка ком слэш сирч вопросительный знак к равно тест ланг равно ру"; "bare_domain_with_query")]
    #[test_case("example.com/#section-2" => "экзампл точка ком решётка секшн два"; "bare_domain_with_fragment")]
    #[test_case("api.example.com/v1/users/123" => "эй пи ай точка экзампл точка ком слэш в один слэш юзерс слэш сто двадцать три"; "digit_runs_in_segments")]
    fn schemeless_production_path(url: &str) -> String {
        let (en, nn) = mk_normalizer();
        norm(&en, &nn).normalize_schemeless(url)
    }
}
