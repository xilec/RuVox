//! On-demand download of the Silero Native model bundle.
//!
//! The bundle ships as files attached to the `silero-models-v5_ru` GitHub
//! Release in the RuVox repo. `manifest.json` is fetched first (it carries
//! the per-file size + sha256 list), then every file it lists:
//!
//! - files already on disk with a matching size + sha256 are skipped
//!   (idempotent re-runs);
//! - each download streams to `<name>.partial`, is sha256-verified while
//!   streaming, and only then renamed into place — a partial or corrupt file
//!   is never treated as installed (the availability probe and the engine
//!   loader both key off the real file names);
//! - a checksum mismatch or any I/O failure removes the `.partial` file and
//!   aborts with a typed error, leaving the engine unavailable.
//!
//! Progress mirrors the Piper voice download, through the shared `Emitter`:
//! - `bundle_download_started`  — `{ engine }`
//! - `bundle_download_progress` — `{ engine, file, file_idx, total_files,
//!                                  downloaded_bytes, total_bytes }`,
//!   throttled to ~1 event per 256 KB, plus `skipped: true` for files
//!   already present and valid
//! - `bundle_download_finished` — `{ engine, ok: true }` or
//!   `{ engine, ok: false, message }`

use std::path::{Path, PathBuf};

use futures_util::StreamExt;
use serde_json::json;
use sha2::{Digest, Sha256};
use silero_native::bundle::{Manifest, ManifestFile};
use tokio::fs;
use tokio::io::AsyncWriteExt;

use crate::tts::supervisor::Emitter;
use crate::tts::TtsError;

/// Base URL of the GitHub Release that hosts the bundle files. The release
/// may not exist yet — a 404 surfaces as a typed `bundle_download_failed`
/// error (and a `bundle_download_finished { ok: false }` event), never a
/// panic or a half-installed bundle.
pub const BUNDLE_RELEASE_BASE_URL: &str =
    "https://github.com/xilec/RuVox/releases/download/silero-models-v5_ru";

/// Throttle progress events to ~1 per 256 KB so the IPC bridge does not
/// drown the renderer (same value as the Piper voice download).
const PROGRESS_EMIT_BYTES: u64 = 256 * 1024;

/// Download and verify the whole bundle into `bundle_dir`. Emits the
/// `bundle_download_*` event sequence; the returned `Result` only reports
/// the final outcome so the frontend can show one terminal notification.
pub async fn download_bundle(bundle_dir: &Path, emitter: &Emitter) -> Result<(), TtsError> {
    (emitter)(
        "bundle_download_started",
        json!({ "engine": "silero_native" }),
    );

    let result = download_inner(bundle_dir, emitter).await;

    match &result {
        Ok(()) => {
            (emitter)(
                "bundle_download_finished",
                json!({ "engine": "silero_native", "ok": true }),
            );
        }
        Err(e) => {
            (emitter)(
                "bundle_download_finished",
                json!({
                    "engine": "silero_native",
                    "ok": false,
                    "message": e.to_string(),
                }),
            );
        }
    }
    result
}

async fn download_inner(bundle_dir: &Path, emitter: &Emitter) -> Result<(), TtsError> {
    fs::create_dir_all(bundle_dir)
        .await
        .map_err(TtsError::Ipc)?;

    let client = reqwest::Client::builder()
        // The whole bundle is ~230 MB: 30 min overall is generous even on a
        // slow link, while the connect timeout keeps a dead host from
        // hanging the download forever.
        .connect_timeout(std::time::Duration::from_secs(60))
        .timeout(std::time::Duration::from_secs(30 * 60))
        .build()
        .map_err(|e| TtsError::Ttsd {
            code: "bundle_download_failed".to_string(),
            message: format!("не удалось инициализировать HTTP-клиент: {e}"),
        })?;

    // 1. Manifest first — it is the source of truth for the file list and
    // the expected checksums. Validated in memory through the crate's own
    // parser (rejects malformed JSON and empty lists) *before* being
    // persisted atomically, so a broken download never leaves a corrupt
    // manifest.json behind for the availability probe to trip over.
    let manifest_url = format!("{BUNDLE_RELEASE_BASE_URL}/manifest.json");
    let manifest_bytes = fetch_whole(&client, &manifest_url).await?;
    let manifest = Manifest::parse(&manifest_bytes).map_err(|e| TtsError::Ttsd {
        code: "bundle_manifest_invalid".to_string(),
        message: format!("манифест бандла повреждён: {e}"),
    })?;
    write_atomic(&bundle_dir.join("manifest.json"), &manifest_bytes).await?;

    // 2. Bundle files, in manifest order.
    let total_files = manifest.files.len() as u32;
    for (idx, entry) in manifest.files.iter().enumerate() {
        let dest = manifest
            .file_path(bundle_dir, &entry.path)
            .map_err(|e| TtsError::Ttsd {
                code: "bundle_path_unsafe".to_string(),
                message: format!("небезопасный путь в манифесте: {e}"),
            })?;

        if file_is_valid(&dest, entry).await {
            (emitter)(
                "bundle_download_progress",
                json!({
                    "engine": "silero_native",
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

        let url = format!("{BUNDLE_RELEASE_BASE_URL}/{}", entry.path);
        download_one(
            &client,
            &url,
            &dest,
            entry,
            idx as u32,
            total_files,
            emitter,
        )
        .await?;
    }

    Ok(())
}

/// `true` when `dest` exists and matches the manifest's size + sha256.
/// Any I/O error counts as "not valid" — the file is re-downloaded.
async fn file_is_valid(dest: &Path, entry: &ManifestFile) -> bool {
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

/// sha256 of a local file, read in 1 MiB chunks (bundle files reach ~117 MB).
///
/// Deliberate duplicate of the sync `sha256_file` in the engine crate
/// (`silero-native/src/bundle.rs`): this is the async downloader side, and
/// sharing one helper would drag a runtime dependency across the crate
/// boundary. Keep both.
async fn sha256_file(path: &Path) -> std::io::Result<String> {
    let mut file = fs::File::open(path).await?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = tokio::io::AsyncReadExt::read(&mut file, &mut buf).await?;
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
        .map(|b| b.to_vec())
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
async fn download_one(
    client: &reqwest::Client,
    url: &str,
    dest: &Path,
    entry: &ManifestFile,
    file_idx: u32,
    total_files: u32,
    emitter: &Emitter,
) -> Result<(), TtsError> {
    let result = download_one_inner(client, url, dest, entry, file_idx, total_files, emitter).await;
    if result.is_err() {
        let _ = fs::remove_file(partial_path(dest)).await;
    }
    result
}

async fn download_one_inner(
    client: &reqwest::Client,
    url: &str,
    dest: &Path,
    entry: &ManifestFile,
    file_idx: u32,
    total_files: u32,
    emitter: &Emitter,
) -> Result<(), TtsError> {
    let resp = client.get(url).send().await.map_err(|e| TtsError::Ttsd {
        code: "bundle_download_failed".to_string(),
        message: format!("не удалось скачать {url}: {e}"),
    })?;
    let resp = check_status(resp, url)?;

    let tmp = partial_path(dest);
    let mut file = fs::File::create(&tmp).await.map_err(TtsError::Ipc)?;
    let mut hasher = Sha256::new();
    let mut downloaded: u64 = 0;
    let mut last_emit: u64 = 0;
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| TtsError::Ttsd {
            code: "bundle_download_failed".to_string(),
            message: format!("ошибка чтения {url}: {e}"),
        })?;
        file.write_all(&chunk).await.map_err(TtsError::Ipc)?;
        hasher.update(&chunk);
        downloaded += chunk.len() as u64;
        if downloaded - last_emit >= PROGRESS_EMIT_BYTES {
            emit_progress(emitter, entry, file_idx, total_files, downloaded);
            last_emit = downloaded;
        }
    }
    file.flush().await.map_err(TtsError::Ipc)?;
    drop(file);

    let actual = format!("{:x}", hasher.finalize());
    if !actual.eq_ignore_ascii_case(&entry.sha256) {
        return Err(TtsError::Ttsd {
            code: "bundle_checksum_failed".to_string(),
            message: format!(
                "контрольная сумма {} не совпала с манифестом — файл удалён, \
                 бандл не считается установленным",
                entry.path
            ),
        });
    }

    fs::rename(&tmp, dest).await.map_err(TtsError::Ipc)?;

    // Final 100% tick — the throttle boundary rarely lands exactly on the
    // file size, so this guarantees the UI's progress bar fills.
    emit_progress(emitter, entry, file_idx, total_files, downloaded);
    Ok(())
}

fn emit_progress(
    emitter: &Emitter,
    entry: &ManifestFile,
    file_idx: u32,
    total_files: u32,
    downloaded: u64,
) {
    (emitter)(
        "bundle_download_progress",
        json!({
            "engine": "silero_native",
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

    fn entry_for(path: &str, contents: &[u8]) -> ManifestFile {
        ManifestFile {
            path: path.to_string(),
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
    async fn file_is_valid_rejects_size_mismatch() {
        let dir = tempfile::TempDir::new().unwrap();
        let dest = dir.path().join("a.onnx");
        fs::write(&dest, b"short").await.unwrap();
        // Entry claims a different size (with a hash matching that size).
        let entry = entry_for("a.onnx", b"much longer contents");
        assert!(!file_is_valid(&dest, &entry).await);
    }

    #[tokio::test]
    async fn file_is_valid_rejects_hash_mismatch() {
        let dir = tempfile::TempDir::new().unwrap();
        let contents = b"corrupted!";
        let dest = dir.path().join("a.onnx");
        fs::write(&dest, contents).await.unwrap();
        // Same size, different hash.
        let entry = ManifestFile {
            sha256: "0".repeat(64),
            ..entry_for("a.onnx", contents)
        };
        assert!(!file_is_valid(&dest, &entry).await);
    }

    #[test]
    fn partial_path_uses_partial_extension() {
        assert_eq!(
            partial_path(Path::new("/b/tts_main.onnx")),
            PathBuf::from("/b/tts_main.partial")
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
