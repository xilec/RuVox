//! Manifest-driven model bundle downloader shared by every engine that ships
//! its weights as GitHub Release assets.
//!
//! The manifest is fetched first (it carries the per-file size + sha256 list
//! and, for TeraTTS, the flat asset name), then every listed file:
//!
//! - files already on disk with a matching size + sha256 are skipped
//!   (idempotent re-runs);
//! - a partial file from an interrupted run is resumed with a `Range` request;
//!   if the server ignores the range, the file restarts from zero;
//! - each download is sha256-verified while streaming and only then renamed
//!   into place — a partial or corrupt file is never treated as installed
//!   (the availability probes and the engine loaders key off the real names);
//! - a checksum mismatch or any I/O failure removes the `.partial` file and
//!   aborts with a typed error, leaving the engine unavailable.
//!
//! Progress goes through the shared `Emitter`:
//! - `bundle_download_started`  — `{ engine }`
//! - `bundle_download_progress` — `{ engine, file, file_idx, total_files,
//!                                  downloaded_bytes, total_bytes }`,
//!   throttled to ~1 event per 256 KB, plus `skipped: true` for files
//!   already present and valid
//! - `bundle_download_finished` — `{ engine, ok: true }` or
//!   `{ engine, ok: false, message }`

use std::path::{Component, Path, PathBuf};

use futures_util::StreamExt;
use serde_json::json;
use sha2::{Digest, Sha256};
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::tts::TtsError;
use crate::tts::supervisor::Emitter;

/// Throttle progress events to ~1 per 256 KB so the IPC bridge does not
/// drown the renderer (same value as the Piper voice download).
const PROGRESS_EMIT_BYTES: u64 = 256 * 1024;

/// One file of a bundle, normalized across engines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleFile {
    /// Bundle-relative destination path.
    pub path: String,
    /// Flat release asset name (GitHub release assets cannot contain `/`).
    pub asset: String,
    pub size: u64,
    pub sha256: String,
}

/// Download and verify a whole bundle into `bundle_dir`.
///
/// `parse_manifest` validates the manifest bytes and returns the file list; a
/// malformed manifest is rejected before anything is persisted, so a broken
/// download never leaves a corrupt `manifest.json` for the availability probe.
pub async fn download(
    bundle_dir: &Path,
    engine: &str,
    base_url: &str,
    parse_manifest: impl Fn(&[u8]) -> Result<Vec<BundleFile>, String>,
    emitter: &Emitter,
) -> Result<(), TtsError> {
    download_from_manifest_url(
        bundle_dir,
        engine,
        &format!("{base_url}/manifest.json"),
        None,
        parse_manifest,
        emitter,
    )
    .await
}

/// Same as [`download`], but with the manifest URL supplied by a channel and
/// an optional expected sha256 for the manifest itself. The channel hash is
/// verified before the manifest is parsed or written, so a swapped or
/// truncated manifest cannot half-install a bundle.
pub async fn download_from_manifest_url(
    bundle_dir: &Path,
    engine: &str,
    manifest_url: &str,
    manifest_sha256: Option<&str>,
    parse_manifest: impl Fn(&[u8]) -> Result<Vec<BundleFile>, String>,
    emitter: &Emitter,
) -> Result<(), TtsError> {
    (emitter)("bundle_download_started", json!({ "engine": engine }));

    let result = download_inner(
        bundle_dir,
        engine,
        manifest_url,
        manifest_sha256,
        parse_manifest,
        emitter,
    )
    .await;

    match &result {
        Ok(()) => {
            (emitter)(
                "bundle_download_finished",
                json!({ "engine": engine, "ok": true }),
            );
        }
        Err(e) => {
            (emitter)(
                "bundle_download_finished",
                json!({ "engine": engine, "ok": false, "message": e.to_string() }),
            );
        }
    }
    result
}

async fn download_inner(
    bundle_dir: &Path,
    engine: &str,
    manifest_url: &str,
    manifest_sha256: Option<&str>,
    parse_manifest: impl Fn(&[u8]) -> Result<Vec<BundleFile>, String>,
    emitter: &Emitter,
) -> Result<(), TtsError> {
    fs::create_dir_all(bundle_dir)
        .await
        .map_err(TtsError::Ipc)?;

    let client = reqwest::Client::builder()
        // The largest bundle is ~390 MiB: 30 min overall is generous even on a
        // slow link, while the connect timeout keeps a dead host from hanging
        // the download forever.
        .connect_timeout(std::time::Duration::from_secs(60))
        .timeout(std::time::Duration::from_secs(30 * 60))
        .build()
        .map_err(|e| TtsError::Ttsd {
            code: "bundle_download_failed".to_string(),
            message: format!("не удалось инициализировать HTTP-клиент: {e}"),
        })?;

    let manifest_bytes = fetch_whole(&client, manifest_url).await?;
    if let Some(expected) = manifest_sha256 {
        let actual = sha256_bytes(&manifest_bytes);
        if !actual.eq_ignore_ascii_case(expected) {
            return Err(TtsError::Ttsd {
                code: "bundle_manifest_checksum_failed".to_string(),
                message: "контрольная сумма манифеста не совпала с каналом".to_string(),
            });
        }
    }
    let files = parse_manifest(&manifest_bytes).map_err(|e| TtsError::Ttsd {
        code: "bundle_manifest_invalid".to_string(),
        message: format!("манифест бандла повреждён: {e}"),
    })?;
    if files.is_empty() {
        return Err(TtsError::Ttsd {
            code: "bundle_manifest_invalid".to_string(),
            message: "манифест бандла не содержит файлов".to_string(),
        });
    }
    write_atomic(&bundle_dir.join("manifest.json"), &manifest_bytes).await?;

    let base_url = manifest_url
        .strip_suffix("/manifest.json")
        .unwrap_or(manifest_url);
    let total_files = files.len() as u32;
    for (idx, entry) in files.iter().enumerate() {
        let dest = safe_path(bundle_dir, &entry.path)?;
        if file_is_valid(&dest, entry).await {
            (emitter)(
                "bundle_download_progress",
                json!({
                    "engine": engine,
                    "file": entry.path,
                    "file_idx": idx as u32,
                    "total_files": total_files,
                    "downloaded_bytes": 0u64,
                    "total_bytes": entry.size,
                    "skipped": true,
                }),
            );
            continue;
        }
        let url = format!("{base_url}/{}", entry.asset);
        download_one(
            &client,
            &url,
            &dest,
            entry,
            engine,
            idx as u32,
            total_files,
            emitter,
        )
        .await?;
    }

    Ok(())
}

/// Resolve a manifest path inside the bundle directory, rejecting absolute
/// paths, `..`, `.` and anything else that could escape it.
fn safe_path(bundle_dir: &Path, relative: &str) -> Result<PathBuf, TtsError> {
    let path = Path::new(relative);
    if relative.is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(TtsError::Ttsd {
            code: "bundle_path_unsafe".to_string(),
            message: format!("небезопасный путь в манифесте: {relative}"),
        });
    }
    Ok(bundle_dir.join(path))
}

/// `true` when `dest` exists and matches the manifest's size + sha256.
/// Any I/O error counts as "not valid" — the file is re-downloaded.
pub(crate) async fn file_is_valid(dest: &Path, entry: &BundleFile) -> bool {
    let Ok(meta) = fs::metadata(dest).await else {
        return false;
    };
    if meta.len() != entry.size {
        return false;
    }
    match sha256_file(dest).await {
        Ok(actual) => actual.eq_ignore_ascii_case(&entry.sha256),
        Err(_) => false,
    }
}

fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

/// sha256 of a local file, read in 1 MiB chunks (bundle files reach ~256 MB).
async fn sha256_file(path: &Path) -> std::io::Result<String> {
    let mut file = fs::File::open(path).await?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = file.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

/// Where an in-flight download lives before it is verified and renamed into
/// place. Mirrors the Piper `.partial` convention.
fn partial_path(dest: &Path) -> PathBuf {
    dest.with_extension("partial")
}

/// Small whole-body GET (the manifest). Non-2xx — including the 404 of a
/// not-yet-published release — is a typed `bundle_download_failed` error.
async fn fetch_whole(client: &reqwest::Client, url: &str) -> Result<Vec<u8>, TtsError> {
    let resp = client.get(url).send().await.map_err(|e| TtsError::Ttsd {
        code: "bundle_download_failed".to_string(),
        message: format!("не удалось скачать {url}: {e}"),
    })?;
    let resp = check_status(resp, url)?;
    resp.bytes()
        .await
        .map(|bytes| bytes.to_vec())
        .map_err(|e| TtsError::Ttsd {
            code: "bundle_download_failed".to_string(),
            message: format!("не удалось прочитать ответ {url}: {e}"),
        })
}

fn check_status(resp: reqwest::Response, url: &str) -> Result<reqwest::Response, TtsError> {
    if resp.status().is_success() {
        return Ok(resp);
    }
    let status = resp.status();
    let message = if status == reqwest::StatusCode::NOT_FOUND {
        format!(
            "файл не найден в релизе (HTTP 404): {url}. \
             Релиз с моделями ещё не опубликован или ссылка устарела."
        )
    } else {
        format!("HTTP {status} при скачивании {url}")
    };
    Err(TtsError::Ttsd {
        code: "bundle_download_failed".to_string(),
        message,
    })
}

/// Write `bytes` to `dest` via a `.partial` sibling + rename, so a crash
/// mid-write never leaves a torn file under the real name.
async fn write_atomic(dest: &Path, bytes: &[u8]) -> Result<(), TtsError> {
    let tmp = partial_path(dest);
    fs::write(&tmp, bytes).await.map_err(TtsError::Ipc)?;
    fs::rename(&tmp, dest).await.map_err(TtsError::Ipc)
}

/// Stream one bundle file to `<dest>.partial`, verifying sha256 on the fly.
/// On success the file is renamed into place; on any failure the partial
/// file is removed so the next run does not mistake it for an installed one.
#[allow(clippy::too_many_arguments)]
async fn download_one(
    client: &reqwest::Client,
    url: &str,
    dest: &Path,
    entry: &BundleFile,
    engine: &str,
    file_idx: u32,
    total_files: u32,
    emitter: &Emitter,
) -> Result<(), TtsError> {
    let result = download_one_inner(
        client,
        url,
        dest,
        entry,
        engine,
        file_idx,
        total_files,
        emitter,
    )
    .await;
    if result.is_err() {
        let _ = fs::remove_file(partial_path(dest)).await;
    }
    result
}

#[allow(clippy::too_many_arguments)]
async fn download_one_inner(
    client: &reqwest::Client,
    url: &str,
    dest: &Path,
    entry: &BundleFile,
    engine: &str,
    file_idx: u32,
    total_files: u32,
    emitter: &Emitter,
) -> Result<(), TtsError> {
    // TeraTTS manifests use nested destinations such as
    // `models/vocoder.onnx`; the release assets themselves are flat, so the
    // destination parent must be created before the `.partial` file.
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).await.map_err(TtsError::Ipc)?;
    }
    let tmp = partial_path(dest);
    let mut existing = match fs::metadata(&tmp).await {
        Ok(meta) if meta.len() <= entry.size => meta.len(),
        _ => 0,
    };
    // A complete partial file is verified before a Range request: a full-size
    // file would make the server answer 416, and the bytes may be from an
    // older bundle version.
    if existing == entry.size && existing > 0 {
        let actual = sha256_file(&tmp).await.map_err(TtsError::Ipc)?;
        if actual.eq_ignore_ascii_case(&entry.sha256) {
            fs::rename(&tmp, dest).await.map_err(TtsError::Ipc)?;
            emit_progress(emitter, engine, entry, file_idx, total_files, existing);
            return Ok(());
        }
        let _ = fs::remove_file(&tmp).await;
        existing = 0;
    }

    let mut hasher = Sha256::new();
    if existing > 0 {
        let mut prefix = fs::File::open(&tmp).await.map_err(TtsError::Ipc)?;
        let mut buf = vec![0u8; 1 << 20];
        loop {
            let n = prefix.read(&mut buf).await.map_err(TtsError::Ipc)?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
        }
    }

    let mut request = client.get(url);
    if existing > 0 {
        request = request.header(reqwest::header::RANGE, format!("bytes={existing}-"));
    }
    let resp = request.send().await.map_err(|e| TtsError::Ttsd {
        code: "bundle_download_failed".to_string(),
        message: format!("не удалось скачать {url}: {e}"),
    })?;

    let append = existing > 0 && resp.status() == reqwest::StatusCode::PARTIAL_CONTENT;
    let (resp, mut file, existing) = if append {
        let file = fs::OpenOptions::new()
            .append(true)
            .open(&tmp)
            .await
            .map_err(TtsError::Ipc)?;
        (resp, file, existing)
    } else {
        // A 200 answer to a Range request means the server ignored it: start
        // over and drop the stale prefix from the hasher.
        let resp = check_status(resp, url)?;
        let file = fs::File::create(&tmp).await.map_err(TtsError::Ipc)?;
        (resp, file, 0)
    };
    if !append {
        hasher = Sha256::new();
    }

    let (downloaded, actual) = stream_body(
        resp,
        &mut file,
        &mut hasher,
        existing,
        entry,
        engine,
        file_idx,
        total_files,
        emitter,
    )
    .await?;

    if downloaded != entry.size || !actual.eq_ignore_ascii_case(&entry.sha256) {
        return Err(TtsError::Ttsd {
            code: "bundle_checksum_failed".to_string(),
            message: format!(
                "контрольная сумма {} не совпала с манифестом — файл удалён, \
                 бандл не считается установленным",
                entry.path
            ),
        });
    }

    std::fs::rename(&tmp, dest).map_err(TtsError::Ipc)?;
    // Final 100% tick — the throttle boundary rarely lands exactly on the
    // file size, so this guarantees the UI's progress bar fills.
    emit_progress(emitter, engine, entry, file_idx, total_files, downloaded);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn stream_body(
    resp: reqwest::Response,
    file: &mut fs::File,
    hasher: &mut Sha256,
    existing: u64,
    entry: &BundleFile,
    engine: &str,
    file_idx: u32,
    total_files: u32,
    emitter: &Emitter,
) -> Result<(u64, String), TtsError> {
    let mut downloaded = existing;
    let mut last_emit = existing;
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| TtsError::Ttsd {
            code: "bundle_download_failed".to_string(),
            message: format!("ошибка чтения {}: {e}", entry.asset),
        })?;
        file.write_all(&chunk).await.map_err(TtsError::Ipc)?;
        hasher.update(&chunk);
        downloaded += chunk.len() as u64;
        if downloaded - last_emit >= PROGRESS_EMIT_BYTES {
            emit_progress(emitter, engine, entry, file_idx, total_files, downloaded);
            last_emit = downloaded;
        }
    }
    file.flush().await.map_err(TtsError::Ipc)?;
    let hasher = std::mem::take(hasher);
    Ok((downloaded, format!("{:x}", hasher.finalize())))
}

fn emit_progress(
    emitter: &Emitter,
    engine: &str,
    entry: &BundleFile,
    file_idx: u32,
    total_files: u32,
    downloaded: u64,
) {
    (emitter)(
        "bundle_download_progress",
        json!({
            "engine": engine,
            "file": entry.path,
            "file_idx": file_idx,
            "total_files": total_files,
            "downloaded_bytes": downloaded,
            "total_bytes": entry.size,
        }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry_for(path: &str, contents: &[u8]) -> BundleFile {
        BundleFile {
            path: path.to_string(),
            asset: path.replace('/', "__"),
            size: contents.len() as u64,
            sha256: format!("{:x}", Sha256::new().chain_update(contents).finalize()),
        }
    }

    #[tokio::test]
    async fn file_is_valid_accepts_matching_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let contents = b"model bytes";
        let dest = dir.path().join("a.onnx");
        fs::write(&dest, contents).await.unwrap();
        assert!(file_is_valid(&dest, &entry_for("a.onnx", contents)).await);
    }

    #[tokio::test]
    async fn file_is_valid_rejects_missing_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let dest = dir.path().join("missing.onnx");
        assert!(!file_is_valid(&dest, &entry_for("missing.onnx", b"x")).await);
    }

    #[tokio::test]
    async fn file_is_valid_rejects_size_and_hash_mismatch() {
        let dir = tempfile::TempDir::new().unwrap();
        let dest = dir.path().join("a.onnx");
        fs::write(&dest, b"short").await.unwrap();
        assert!(!file_is_valid(&dest, &entry_for("a.onnx", b"much longer contents")).await);
        fs::write(&dest, b"corrupted!").await.unwrap();
        let entry = BundleFile {
            sha256: "0".repeat(64),
            ..entry_for("a.onnx", b"corrupted!")
        };
        assert!(!file_is_valid(&dest, &entry).await);
    }

    fn test_emitter() -> Emitter {
        std::sync::Arc::new(|_, _| {})
    }

    /// A complete `.partial` file from an interrupted run is verified and
    /// renamed without any HTTP request; the unreachable URL proves the
    /// network is never touched on this path.
    #[tokio::test]
    async fn complete_partial_file_is_verified_and_renamed_without_a_request() {
        let dir = tempfile::TempDir::new().unwrap();
        let dest = dir.path().join("model.onnx");
        let contents = b"already downloaded";
        let entry = entry_for("model.onnx", contents);
        fs::write(partial_path(&dest), contents).await.unwrap();

        let client = reqwest::Client::new();
        download_one(
            &client,
            "http://127.0.0.1:1/never",
            &dest,
            &entry,
            "teratts",
            0,
            1,
            &test_emitter(),
        )
        .await
        .expect("verified partial is promoted");

        assert_eq!(fs::read(&dest).await.unwrap(), contents);
        assert!(!partial_path(&dest).exists());
    }

    /// A complete but corrupt `.partial` file is removed and the download
    /// restarts (here it fails on the unreachable URL, so no stale partial
    /// survives to be mistaken for an installed file).
    #[tokio::test]
    async fn corrupt_partial_file_is_removed_before_restarting() {
        let dir = tempfile::TempDir::new().unwrap();
        let dest = dir.path().join("model.onnx");
        let entry = entry_for("model.onnx", b"expected bytes");
        fs::write(partial_path(&dest), b"wrong bytes!!!!!")
            .await
            .unwrap();

        let client = reqwest::Client::new();
        let result = download_one(
            &client,
            "http://127.0.0.1:1/never",
            &dest,
            &entry,
            "teratts",
            0,
            1,
            &test_emitter(),
        )
        .await;

        assert!(result.is_err());
        assert!(!partial_path(&dest).exists());
        assert!(!dest.exists());
    }

    /// A manifest may place a file in a subdirectory (TeraTTS does); the
    /// downloader must create that directory before writing the `.partial`.
    #[tokio::test]
    async fn nested_destination_creates_parent_directory() {
        use std::io::{Read as _, Write as _};

        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let contents = b"nested model bytes";
        let response = {
            let head = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                contents.len()
            );
            let mut bytes = head.into_bytes();
            bytes.extend_from_slice(contents);
            bytes
        };
        std::thread::spawn(move || {
            if let Ok((mut sock, _)) = listener.accept() {
                let mut buf = [0u8; 1024];
                let _ = sock.read(&mut buf);
                let _ = sock.write_all(&response);
            }
        });

        let dir = tempfile::TempDir::new().unwrap();
        let dest = dir.path().join("models").join("vocoder.onnx");
        let entry = entry_for("models/vocoder.onnx", contents);
        download_one(
            &reqwest::Client::new(),
            &format!("http://{addr}/vocoder.onnx"),
            &dest,
            &entry,
            "teratts",
            0,
            1,
            &test_emitter(),
        )
        .await
        .expect("nested download succeeds");
        assert_eq!(fs::read(&dest).await.unwrap(), contents);
    }

    #[test]
    fn partial_path_uses_partial_extension() {
        assert_eq!(
            partial_path(Path::new("/b/tts_main.onnx")),
            PathBuf::from("/b/tts_main.partial")
        );
    }

    #[test]
    fn safe_path_rejects_escapes() {
        let root = Path::new("/bundle");
        assert!(safe_path(root, "../etc/passwd").is_err());
        assert!(safe_path(root, "/etc/passwd").is_err());
        assert!(safe_path(root, "").is_err());
        assert!(safe_path(root, "models/../../x").is_err());
        assert_eq!(
            safe_path(root, "models/vocoder.onnx").unwrap(),
            PathBuf::from("/bundle/models/vocoder.onnx")
        );
    }

    #[tokio::test]
    async fn write_atomic_leaves_no_partial_behind() {
        let dir = tempfile::TempDir::new().unwrap();
        let dest = dir.path().join("manifest.json");
        write_atomic(&dest, b"{}").await.unwrap();
        assert_eq!(fs::read(&dest).await.unwrap(), b"{}");
        assert!(!partial_path(&dest).exists());
    }
}
