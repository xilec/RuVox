//! Import command cluster (text-import spec, #224): file picker, file
//! reader, and page fetcher. Split out of `mod.rs` along the domain seam
//! (review finding: the shared commands file had crossed its one-read-pass
//! budget); errors map onto the `import.*` wire codes here, byte decoding
//! lives in [`crate::import`].

use super::*;

#[derive(Debug, Serialize)]
pub struct ReadTextFileResult {
    /// Decoded UTF-8 text.
    pub text: String,
    /// Canonical name of the encoding actually used.
    pub encoding: String,
}

/// Map an [`crate::import::ImportError`] onto the `import.*` wire codes
/// localized by the frontend catalogs.
fn map_import_error(e: crate::import::ImportError) -> CommandError {
    use crate::import::ImportError as E;
    match e {
        E::UnsupportedExtension { extension } => {
            CommandError::internal("import.unsupported_extension", vec![extension])
        }
        E::TooLarge { size, limit } => CommandError::internal(
            "import.too_large",
            vec![size.to_string(), limit.to_string()],
        ),
        E::DecodeFailed => CommandError::internal("import.decode_failed", vec![]),
        E::UnknownEncoding { label } => {
            CommandError::internal("import.unknown_encoding", vec![label])
        }
    }
}

/// Size-check + read of one import source file on a blocking thread; decode
/// happens in [`crate::import`] so files and fetched pages share it.
fn read_import_file(
    path: &str,
    encoding_label: Option<&str>,
) -> Result<ReadTextFileResult, CommandError> {
    let metadata = std::fs::metadata(path).map_err(|e| {
        CommandError::not_found("import.io_failed", vec![path.to_string()])
            .with_message(e.to_string())
    })?;
    if metadata.len() > crate::import::MAX_IMPORT_BYTES {
        return Err(map_import_error(crate::import::ImportError::TooLarge {
            size: metadata.len(),
            limit: crate::import::MAX_IMPORT_BYTES,
        }));
    }
    let bytes = std::fs::read(path).map_err(|e| {
        CommandError::not_found("import.io_failed", vec![path.to_string()])
            .with_message(e.to_string())
    })?;
    let decoded = match encoding_label {
        Some(label) => crate::import::decode_with_label(&bytes, label),
        None => crate::import::detect_and_decode(&bytes),
    }
    .map_err(map_import_error)?;
    Ok(ReadTextFileResult {
        text: decoded.text,
        encoding: decoded.encoding.to_string(),
    })
}

/// Native file picker for the import menu (#224). Runs on plain `rfd` — the
/// same backend tauri-plugin-dialog uses — so the webview needs no dialog
/// capability and no new dependency (precedent: the startup storage-failure
/// dialog, #223). Returns None when the user cancels. The filter mirrors
/// [`crate::import::SUPPORTED_EXTENSIONS`] (single home of the allowlist).
#[tauri::command]
pub async fn pick_import_file() -> CmdResult<Option<String>> {
    tokio::task::spawn_blocking(|| {
        rfd::FileDialog::new()
            .add_filter("Текстовые файлы", &crate::import::SUPPORTED_EXTENSIONS)
            .pick_file()
            .map(|p| p.to_string_lossy().into_owned())
    })
    .await
    .map_err(|e| CommandError::internal("import.task_panicked", vec![]).with_message(e.to_string()))
}

/// Read a local text file for import (#224): validate the extension
/// allowlist and the 10 MiB cap, detect the encoding from a BOM first and
/// statistics second (or re-decode under the explicit `encoding` label the
/// user picked in «Файл с кодировкой…»), return UTF-8 text plus the encoding
/// name actually used.
#[tauri::command]
pub async fn read_text_file(
    path: String,
    encoding: Option<String>,
) -> CmdResult<ReadTextFileResult> {
    match crate::import::extension_of(&path) {
        Some(ext) if crate::import::is_supported_extension(&ext) => {}
        // No extension at all reports the file name — an empty param would
        // read as a broken sentence in the localized message.
        Some(ext) => {
            return Err(map_import_error(
                crate::import::ImportError::UnsupportedExtension { extension: ext },
            ));
        }
        None => {
            let name = path.rsplit(['/', '\\']).next().unwrap_or(&path).to_string();
            return Err(map_import_error(
                crate::import::ImportError::UnsupportedExtension { extension: name },
            ));
        }
    }

    // File I/O is blocking by nature — keep the async runtime free.
    tokio::task::spawn_blocking(move || read_import_file(&path, encoding.as_deref()))
        .await
        .map_err(|e| {
            CommandError::internal("import.task_panicked", vec![]).with_message(e.to_string())
        })?
}

#[derive(Debug, Serialize)]
pub struct FetchUrlTextResult {
    /// Page body decoded to UTF-8 by the shared import decoder.
    pub text: String,
    /// Canonical name of the encoding used for the body.
    pub encoding: String,
    /// Lowercased media type of the response (`text/html`, …) — the format
    /// classification hint for the frontend routing layer; None when the
    /// server sent no usable Content-Type.
    pub content_type: Option<String>,
}

/// Core of [`fetch_url_text`] with an injectable client so tests can point
/// it at a loopback mock server.
async fn download_page(client: &reqwest::Client, url: &str) -> CmdResult<FetchUrlTextResult> {
    const NS: &str = "import";
    let parsed = parse_http_url(url).map_err(|e| map_url_error(NS, e))?;

    let response = client.get(parsed).send().await.map_err(|e| {
        CommandError::internal("import.fetch_failed", vec![]).with_message(e.to_string())
    })?;
    if !response.status().is_success() {
        return Err(CommandError::internal(
            "import.fetch_failed",
            vec![response.status().as_u16().to_string()],
        ));
    }
    if let Some(len) = response.content_length() {
        ensure_import_size(len)?;
    }
    // The raw header value (charset parameter included) feeds the decoder;
    // only the lowercased media type goes back to the frontend.
    let header_content_type: Option<String> = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    let content_type = media_type(response.headers().get(reqwest::header::CONTENT_TYPE));
    let bytes = response.bytes().await.map_err(|e| {
        CommandError::internal("import.fetch_failed", vec![]).with_message(e.to_string())
    })?;
    ensure_import_size(bytes.len() as u64)?;

    let decoded = crate::import::decode_for_content_type(&bytes, header_content_type.as_deref())
        .map_err(map_import_error)?;
    Ok(FetchUrlTextResult {
        text: decoded.text,
        encoding: decoded.encoding.to_string(),
        content_type,
    })
}

/// Enforce the import size cap for both the header pre-check and the
/// post-read re-check of [`download_page`] / [`read_import_file`].
fn ensure_import_size(len: u64) -> Result<(), CommandError> {
    if len > crate::import::MAX_IMPORT_BYTES {
        return Err(map_import_error(crate::import::ImportError::TooLarge {
            size: len,
            limit: crate::import::MAX_IMPORT_BYTES,
        }));
    }
    Ok(())
}

/// Fetch an http(s) page for import (#224): scheme allowlist, 10 MiB cap,
/// connect/total timeouts via the shared client, decoded UTF-8 text back
/// together with the encoding used and the response content type. Routing
/// the text into plain/markdown/html and detecting SPA shells is frontend
/// work (preview-dialog + text-import specs).
#[tauri::command]
pub async fn fetch_url_text(url: String) -> CmdResult<FetchUrlTextResult> {
    download_page(http_client(), &url).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read as _, Write as _};

    /// Serve one canned HTTP/1.1 response on a loopback port and return the
    /// base URL. The socket closes after writing, which reqwest reads as a
    /// complete body (Content-Length matches the body). The listening thread
    /// is detached: every test awaits its single request through
    /// `download_page`, so nothing outlives the assertion.
    fn serve_once(response: Vec<u8>) -> String {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            if let Ok((mut sock, _)) = listener.accept() {
                let mut buf = [0u8; 1024];
                let _ = sock.read(&mut buf);
                let _ = sock.write_all(&response);
            }
        });
        format!("http://{addr}/page")
    }

    fn http_response(status_line: &str, content_type: &str, body: &[u8]) -> Vec<u8> {
        let head = format!(
            "{status_line}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        let mut bytes = head.into_bytes();
        bytes.extend_from_slice(body);
        bytes
    }

    fn cp1251_body(text: &str) -> Vec<u8> {
        let enc = encoding_rs::Encoding::for_label(b"windows-1251").expect("windows-1251 exists");
        let (cow, _errors, _repl) = enc.encode(text);
        cow.into_owned()
    }

    #[tokio::test]
    async fn download_page_decodes_cp1251_html_page() {
        let phrase = "Статья из старого сайта";
        let url = serve_once(http_response(
            "HTTP/1.1 200 OK",
            "text/html; charset=windows-1251",
            &cp1251_body(phrase),
        ));
        let result = download_page(http_client(), &url).await.unwrap();
        assert_eq!(result.text, phrase);
        assert_eq!(result.encoding, "windows-1251");
        assert_eq!(result.content_type.as_deref(), Some("text/html"));
    }

    /// The header charset is honored even when statistics would disagree —
    /// a short page served without a matching statistical profile.
    #[tokio::test]
    async fn download_page_prefers_declared_charset_over_statistics() {
        let enc = encoding_rs::Encoding::for_label(b"KOI8-R").expect("KOI8-R exists");
        let phrase = "Статья, объявленная как КОИ8";
        let (cow, _e, _r) = enc.encode(phrase);
        let url = serve_once(http_response(
            "HTTP/1.1 200 OK",
            "text/html; charset=KOI8-R",
            cow.as_ref(),
        ));
        let result = download_page(http_client(), &url).await.unwrap();
        assert_eq!(result.text, phrase);
        assert_eq!(result.encoding, "KOI8-R");
    }

    #[tokio::test]
    async fn download_page_surfaces_http_status_in_params() {
        let url = serve_once(http_response("HTTP/1.1 403 Forbidden", "text/html", b"no"));
        let err = download_page(http_client(), &url).await.unwrap_err();
        match err {
            CommandError::Internal { code, params, .. } => {
                assert_eq!(code, "import.fetch_failed");
                assert_eq!(params, vec!["403".to_string()]);
            }
            other => panic!("expected Internal(import.fetch_failed), got {other:?}"),
        }
    }

    #[tokio::test]
    async fn download_page_rejects_oversized_declared_length_before_reading() {
        // Declared length exceeds the cap; the body itself is never sent.
        let mut response = b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n".to_vec();
        response.extend_from_slice(
            format!(
                "Content-Length: {}\r\nConnection: close\r\n\r\nsmall",
                crate::import::MAX_IMPORT_BYTES + 1
            )
            .as_bytes(),
        );
        let url = serve_once(response);
        let err = download_page(http_client(), &url).await.unwrap_err();
        match err {
            CommandError::Internal { code, params, .. } => {
                assert_eq!(code, "import.too_large");
                assert_eq!(
                    params,
                    vec![
                        (crate::import::MAX_IMPORT_BYTES + 1).to_string(),
                        crate::import::MAX_IMPORT_BYTES.to_string(),
                    ]
                );
            }
            other => panic!("expected Internal(import.too_large), got {other:?}"),
        }
    }

    #[tokio::test]
    async fn download_page_maps_scheme_and_parse_failures_to_import_codes() {
        for (input, expected_code) in [
            ("not a url", "import.url_invalid"),
            ("ftp://example.com/a", "import.url_scheme_unsupported"),
        ] {
            let err = download_page(http_client(), input).await.unwrap_err();
            match err {
                CommandError::Internal { code, .. } => assert_eq!(code, expected_code, "{input}"),
                other => panic!("{input}: expected Internal, got {other:?}"),
            }
        }
    }

    #[tokio::test]
    async fn download_page_reports_connection_failure_as_fetch_failed() {
        // Bind, learn the port, then release: nothing listens there anymore,
        // so the connect fails fast and deterministically.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);
        let err = download_page(http_client(), &format!("http://{addr}/x"))
            .await
            .unwrap_err();
        match err {
            CommandError::Internal {
                code,
                params,
                message,
                ..
            } => {
                assert_eq!(code, "import.fetch_failed");
                // No HTTP status — the failure never got a response.
                assert!(params.is_empty());
                assert!(message.is_some());
            }
            other => panic!("expected Internal(import.fetch_failed), got {other:?}"),
        }
    }

    /// The blocking read path behind `read_text_file`: extension allowlist,
    /// size cap, and decode-by-override flow end to end on real files.
    #[test]
    fn read_import_file_rejects_unsupported_extension_and_oversize() {
        let dir = tempfile::TempDir::new().unwrap();
        let bad = dir.path().join("image.png");
        std::fs::write(&bad, b"png").unwrap();

        let extension = crate::import::extension_of(bad.to_str().unwrap()).unwrap();
        assert!(!crate::import::is_supported_extension(&extension));

        let big = dir.path().join("big.txt");
        std::fs::write(
            &big,
            vec![b'a'; (crate::import::MAX_IMPORT_BYTES + 1) as usize],
        )
        .unwrap();
        let err = read_import_file(big.to_str().unwrap(), None).unwrap_err();
        match err {
            CommandError::Internal { code, .. } => assert_eq!(code, "import.too_large"),
            other => panic!("expected too_large, got {other:?}"),
        }
    }

    #[test]
    fn read_import_file_detects_and_overrides_encoding() {
        let dir = tempfile::TempDir::new().unwrap();
        let enc =
            encoding_rs::Encoding::for_label(b"windows-1251").expect("windows-1251 label exists");
        // A paragraph-length document: a one-liner is statistically ambiguous
        // between single-byte Cyrillic tables (documented risk; the manual
        // encoding dialog is the escape hatch), while real files are long.
        let text = "Заголовок отчёта по проекту.\nРаздел содержит описание технических требований и перечень использованных компонентов.\n"
            .repeat(6);
        let (cow, _e, _r) = enc.encode(&text);
        let path = dir.path().join("old.txt");
        std::fs::write(&path, &*cow).unwrap();

        let detected = read_import_file(path.to_str().unwrap(), None).unwrap();
        assert_eq!(detected.encoding, "windows-1251");
        assert_eq!(detected.text, text);

        // An explicit override re-decodes deterministically even when it
        // disagrees with detection (mojibake but still textual).
        let overridden = read_import_file(path.to_str().unwrap(), Some("KOI8-R")).unwrap();
        assert_eq!(overridden.encoding, "KOI8-R");
        assert_ne!(overridden.text, detected.text);

        let unknown = read_import_file(path.to_str().unwrap(), Some("nope")).unwrap_err();
        match unknown {
            CommandError::Internal { code, params, .. } => {
                assert_eq!(code, "import.unknown_encoding");
                assert_eq!(params, vec!["nope".to_string()]);
            }
            other => panic!("expected unknown_encoding, got {other:?}"),
        }
    }

    #[test]
    fn map_import_error_covers_every_variant_with_params() {
        use crate::import::ImportError as E;
        let cases: [(E, &str, &[&str]); 4] = [
            (
                E::UnsupportedExtension {
                    extension: "exe".into(),
                },
                "import.unsupported_extension",
                &["exe"],
            ),
            (
                E::TooLarge {
                    size: 11,
                    limit: 10,
                },
                "import.too_large",
                &["11", "10"],
            ),
            (E::DecodeFailed, "import.decode_failed", &[]),
            (
                E::UnknownEncoding {
                    label: "zzz".into(),
                },
                "import.unknown_encoding",
                &["zzz"],
            ),
        ];
        for (source, code, params) in cases {
            match map_import_error(source) {
                CommandError::Internal {
                    code: wire_code,
                    params: wire_params,
                    ..
                } => {
                    assert_eq!(wire_code, code);
                    assert_eq!(
                        wire_params,
                        params.iter().map(|s| s.to_string()).collect::<Vec<_>>()
                    );
                }
                other => panic!("expected Internal({code}), got {other:?}"),
            }
        }
    }
}
