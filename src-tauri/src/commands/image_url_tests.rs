use super::CommandError;
use super::{MAX_IMAGE_BYTES, ensure_image_size, media_type, validate_image_url};

#[test]
fn accepts_absolute_http_and_https_urls() {
    for url in ["https://example.com/a.png", "http://example.com/a.png"] {
        let parsed = validate_image_url(url).unwrap();
        assert_eq!(parsed.as_str(), url);
    }
}

#[test]
fn rejects_non_http_schemes() {
    for url in [
        "file:///etc/passwd",
        "data:image/png;base64,AAAA",
        "ftp://example.com/a.png",
    ] {
        let err = validate_image_url(url).unwrap_err();
        match err {
            CommandError::Internal { code, .. } => {
                assert_eq!(code, "image.url_scheme_unsupported")
            }
            other => panic!("{url}: expected Internal, got {other:?}"),
        }
    }
}

#[test]
fn rejects_unparseable_urls() {
    assert!(validate_image_url("not a url").is_err());
}

#[test]
fn size_cap_admits_the_boundary_and_rejects_one_byte_over() {
    assert!(ensure_image_size(MAX_IMAGE_BYTES as u64).is_ok());
    assert!(ensure_image_size(MAX_IMAGE_BYTES as u64 + 1).is_err());
}

#[test]
fn media_type_strips_parameters_and_lowercases() {
    let value = reqwest::header::HeaderValue::from_static("Image/PNG; charset=utf-8");
    assert_eq!(media_type(Some(&value)).as_deref(), Some("image/png"),);
}

#[test]
fn media_type_is_none_for_missing_empty_or_invalid_headers() {
    assert_eq!(media_type(None), None);
    let empty = reqwest::header::HeaderValue::from_static("");
    assert_eq!(media_type(Some(&empty)), None);
    let invalid = reqwest::header::HeaderValue::from_bytes(&[0xff, 0xfe]).unwrap();
    assert_eq!(media_type(Some(&invalid)), None);
}
