//! On-demand download of Piper voice files (Phase 4 of #42).
//!
//! Each voice ships as two files in `<voices_dir>/<voice_id>/`:
//! - `ru_RU-<voice_id>-medium.onnx.json` — voice config (~4 KB)
//! - `ru_RU-<voice_id>-medium.onnx`      — VITS weights (~60 MB)
//!
//! The download is split into two requests because Piper's loader needs the
//! `.onnx.json` first (it carries the sample rate and phoneme map). Each
//! file is staged in `<dest>.partial` and atomically renamed into place
//! once fully written, so a crashed download cannot leave a half-baked
//! `.onnx` lying around for the loader to choke on next launch.
//!
//! Progress is reported through the supervisor's `Emitter` callback so the
//! engine layer stays free of `tauri::AppHandle`. Three event names:
//! - `voice_download_started`  — `{ engine, voice }`
//! - `voice_download_progress` — `{ engine, voice, file_kind, file_idx,
//!                                  total_files, downloaded_bytes,
//!                                  total_bytes }`
//! - `voice_download_finished` — `{ engine, voice }`
//!
//! Failures emit a `voice_download_finished` with `{ ok: false, message }`
//! so the frontend notification has a single terminal event to listen for.

use std::path::Path;

use futures_util::StreamExt;
use serde_json::json;
use tokio::fs;
use tokio::io::AsyncWriteExt;

use super::catalog;
use crate::tts::supervisor::Emitter;
use crate::tts::TtsError;

/// Throttle progress events to ~1 per 256 KB so the IPC bridge does not
/// drown the renderer.
const PROGRESS_EMIT_BYTES: u64 = 256 * 1024;

/// Download both files for `voice_id` into `voices_dir`. Idempotent: skips
/// any file already present on disk. Atomic per-file (write to `.partial`,
/// then rename). Cancellation is not supported in this revision — callers
/// drop the future to abort, leaving any `.partial` files for the next run
/// to overwrite.
pub async fn download_voice(
    voices_dir: &Path,
    voice_id: &str,
    emitter: &Emitter,
) -> Result<(), TtsError> {
    let voice = catalog::lookup(voice_id).ok_or_else(|| TtsError::Ttsd {
        code: "voice_unknown".to_string(),
        message: format!("неизвестный голос Piper: \"{voice_id}\""),
    })?;

    let dest_dir = voices_dir.join(voice_id);
    fs::create_dir_all(&dest_dir).await.map_err(TtsError::Ipc)?;

    let json_filename = format!("ru_RU-{voice_id}-medium.onnx.json");
    let onnx_filename = format!("ru_RU-{voice_id}-medium.onnx");
    let json_path = dest_dir.join(&json_filename);
    let onnx_path = dest_dir.join(&onnx_filename);

    (emitter)(
        "voice_download_started",
        json!({ "engine": "piper", "voice": voice_id }),
    );

    let result = async {
        // Config first — Piper's loader reads it before the model.
        if !json_path.exists() {
            download_one(
                voice.config_url,
                &json_path,
                voice_id,
                "json",
                0,
                2,
                emitter,
            )
            .await?;
        } else {
            emit_skip(emitter, voice_id, "json", 0, 2);
        }
        if !onnx_path.exists() {
            download_one(voice.model_url, &onnx_path, voice_id, "onnx", 1, 2, emitter).await?;
        } else {
            emit_skip(emitter, voice_id, "onnx", 1, 2);
        }
        Ok::<_, TtsError>(())
    }
    .await;

    match &result {
        Ok(()) => {
            (emitter)(
                "voice_download_finished",
                json!({ "engine": "piper", "voice": voice_id, "ok": true }),
            );
        }
        Err(e) => {
            (emitter)(
                "voice_download_finished",
                json!({
                    "engine": "piper",
                    "voice": voice_id,
                    "ok": false,
                    "message": e.to_string(),
                }),
            );
        }
    }
    result
}

fn emit_skip(emitter: &Emitter, voice_id: &str, kind: &str, idx: u32, total: u32) {
    (emitter)(
        "voice_download_progress",
        json!({
            "engine": "piper",
            "voice": voice_id,
            "file_kind": kind,
            "file_idx": idx,
            "total_files": total,
            "downloaded_bytes": 0u64,
            "total_bytes": 0u64,
            "skipped": true,
        }),
    );
}

async fn download_one(
    url: &str,
    dest: &Path,
    voice_id: &str,
    file_kind: &str,
    file_idx: u32,
    total_files: u32,
    emitter: &Emitter,
) -> Result<(), TtsError> {
    let resp = reqwest::get(url).await.map_err(|e| TtsError::Ttsd {
        code: "voice_download_failed".to_string(),
        message: format!("HTTP GET {url} failed: {e}"),
    })?;
    if !resp.status().is_success() {
        return Err(TtsError::Ttsd {
            code: "voice_download_failed".to_string(),
            message: format!("HTTP {} for {url}", resp.status()),
        });
    }
    let total_bytes = resp.content_length();

    let tmp = dest.with_extension("partial");
    let mut file = fs::File::create(&tmp).await.map_err(TtsError::Ipc)?;
    let mut downloaded: u64 = 0;
    let mut last_emit: u64 = 0;
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| TtsError::Ttsd {
            code: "voice_download_failed".to_string(),
            message: format!("chunk read failed: {e}"),
        })?;
        file.write_all(&chunk).await.map_err(TtsError::Ipc)?;
        downloaded += chunk.len() as u64;
        if downloaded - last_emit >= PROGRESS_EMIT_BYTES {
            (emitter)(
                "voice_download_progress",
                json!({
                    "engine": "piper",
                    "voice": voice_id,
                    "file_kind": file_kind,
                    "file_idx": file_idx,
                    "total_files": total_files,
                    "downloaded_bytes": downloaded,
                    "total_bytes": total_bytes,
                }),
            );
            last_emit = downloaded;
        }
    }
    file.flush().await.map_err(TtsError::Ipc)?;
    drop(file);
    fs::rename(&tmp, dest).await.map_err(TtsError::Ipc)?;

    // Final 100% tick — `bytes_stream` rarely lands exactly on the
    // throttle boundary, so this guarantees the UI's progress bar fills.
    (emitter)(
        "voice_download_progress",
        json!({
            "engine": "piper",
            "voice": voice_id,
            "file_kind": file_kind,
            "file_idx": file_idx,
            "total_files": total_files,
            "downloaded_bytes": downloaded,
            "total_bytes": total_bytes,
        }),
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tts::supervisor::test_helpers::recording_emitter;

    #[tokio::test]
    async fn download_voice_rejects_unknown_voice_id() {
        let dir = tempfile::TempDir::new().unwrap();
        let (emitter, _) = recording_emitter();
        let err = download_voice(dir.path(), "does-not-exist", &emitter)
            .await
            .unwrap_err();
        match err {
            TtsError::Ttsd { code, .. } => assert_eq!(code, "voice_unknown"),
            other => panic!("expected voice_unknown, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn download_voice_skips_files_already_on_disk() {
        // Pre-populate both files; the function must not touch the network
        // and must emit a `skipped: true` progress event for each file.
        let dir = tempfile::TempDir::new().unwrap();
        let voice = "ruslan";
        let voice_dir = dir.path().join(voice);
        fs::create_dir_all(&voice_dir).await.unwrap();
        fs::write(voice_dir.join("ru_RU-ruslan-medium.onnx"), b"x")
            .await
            .unwrap();
        fs::write(voice_dir.join("ru_RU-ruslan-medium.onnx.json"), b"{}")
            .await
            .unwrap();

        let (emitter, log) = recording_emitter();
        download_voice(dir.path(), voice, &emitter).await.unwrap();

        let log = log.lock().unwrap();
        let names: Vec<&str> = log.iter().map(|(n, _)| n.as_str()).collect();
        assert!(names.contains(&"voice_download_started"));
        assert!(names.contains(&"voice_download_finished"));
        let skipped = log
            .iter()
            .filter(|(n, p)| n == "voice_download_progress" && p["skipped"] == true)
            .count();
        assert_eq!(skipped, 2, "expected 2 skipped progress events");
    }
}
