//! Byte-to-text decoding for imported sources (#224).
//!
//! Single home for turning imported bytes into UTF-8 text: statistical
//! detection (used by `read_text_file` and `fetch_url_text`) and
//! deterministic re-decoding under a user-chosen encoding name live here,
//! behind [`ImportError`] so the command layer owns the wire codes.

use encoding_rs::Encoding;

/// Hard size cap for an imported file or fetched page (10 MiB — same order
/// as the image-fetch cap; a real article is far below it).
pub const MAX_IMPORT_BYTES: u64 = 10 * 1024 * 1024;

/// Extension allowlist for imported files (spec `text-import`).
pub const SUPPORTED_EXTENSIONS: [&str; 4] = ["txt", "md", "html", "htm"];

/// Wire names (`encoding_rs` canonical `Encoding::name()` values) of every
/// encoding listed in the manual-override dialog; the same strings come back
/// from detection as [`DecodedText::encoding`]. UTF-16 is covered through
/// BOM sniffing only — chardetng cannot see it in BOM-less bytes.
pub const SUPPORTED_ENCODING_NAMES: [&str; 13] = [
    "UTF-8",
    "UTF-16LE",
    "UTF-16BE",
    "windows-1251",
    "IBM866",
    "ISO-8859-5",
    "KOI8-R",
    "KOI8-U",
    "x-mac-cyrillic",
    "windows-1250",
    "windows-1252",
    "ISO-8859-1",
    "ISO-8859-15",
];

/// Prefix fed to the statistical detector. A few kilobytes decide reliably;
/// feeding a 10 MiB blob would be wasted work.
const DETECT_SAMPLE_BYTES: usize = 64 * 1024;

/// Characters examined by the non-text guard. Larger files stay guarded via
/// their prefix — a genuinely textual document has stray binary spread out,
/// while a renamed blob is dense with control bytes from the first line.
const NON_TEXT_SAMPLE_CHARS: usize = 4096;

/// Percent of control/replacement characters above which decoded text is
/// declared non-text (binary blob renamed to `.txt`).
const NON_TEXT_MAX_PERCENT: u32 = 10;

#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("unsupported file extension: .{extension}")]
    UnsupportedExtension { extension: String },
    #[error("{size} bytes exceed the {limit}-byte import cap")]
    TooLarge { size: u64, limit: u64 },
    #[error("bytes could not be decoded into a supported text encoding")]
    DecodeFailed,
    #[error("unknown encoding label: {label}")]
    UnknownEncoding { label: String },
}

/// Successfully decoded import payload: always UTF-8 text, plus the name of
/// the encoding actually used (reported to the dialog / debug logs).
#[derive(Debug)]
pub struct DecodedText {
    pub text: String,
    pub encoding: &'static str,
}

/// Lowercase extension of an imported filename; None when it has none.
pub fn extension_of(file_name: &str) -> Option<String> {
    let name = file_name.rsplit(['/', '\\']).next().unwrap_or(file_name);
    let (_, ext) = name.rsplit_once('.')?;
    if ext.is_empty() {
        return None;
    }
    Some(ext.to_ascii_lowercase())
}

/// True when `ext` (already lowercased) is in [`SUPPORTED_EXTENSIONS`].
pub fn is_supported_extension(ext: &str) -> bool {
    SUPPORTED_EXTENSIONS.contains(&ext)
}

/// Decode bytes whose encoding must be detected. Order follows the spec:
/// BOM sniffing first (UTF-8/UTF-16LE/UTF-16BE), full-buffer UTF-8 validity
/// second (most modern files are UTF-8 and a validity check beats
/// statistics), chardetng statistical guess last. The guessed encoding runs
/// through the same decode + non-text guard as everything else.
pub fn detect_and_decode(bytes: &[u8]) -> Result<DecodedText, ImportError> {
    if let Some((enc, bom_len)) = Encoding::for_bom(bytes) {
        return decode_with_encoding(enc, &bytes[bom_len..]);
    }
    if std::str::from_utf8(bytes).is_ok() {
        return decode_with_encoding(encoding_rs::UTF_8, bytes);
    }
    // The buffer is not valid UTF-8, so it contains at least one byte
    // >= 0x80 — the statistical guess always has signal here. ISO-2022-JP
    // stays denied: irrelevant for Cyrillic text files.
    let mut detector = chardetng::EncodingDetector::new(chardetng::Iso2022JpDetection::Deny);
    detector.feed(&bytes[..bytes.len().min(DETECT_SAMPLE_BYTES)], true);
    let enc = detector.guess(None, chardetng::Utf8Detection::Deny);
    decode_with_encoding(enc, bytes)
}

/// Decode bytes under an explicit encoding-label override («Файл с
/// кодировкой…»). Labels are the canonical names from
/// [`SUPPORTED_ENCODING_NAMES`]; BOM handling follows WHATWG rules inside
/// `Encoding::decode`.
pub fn decode_with_label(bytes: &[u8], label: &str) -> Result<DecodedText, ImportError> {
    let enc =
        Encoding::for_label(label.as_bytes()).ok_or_else(|| ImportError::UnknownEncoding {
            label: label.to_string(),
        })?;
    decode_with_encoding(enc, bytes)
}

/// Decode fetched page bytes for URL imports. Precedence follows WHATWG
/// conventions: a byte-level BOM beats any declaration (it is explicit
/// evidence), then a resolvable `charset=` parameter beats statistics —
/// without this, short pages with a small Cyrillic fraction sniff toward
/// the wrong single-byte table while still decoding "validly". Absent or
/// unresolvable declarations fall through to full detection.
pub fn decode_for_content_type(
    bytes: &[u8],
    content_type_header: Option<&str>,
) -> Result<DecodedText, ImportError> {
    if let Some((enc, bom_len)) = Encoding::for_bom(bytes) {
        return decode_with_encoding(enc, &bytes[bom_len..]);
    }
    if let Some(header) = content_type_header {
        if let Some(label) = charset_param(header) {
            if let Some(enc) = Encoding::for_label(label.as_bytes()) {
                return decode_with_encoding(enc, bytes);
            }
        }
    }
    detect_and_decode(bytes)
}

/// Value of the case-insensitive `charset` parameter of a Content-Type
/// header (quotes stripped); None when absent.
fn charset_param(content_type: &str) -> Option<&str> {
    content_type.split(';').skip(1).find_map(|param| {
        let mut pair = param.splitn(2, '=');
        let name = pair.next()?.trim();
        if !name.eq_ignore_ascii_case("charset") {
            return None;
        }
        let value = pair.next()?.trim();
        Some(value.trim_matches('"'))
    })
}

fn decode_with_encoding(enc: &'static Encoding, bytes: &[u8]) -> Result<DecodedText, ImportError> {
    // `decode` never fails: malformed sequences become U+FFFD. The non-text
    // ratio is what actually distinguishes a mojibake-but-textual document
    // from a renamed binary blob.
    let (cow, _had_errors, _had_replacements) = enc.decode(bytes);
    if is_mostly_non_text(&cow) {
        return Err(ImportError::DecodeFailed);
    }
    Ok(DecodedText {
        text: cow.into_owned(),
        encoding: enc.name(),
    })
}

/// Whether the sampled prefix of `text` reads as binary rather than text:
/// more than [`NON_TEXT_MAX_PERCENT`] percent of C0/C1 controls (line/tab
/// whitespace excluded) and U+FFFD replacements.
fn is_mostly_non_text(text: &str) -> bool {
    let mut total = 0usize;
    let mut bad = 0u32;
    for ch in text.chars().take(NON_TEXT_SAMPLE_CHARS) {
        total += 1;
        let is_line_whitespace = matches!(ch, '\n' | '\r' | '\t');
        if (ch.is_control() && !is_line_whitespace) || ch == '\u{FFFD}' {
            bad += 1;
        }
    }
    total > 0 && bad * 100 > NON_TEXT_MAX_PERCENT * total as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Encode a fixture with encoding_rs's own encoder. The correctness of
    /// the codec tables is Firefox's problem, not ours — building inputs
    /// this way pins exactly what matters here: our detection/ordering/glue
    /// behavior against canonical byte streams. (Not usable for UTF-16:
    /// per WHATWG its encoder IS the UTF-8 encoder, so use [`utf16_le`]
    /// / [`utf16_be`] there.)
    fn encoded(encoding_label: &str, text: &str) -> Vec<u8> {
        let enc = Encoding::for_label(encoding_label.as_bytes()).expect("fixture label");
        let (cow, _had_errors, _had_replacements) = enc.encode(text);
        cow.into_owned()
    }

    fn utf16_le(text: &str) -> Vec<u8> {
        text.encode_utf16().flat_map(u16::to_le_bytes).collect()
    }

    fn utf16_be(text: &str) -> Vec<u8> {
        text.encode_utf16().flat_map(u16::to_be_bytes).collect()
    }

    #[test]
    fn extension_of_lowercases_last_component() {
        assert_eq!(extension_of("/tmp/Notes.TXT"), Some("txt".to_string()));
        assert_eq!(extension_of("C:\\доки\\a.HTML"), Some("html".to_string()));
        assert_eq!(extension_of("/with.dots/file.md"), Some("md".to_string()));
        assert_eq!(extension_of("no_ext"), None);
        assert_eq!(extension_of("trailing."), None);
    }

    #[test]
    fn supported_extension_allowlist_matches_spec() {
        assert!(is_supported_extension("txt"));
        assert!(is_supported_extension("md"));
        assert!(is_supported_extension("html"));
        assert!(is_supported_extension("htm"));
        for odd in ["png", "pdf", "exe", ""] {
            assert!(!is_supported_extension(odd));
        }
    }

    #[test]
    fn every_dialog_name_resolves_to_a_real_codec() {
        for name in SUPPORTED_ENCODING_NAMES {
            assert!(
                Encoding::for_label(name.as_bytes()).is_some(),
                "{name} must resolve"
            );
        }
    }

    #[test]
    fn detects_utf8_without_bom() {
        let text = "Технический текст — UTF-8 без BOM";
        let decoded = detect_and_decode(text.as_bytes()).unwrap();
        assert_eq!(decoded.encoding, "UTF-8");
        assert_eq!(decoded.text, text);
    }

    #[test]
    fn detects_ascii_only_as_utf8() {
        let decoded = detect_and_decode(b"plain ascii notes").unwrap();
        assert_eq!(decoded.encoding, "UTF-8");
        assert_eq!(decoded.text, "plain ascii notes");
    }

    #[test]
    fn strips_utf8_bom() {
        let decoded = detect_and_decode("\u{FEFF}Текст после BOM".as_bytes()).unwrap();
        assert_eq!(decoded.encoding, "UTF-8");
        assert_eq!(decoded.text, "Текст после BOM");
    }

    #[test]
    fn decodes_utf16le_with_bom() {
        let mut bytes = vec![0xFF, 0xFE];
        bytes.extend_from_slice(&utf16_le("Юникод из UTF-16"));
        let decoded = detect_and_decode(&bytes).unwrap();
        assert_eq!(decoded.encoding, "UTF-16LE");
        assert_eq!(decoded.text, "Юникод из UTF-16");
    }

    #[test]
    fn decodes_utf16be_with_bom() {
        let mut bytes = vec![0xFE, 0xFF];
        bytes.extend_from_slice(&utf16_be("Текст в BE"));
        let decoded = detect_and_decode(&bytes).unwrap();
        assert_eq!(decoded.encoding, "UTF-16BE");
        assert_eq!(decoded.text, "Текст в BE");
    }

    /// The load-bearing spec risk: a BOM-less CP1251 file must come back as
    /// clean Cyrillic named windows-1251, not mojibake from a wrong Cyrillic
    /// table. The phrase distinguishes CP1251 readings from KOI8-R ones.
    #[test]
    fn detects_cp1251_file_without_bom() {
        let phrase = "Пример русского текста для проверки детектора";
        let decoded = detect_and_decode(&encoded("windows-1251", phrase)).unwrap();
        assert_eq!(decoded.encoding, "windows-1251");
        assert_eq!(decoded.text, phrase);
    }

    /// Pure-Russian bytes decode identically under KOI8-R and KOI8-U (they
    /// differ only in a few Ukrainian glyphs), so the detector may report
    /// either name for the same bytes — pin the round-trip, not the tie-break.
    #[test]
    fn detects_koi8_family_file_without_bom() {
        let phrase = "Файл в кодировке КОИ8 для проверки детектора кодировок";
        let decoded = detect_and_decode(&encoded("KOI8-R", phrase)).unwrap();
        assert!(
            matches!(decoded.encoding, "KOI8-R" | "KOI8-U"),
            "got {}",
            decoded.encoding
        );
        assert_eq!(decoded.text, phrase);
    }

    #[test]
    fn manual_override_decodes_cp1251_deterministically() {
        let phrase = "Неверный автодетект исправляется вручную";
        let bytes = encoded("windows-1251", phrase);
        let decoded = decode_with_label(&bytes, "windows-1251").unwrap();
        assert_eq!(decoded.encoding, "windows-1251");
        assert_eq!(decoded.text, phrase);
    }

    #[test]
    fn manual_override_decodes_cp866_deterministically() {
        let phrase = "Досовский текст в CP866";
        let bytes = encoded("IBM866", phrase);
        let decoded = decode_with_label(&bytes, "IBM866").unwrap();
        assert_eq!(decoded.encoding, "IBM866");
        assert_eq!(decoded.text, phrase);
    }

    #[test]
    fn unknown_override_label_is_an_error() {
        let err = decode_with_label(b"bytes", "not-a-codec").unwrap_err();
        assert!(
            matches!(err, ImportError::UnknownEncoding { ref label } if label == "not-a-codec")
        );
    }

    #[test]
    fn binary_blob_is_decode_failed() {
        let mut blob: Vec<u8> = (0..=255u8).cycle().take(2048).collect();
        // The guard allows line/tab whitespace even in binary; excluding them
        // proves they cannot mask the verdict either way.
        blob.retain(|b| !matches!(b, b'\n' | b'\r' | b'\t'));
        let err = detect_and_decode(&blob).unwrap_err();
        assert!(matches!(err, ImportError::DecodeFailed));
    }

    /// Real-world edge: a mostly-textual old file with occasional stray
    /// bytes yields some U+FFFD replacements but stays far below the cap.
    #[test]
    fn mostly_text_with_some_noise_is_accepted() {
        let mut text = "Нормальный текст ".repeat(400);
        text.push('\u{FFFD}');
        text.push_str("конец файла");
        assert!(!is_mostly_non_text(&text));
    }

    #[test]
    fn empty_input_decodes_to_empty_utf8() {
        let decoded = detect_and_decode(b"").unwrap();
        assert_eq!(decoded.encoding, "UTF-8");
        assert!(decoded.text.is_empty());
    }

    // ── fetch-time Content-Type hints (decode_for_content_type) ───────────

    /// The silent-mojibake class the reviewer flagged: a short page whose
    /// statistics point at the wrong single-byte table decodes correctly
    /// when the server declares the charset.
    #[test]
    fn declared_charset_beats_statistics_on_ambiguous_short_pages() {
        let phrase = "Файл в CP1251 без BOM"; // historically sniffs wrong tables
        let bytes = encoded("KOI8-R", phrase);
        let decoded = decode_for_content_type(&bytes, Some("text/html; charset=KOI8-R")).unwrap();
        assert_eq!(decoded.encoding, "KOI8-R");
        assert_eq!(decoded.text, phrase);
    }

    #[test]
    fn quoted_lowercase_charset_params_are_accepted() {
        let phrase = "Объявленная кодировка в нижнем регистре";
        let bytes = encoded("IBM866", phrase);
        let decoded =
            decode_for_content_type(&bytes, Some("Text/HTML;charset=\"ibm866\" ")).unwrap();
        assert_eq!(decoded.encoding, "IBM866");
        assert_eq!(decoded.text, phrase);
    }

    #[test]
    fn bom_wins_over_the_declared_charset() {
        let mut bytes = vec![0xFF, 0xFE];
        bytes.extend_from_slice(&utf16_le("Бом сильнее объявления"));
        let decoded =
            decode_for_content_type(&bytes, Some("text/html; charset=windows-1251")).unwrap();
        assert_eq!(decoded.encoding, "UTF-16LE");
        assert_eq!(decoded.text, "Бом сильнее объявления");
    }

    #[test]
    fn missing_or_unknown_declared_charset_falls_back_to_detection() {
        // No charset parameter at all: statistics decide (long enough here).
        let doc = "Абзац документа для статистического детектора кодировок.\n".repeat(6);
        let detected =
            decode_for_content_type(&encoded("windows-1251", &doc), Some("text/html")).unwrap();
        assert_eq!(detected.encoding, "windows-1251");
        assert_eq!(detected.text, doc);

        // An invented label must not fail the whole import: UTF-8 input is
        // served by the validity fast path regardless.
        let utf8 =
            decode_for_content_type("просто текст".as_bytes(), Some("x/y; charset=nonsense"));
        assert!(utf8.is_ok());
    }
}
