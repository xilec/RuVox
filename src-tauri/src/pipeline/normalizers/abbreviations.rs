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

    fn normalizer() -> AbbreviationNormalizer {
        AbbreviationNormalizer::new()
    }

    // --- AS_WORD tests ---

    #[test]
    fn test_json_as_word() {
        assert_eq!(normalizer().normalize("JSON"), "джейсон");
    }

    #[test]
    fn test_yaml_as_word() {
        assert_eq!(normalizer().normalize("YAML"), "ямл");
    }

    #[test]
    fn test_toml_as_word() {
        assert_eq!(normalizer().normalize("TOML"), "томл");
    }

    #[test]
    fn test_rest_as_word() {
        assert_eq!(normalizer().normalize("REST"), "рест");
    }

    #[test]
    fn test_ajax_as_word() {
        assert_eq!(normalizer().normalize("AJAX"), "эйджакс");
    }

    #[test]
    fn test_crud_as_word() {
        assert_eq!(normalizer().normalize("CRUD"), "крад");
    }

    #[test]
    fn test_cors_as_word() {
        assert_eq!(normalizer().normalize("CORS"), "корс");
    }

    #[test]
    fn test_oauth_as_word() {
        assert_eq!(normalizer().normalize("OAuth"), "о ауз");
    }

    #[test]
    fn test_gif_as_word() {
        assert_eq!(normalizer().normalize("GIF"), "гиф");
    }

    #[test]
    fn test_jpeg_as_word() {
        assert_eq!(normalizer().normalize("JPEG"), "джейпег");
    }

    #[test]
    fn test_png_spelled_out() {
        // PNG is not in AS_WORD, so it gets spelled out
        assert_eq!(normalizer().normalize("PNG"), "пи эн джи");
    }

    #[test]
    fn test_ram_as_word() {
        assert_eq!(normalizer().normalize("RAM"), "рам");
    }

    #[test]
    fn test_rom_as_word() {
        assert_eq!(normalizer().normalize("ROM"), "ром");
    }

    #[test]
    fn test_lan_as_word() {
        assert_eq!(normalizer().normalize("LAN"), "лан");
    }

    #[test]
    fn test_wan_as_word() {
        assert_eq!(normalizer().normalize("WAN"), "ван");
    }

    #[test]
    fn test_spa_as_word() {
        assert_eq!(normalizer().normalize("SPA"), "спа");
    }

    #[test]
    fn test_dom_as_word() {
        assert_eq!(normalizer().normalize("DOM"), "дом");
    }

    #[test]
    fn test_gui_as_word() {
        assert_eq!(normalizer().normalize("GUI"), "гуи");
    }

    #[test]
    fn test_imap_as_word() {
        assert_eq!(normalizer().normalize("IMAP"), "ай мап");
    }

    #[test]
    fn test_pop_as_word() {
        assert_eq!(normalizer().normalize("POP"), "поп");
    }

    // --- Spelled out tests ---

    #[test]
    fn test_http_spelled_out() {
        assert_eq!(normalizer().normalize("HTTP"), "эйч ти ти пи");
    }

    #[test]
    fn test_https_spelled_out() {
        assert_eq!(normalizer().normalize("HTTPS"), "эйч ти ти пи эс");
    }

    #[test]
    fn test_html_spelled_out() {
        assert_eq!(normalizer().normalize("HTML"), "эйч ти эм эл");
    }

    #[test]
    fn test_css_spelled_out() {
        assert_eq!(normalizer().normalize("CSS"), "си эс эс");
    }

    #[test]
    fn test_xml_spelled_out() {
        assert_eq!(normalizer().normalize("XML"), "экс эм эл");
    }

    #[test]
    fn test_url_spelled_out() {
        assert_eq!(normalizer().normalize("URL"), "ю ар эл");
    }

    #[test]
    fn test_uri_spelled_out() {
        assert_eq!(normalizer().normalize("URI"), "ю ар ай");
    }

    #[test]
    fn test_api_spelled_out() {
        assert_eq!(normalizer().normalize("API"), "эй пи ай");
    }

    #[test]
    fn test_sdk_spelled_out() {
        assert_eq!(normalizer().normalize("SDK"), "эс ди кей");
    }

    #[test]
    fn test_cli_spelled_out() {
        assert_eq!(normalizer().normalize("CLI"), "си эл ай");
    }

    #[test]
    fn test_ide_spelled_out() {
        assert_eq!(normalizer().normalize("IDE"), "ай ди и");
    }

    #[test]
    fn test_ssl_spelled_out() {
        assert_eq!(normalizer().normalize("SSL"), "эс эс эл");
    }

    #[test]
    fn test_tls_spelled_out() {
        assert_eq!(normalizer().normalize("TLS"), "ти эл эс");
    }

    #[test]
    fn test_ssh_spelled_out() {
        assert_eq!(normalizer().normalize("SSH"), "эс эс эйч");
    }

    #[test]
    fn test_vpn_spelled_out() {
        assert_eq!(normalizer().normalize("VPN"), "ви пи эн");
    }

    #[test]
    fn test_jwt_spelled_out() {
        assert_eq!(normalizer().normalize("JWT"), "джей дабл ю ти");
    }

    #[test]
    fn test_xss_spelled_out() {
        assert_eq!(normalizer().normalize("XSS"), "экс эс эс");
    }

    #[test]
    fn test_csrf_spelled_out() {
        assert_eq!(normalizer().normalize("CSRF"), "си эс ар эф");
    }

    #[test]
    fn test_tcp_spelled_out() {
        assert_eq!(normalizer().normalize("TCP"), "ти си пи");
    }

    #[test]
    fn test_udp_spelled_out() {
        assert_eq!(normalizer().normalize("UDP"), "ю ди пи");
    }

    #[test]
    fn test_ftp_spelled_out() {
        assert_eq!(normalizer().normalize("FTP"), "эф ти пи");
    }

    #[test]
    fn test_dns_spelled_out() {
        assert_eq!(normalizer().normalize("DNS"), "ди эн эс");
    }

    #[test]
    fn test_smtp_spelled_out() {
        assert_eq!(normalizer().normalize("SMTP"), "эс эм ти пи");
    }

    #[test]
    fn test_ip_spelled_out() {
        assert_eq!(normalizer().normalize("IP"), "ай пи");
    }

    #[test]
    fn test_cpu_spelled_out() {
        assert_eq!(normalizer().normalize("CPU"), "си пи ю");
    }

    #[test]
    fn test_gpu_spelled_out() {
        assert_eq!(normalizer().normalize("GPU"), "джи пи ю");
    }

    #[test]
    fn test_ssd_spelled_out() {
        assert_eq!(normalizer().normalize("SSD"), "эс эс ди");
    }

    #[test]
    fn test_hdd_spelled_out() {
        assert_eq!(normalizer().normalize("HDD"), "эйч ди ди");
    }

    #[test]
    fn test_usb_spelled_out() {
        assert_eq!(normalizer().normalize("USB"), "ю эс би");
    }

    #[test]
    fn test_hdmi_spelled_out() {
        assert_eq!(normalizer().normalize("HDMI"), "эйч ди эм ай");
    }

    #[test]
    fn test_ui_spelled_out() {
        assert_eq!(normalizer().normalize("UI"), "ю ай");
    }

    #[test]
    fn test_ux_spelled_out() {
        assert_eq!(normalizer().normalize("UX"), "ю экс");
    }

    #[test]
    fn test_ci_spelled_out() {
        assert_eq!(normalizer().normalize("CI"), "си ай");
    }

    #[test]
    fn test_cd_spelled_out() {
        assert_eq!(normalizer().normalize("CD"), "си ди");
    }

    #[test]
    fn test_ai_spelled_out() {
        assert_eq!(normalizer().normalize("AI"), "эй ай");
    }

    #[test]
    fn test_ml_spelled_out() {
        assert_eq!(normalizer().normalize("ML"), "эм эл");
    }

    #[test]
    fn test_nlp_spelled_out() {
        assert_eq!(normalizer().normalize("NLP"), "эн эл пи");
    }

    #[test]
    fn test_cv_spelled_out() {
        assert_eq!(normalizer().normalize("CV"), "си ви");
    }

    #[test]
    fn test_sql_spelled_out() {
        assert_eq!(normalizer().normalize("SQL"), "эс кью эл");
    }

    #[test]
    fn test_orm_spelled_out() {
        assert_eq!(normalizer().normalize("ORM"), "о ар эм");
    }

    #[test]
    fn test_mvc_spelled_out() {
        assert_eq!(normalizer().normalize("MVC"), "эм ви си");
    }

    #[test]
    fn test_mvp_spelled_out() {
        assert_eq!(normalizer().normalize("MVP"), "эм ви пи");
    }

    #[test]
    fn test_iot_special_case() {
        assert_eq!(normalizer().normalize("IoT"), "ай о ти");
    }

    #[test]
    fn test_ssr_spelled_out() {
        assert_eq!(normalizer().normalize("SSR"), "эс эс ар");
    }

    #[test]
    fn test_ssg_spelled_out() {
        assert_eq!(normalizer().normalize("SSG"), "эс эс джи");
    }

    #[test]
    fn test_csr_spelled_out() {
        assert_eq!(normalizer().normalize("CSR"), "си эс ар");
    }

    #[test]
    fn test_pwa_spelled_out() {
        assert_eq!(normalizer().normalize("PWA"), "пи дабл ю эй");
    }

    #[test]
    fn test_svg_spelled_out() {
        assert_eq!(normalizer().normalize("SVG"), "эс ви джи");
    }

    // --- Letter map tests ---

    #[test]
    fn test_letter_a() {
        assert_eq!(normalizer().normalize("A"), "эй");
    }

    #[test]
    fn test_letter_b() {
        assert_eq!(normalizer().normalize("B"), "би");
    }

    #[test]
    fn test_letter_c() {
        assert_eq!(normalizer().normalize("C"), "си");
    }

    #[test]
    fn test_letter_d() {
        assert_eq!(normalizer().normalize("D"), "ди");
    }

    #[test]
    fn test_letter_e() {
        assert_eq!(normalizer().normalize("E"), "и");
    }

    #[test]
    fn test_letter_f() {
        assert_eq!(normalizer().normalize("F"), "эф");
    }

    #[test]
    fn test_letter_g() {
        assert_eq!(normalizer().normalize("G"), "джи");
    }

    #[test]
    fn test_letter_h() {
        assert_eq!(normalizer().normalize("H"), "эйч");
    }

    #[test]
    fn test_letter_i() {
        assert_eq!(normalizer().normalize("I"), "ай");
    }

    #[test]
    fn test_letter_j() {
        assert_eq!(normalizer().normalize("J"), "джей");
    }

    #[test]
    fn test_letter_k() {
        assert_eq!(normalizer().normalize("K"), "кей");
    }

    #[test]
    fn test_letter_l() {
        assert_eq!(normalizer().normalize("L"), "эл");
    }

    #[test]
    fn test_letter_m() {
        assert_eq!(normalizer().normalize("M"), "эм");
    }

    #[test]
    fn test_letter_n() {
        assert_eq!(normalizer().normalize("N"), "эн");
    }

    #[test]
    fn test_letter_o() {
        assert_eq!(normalizer().normalize("O"), "о");
    }

    #[test]
    fn test_letter_p() {
        assert_eq!(normalizer().normalize("P"), "пи");
    }

    #[test]
    fn test_letter_q() {
        assert_eq!(normalizer().normalize("Q"), "кью");
    }

    #[test]
    fn test_letter_r() {
        assert_eq!(normalizer().normalize("R"), "ар");
    }

    #[test]
    fn test_letter_s() {
        assert_eq!(normalizer().normalize("S"), "эс");
    }

    #[test]
    fn test_letter_t() {
        assert_eq!(normalizer().normalize("T"), "ти");
    }

    #[test]
    fn test_letter_u() {
        assert_eq!(normalizer().normalize("U"), "ю");
    }

    #[test]
    fn test_letter_v() {
        assert_eq!(normalizer().normalize("V"), "ви");
    }

    #[test]
    fn test_letter_w() {
        assert_eq!(normalizer().normalize("W"), "дабл ю");
    }

    #[test]
    fn test_letter_x() {
        assert_eq!(normalizer().normalize("X"), "экс");
    }

    #[test]
    fn test_letter_y() {
        assert_eq!(normalizer().normalize("Y"), "уай");
    }

    #[test]
    fn test_letter_z() {
        assert_eq!(normalizer().normalize("Z"), "зед");
    }

    // --- Case insensitivity tests ---

    #[test]
    fn test_json_lowercase() {
        assert_eq!(normalizer().normalize("json"), "джейсон");
    }

    #[test]
    fn test_json_mixed_case() {
        assert_eq!(normalizer().normalize("Json"), "джейсон");
    }

    #[test]
    fn test_api_lowercase() {
        assert_eq!(normalizer().normalize("api"), "эй пи ай");
    }

    #[test]
    fn test_api_mixed_case() {
        assert_eq!(normalizer().normalize("Api"), "эй пи ай");
    }

    // --- Unknown abbreviations ---

    #[test]
    fn test_xyz_spelled_out() {
        assert_eq!(normalizer().normalize("XYZ"), "экс уай зед");
    }

    #[test]
    fn test_abc_spelled_out() {
        assert_eq!(normalizer().normalize("ABC"), "эй би си");
    }

    #[test]
    fn test_qrs_spelled_out() {
        assert_eq!(normalizer().normalize("QRS"), "кью ар эс");
    }

    #[test]
    fn test_wxyz_spelled_out() {
        assert_eq!(normalizer().normalize("WXYZ"), "дабл ю экс уай зед");
    }

    // --- Mixed case special cases ---

    #[test]
    fn test_ios_special() {
        assert_eq!(normalizer().normalize("iOS"), "ай оу эс");
    }

    #[test]
    fn test_macos_special() {
        assert_eq!(normalizer().normalize("macOS"), "мак оу эс");
    }

    #[test]
    fn test_devops_as_word() {
        assert_eq!(normalizer().normalize("DevOps"), "девопс");
    }

    #[test]
    fn test_graphql_special() {
        assert_eq!(normalizer().normalize("GraphQL"), "граф кью эл");
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

    // --- Edge cases ---

    #[test]
    fn test_empty_string() {
        assert_eq!(normalizer().normalize(""), "");
    }
}
