//! TTS subprocess manager.
//!
//! Manages a long-lived `uv run python -m ttsd` child process. All requests are
//! serialized through an `mpsc` channel and handled by a single driver task, which
//! matches ttsd's own single-threaded design.
//!
//! # Auto-restart
//! TODO: v1 does not auto-restart on crash. The driver task exits when the subprocess
//! dies. Callers must detect the `Died` error and re-create via `TtsSubprocess::spawn()`.

use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, oneshot};
use tokio::time::timeout;
use tracing::{info, warn};

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors returned by [`TtsSubprocess`].
#[derive(Debug, Error)]
pub enum TtsError {
    /// Failed to spawn the ttsd subprocess.
    #[error("failed to spawn ttsd subprocess: {0}")]
    Spawn(#[source] std::io::Error),

    /// I/O error while communicating with the subprocess over stdin/stdout.
    #[error("IPC I/O error: {0}")]
    Ipc(#[source] std::io::Error),

    /// JSON serialization/deserialization error.
    #[error("IPC JSON error: {0}")]
    Json(#[source] serde_json::Error),

    /// ttsd returned an error response (`ok: false`).
    #[error("ttsd error [{code}]: {message}")]
    Ttsd { code: String, message: String },

    /// The subprocess has exited unexpectedly.
    #[error("ttsd subprocess has exited")]
    Died,

    /// Request did not complete within the allowed time.
    #[error("ttsd request timed out")]
    Timeout,
}

// ---------------------------------------------------------------------------
// Protocol types — requests
// ---------------------------------------------------------------------------

/// Requests sent to the ttsd subprocess over stdin (NDJSON).
#[derive(Debug, Serialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum TtsRequest {
    Warmup,
    Synthesize {
        text: String,
        speaker: String,
        sample_rate: u32,
        out_wav: String,
        /// Optional char mapping from the Rust pipeline for precise `original_pos` mapping.
        #[serde(skip_serializing_if = "Option::is_none")]
        char_mapping: Option<Vec<CharMappingEntry>>,
    },
    Shutdown,
}

/// One entry in the optional `char_mapping` array sent with a synthesize request.
/// Field names match the Python pydantic model in `ttsd/ttsd/protocol.py`.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CharMappingEntry {
    pub norm_start: usize,
    pub norm_end: usize,
    pub orig_start: usize,
    pub orig_end: usize,
}

// ---------------------------------------------------------------------------
// Protocol types — responses
// ---------------------------------------------------------------------------

/// Untagged union that covers both success (`ok: true`) and failure (`ok: false`)
/// responses from ttsd. Deserialized from a single JSON line.
///
/// The custom impl checks the `ok` field first to avoid ambiguity in the untagged
/// representation: `OkPayload` uses `flatten` and would otherwise swallow error
/// objects too.
#[derive(Debug)]
pub enum TtsResponse {
    Ok(OkPayload),
    Err(ErrPayload),
}

impl<'de> Deserialize<'de> for TtsResponse {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let v = Value::deserialize(deserializer)?;
        let ok = v
            .get("ok")
            .and_then(Value::as_bool)
            .ok_or_else(|| serde::de::Error::missing_field("ok"))?;
        if ok {
            let payload = OkPayload::deserialize(v).map_err(serde::de::Error::custom)?;
            Ok(TtsResponse::Ok(payload))
        } else {
            let payload = ErrPayload::deserialize(v).map_err(serde::de::Error::custom)?;
            Ok(TtsResponse::Err(payload))
        }
    }
}

/// A successful ttsd response. Command-specific fields are collected into `extra`.
#[derive(Debug, Deserialize)]
pub struct OkPayload {
    #[allow(dead_code)]
    pub ok: bool,
    #[serde(flatten)]
    pub extra: Value,
}

/// A failed ttsd response.
#[derive(Debug, Deserialize)]
pub struct ErrPayload {
    #[allow(dead_code)]
    pub ok: bool,
    pub error: String,
    pub message: String,
}

// ---------------------------------------------------------------------------
// Synthesize output
// ---------------------------------------------------------------------------

/// Word-level timestamp returned by a successful synthesize request.
#[derive(Debug, Deserialize, Clone)]
pub struct WordTimestamp {
    pub word: String,
    pub start: f64,
    pub end: f64,
    /// `[start, end]` char offsets in the original (pre-normalization) text.
    pub original_pos: (usize, usize),
}

/// Structured output of a successful synthesize call.
#[derive(Debug, Deserialize)]
pub struct SynthesizeOutput {
    pub timestamps: Vec<WordTimestamp>,
    pub duration_sec: f64,
}

// ---------------------------------------------------------------------------
// Internal driver message
// ---------------------------------------------------------------------------

/// A request sent through the mpsc channel to the driver task.
struct DriverRequest {
    payload: TtsRequest,
    reply: oneshot::Sender<Result<TtsResponse, TtsError>>,
}

// ---------------------------------------------------------------------------
// TtsSubprocess
// ---------------------------------------------------------------------------

/// Handle to the ttsd subprocess manager.
///
/// All calls are async and execute one at a time (Silero is not thread-safe).
/// The underlying driver task owns the subprocess and serializes all I/O.
pub struct TtsSubprocess {
    sender: mpsc::Sender<DriverRequest>,
}

impl TtsSubprocess {
    /// Spawn the ttsd subprocess and start the driver task.
    ///
    /// `ttsd_dir` is the directory from which `uv run python -m ttsd` is executed.
    pub fn spawn(ttsd_dir: PathBuf) -> Result<Self, TtsError> {
        let child = Command::new("uv")
            .args(["run", "python", "-m", "ttsd"])
            .current_dir(&ttsd_dir)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            // kill_on_drop: process is killed when the Child value is dropped
            .kill_on_drop(true)
            .spawn()
            .map_err(TtsError::Spawn)?;

        let (tx, rx) = mpsc::channel::<DriverRequest>(1);
        tokio::spawn(driver_task(child, rx));

        Ok(Self { sender: tx })
    }

    /// Send a raw request and wait for the response.
    async fn send(&self, req: TtsRequest) -> Result<TtsResponse, TtsError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.sender
            .send(DriverRequest {
                payload: req,
                reply: reply_tx,
            })
            .await
            .map_err(|_| TtsError::Died)?;

        reply_rx.await.map_err(|_| TtsError::Died)?
    }

    /// Load the Silero model. Must be called once before `synthesize`.
    pub async fn warmup(&self) -> Result<(), TtsError> {
        let resp = self.send(TtsRequest::Warmup).await?;
        match resp {
            TtsResponse::Ok(_) => Ok(()),
            TtsResponse::Err(e) => Err(TtsError::Ttsd {
                code: e.error,
                message: e.message,
            }),
        }
    }

    /// Synthesize `text` and write the WAV file to `out_wav`.
    ///
    /// Timeout: 5 minutes. Returns timestamps and duration from ttsd.
    pub async fn synthesize(
        &self,
        text: String,
        speaker: String,
        sample_rate: u32,
        out_wav: String,
        char_mapping: Option<Vec<CharMappingEntry>>,
    ) -> Result<SynthesizeOutput, TtsError> {
        const SYNTHESIZE_TIMEOUT: Duration = Duration::from_secs(5 * 60);

        let req = TtsRequest::Synthesize {
            text,
            speaker,
            sample_rate,
            out_wav,
            char_mapping,
        };
        let resp = timeout(SYNTHESIZE_TIMEOUT, self.send(req))
            .await
            .map_err(|_| TtsError::Timeout)??;

        match resp {
            TtsResponse::Ok(ok) => {
                let output: SynthesizeOutput =
                    serde_json::from_value(ok.extra).map_err(TtsError::Json)?;
                Ok(output)
            }
            TtsResponse::Err(e) => Err(TtsError::Ttsd {
                code: e.error,
                message: e.message,
            }),
        }
    }

    /// Request graceful shutdown.
    ///
    /// Waits up to 5 s for the ttsd response. If the subprocess does not respond
    /// or does not exit cleanly within that window, the driver task force-kills
    /// the process (see `driver_task`).
    pub async fn shutdown(&self) -> Result<(), TtsError> {
        const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);
        let resp = timeout(SHUTDOWN_TIMEOUT, self.send(TtsRequest::Shutdown))
            .await
            .map_err(|_| TtsError::Timeout)??;
        match resp {
            TtsResponse::Ok(_) => Ok(()),
            TtsResponse::Err(e) => Err(TtsError::Ttsd {
                code: e.error,
                message: e.message,
            }),
        }
    }
}

// ---------------------------------------------------------------------------
// Driver task
// ---------------------------------------------------------------------------

/// Owns the subprocess and its stdin/stdout. Serializes all requests one at a time.
async fn driver_task(mut child: Child, mut rx: mpsc::Receiver<DriverRequest>) {
    // stdin is held in an Option so it can be dropped (closing the pipe / sending EOF)
    // before we wait on the child during graceful shutdown.
    let mut stdin = Some(child.stdin.take().expect("stdin was piped"));
    let stdout = child.stdout.take().expect("stdout was piped");
    let stderr = child.stderr.take().expect("stderr was piped");

    // Forward stderr lines to tracing asynchronously.
    tokio::spawn(async move {
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            info!(target: "ttsd", "{}", line);
        }
    });

    let mut stdout_lines = BufReader::new(stdout).lines();

    while let Some(req) = rx.recv().await {
        let is_shutdown = matches!(&req.payload, TtsRequest::Shutdown);
        let result = match stdin.as_mut() {
            Some(s) => handle_one_request(s, &mut stdout_lines, req.payload).await,
            None => Err(TtsError::Died),
        };
        let _ = req.reply.send(result);

        if is_shutdown {
            // Drop stdin → ttsd reads EOF and can exit cleanly.
            drop(stdin.take());
            match timeout(Duration::from_secs(5), child.wait()).await {
                Ok(Ok(status)) => {
                    info!(target: "ttsd", "exited cleanly: {status}");
                }
                Ok(Err(e)) => {
                    warn!(target: "ttsd", "wait() failed: {e}");
                }
                Err(_) => {
                    warn!(target: "ttsd", "did not exit within 5 s, sending SIGKILL");
                    let _ = child.start_kill();
                    let _ = child.wait().await;
                }
            }
            return;
        }
    }
}

/// Write one request to stdin and read one response line from stdout.
async fn handle_one_request(
    stdin: &mut tokio::process::ChildStdin,
    stdout_lines: &mut tokio::io::Lines<BufReader<tokio::process::ChildStdout>>,
    req: TtsRequest,
) -> Result<TtsResponse, TtsError> {
    // Serialize request to a single JSON line.
    let mut line = serde_json::to_string(&req).map_err(TtsError::Json)?;
    line.push('\n');

    stdin
        .write_all(line.as_bytes())
        .await
        .map_err(TtsError::Ipc)?;
    stdin.flush().await.map_err(TtsError::Ipc)?;

    // Read the response line. None means the subprocess closed stdout (i.e. died).
    let response_line = stdout_lines
        .next_line()
        .await
        .map_err(TtsError::Ipc)?
        .ok_or(TtsError::Died)?;

    serde_json::from_str::<TtsResponse>(&response_line).map_err(TtsError::Json)
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- TtsRequest serialization ---

    #[test]
    fn warmup_serializes_correctly() {
        let json = serde_json::to_string(&TtsRequest::Warmup).unwrap();
        let v: Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["cmd"], "warmup");
        // Warmup has no extra fields.
        assert_eq!(v.as_object().unwrap().len(), 1);
    }

    #[test]
    fn shutdown_serializes_correctly() {
        let json = serde_json::to_string(&TtsRequest::Shutdown).unwrap();
        let v: Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["cmd"], "shutdown");
        assert_eq!(v.as_object().unwrap().len(), 1);
    }

    #[test]
    fn synthesize_serializes_all_required_fields() {
        let req = TtsRequest::Synthesize {
            text: "привет мир".to_string(),
            speaker: "xenia".to_string(),
            sample_rate: 48000,
            out_wav: "/tmp/test.wav".to_string(),
            char_mapping: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        let v: Value = serde_json::from_str(&json).unwrap();

        assert_eq!(v["cmd"], "synthesize");
        assert_eq!(v["text"], "привет мир");
        assert_eq!(v["speaker"], "xenia");
        assert_eq!(v["sample_rate"], 48000);
        assert_eq!(v["out_wav"], "/tmp/test.wav");
        // char_mapping is None → must be absent (skip_serializing_if)
        assert!(v.get("char_mapping").is_none());
    }

    #[test]
    fn synthesize_with_char_mapping_includes_it() {
        let mapping = vec![CharMappingEntry {
            norm_start: 0,
            norm_end: 6,
            orig_start: 0,
            orig_end: 12,
        }];
        let req = TtsRequest::Synthesize {
            text: "привет".to_string(),
            speaker: "xenia".to_string(),
            sample_rate: 48000,
            out_wav: "/tmp/test.wav".to_string(),
            char_mapping: Some(mapping),
        };
        let json = serde_json::to_string(&req).unwrap();
        let v: Value = serde_json::from_str(&json).unwrap();

        assert_eq!(v["cmd"], "synthesize");
        let cm = v["char_mapping"]
            .as_array()
            .expect("char_mapping should be present");
        assert_eq!(cm.len(), 1);
        assert_eq!(cm[0]["norm_start"], 0);
        assert_eq!(cm[0]["norm_end"], 6);
        assert_eq!(cm[0]["orig_start"], 0);
        assert_eq!(cm[0]["orig_end"], 12);
    }

    // --- TtsResponse deserialization ---

    #[test]
    fn response_parses_warmup_ok() {
        let json = r#"{"ok":true,"version":"0.1.0"}"#;
        let resp: TtsResponse = serde_json::from_str(json).unwrap();
        match resp {
            TtsResponse::Ok(ok) => {
                assert!(ok.ok);
                assert_eq!(ok.extra["version"], "0.1.0");
            }
            TtsResponse::Err(_) => panic!("expected Ok variant"),
        }
    }

    #[test]
    fn response_parses_error() {
        let json =
            r#"{"ok":false,"error":"model_not_loaded","message":"Silero model is not loaded"}"#;
        let resp: TtsResponse = serde_json::from_str(json).unwrap();
        match resp {
            TtsResponse::Err(e) => {
                assert!(!e.ok);
                assert_eq!(e.error, "model_not_loaded");
                assert_eq!(e.message, "Silero model is not loaded");
            }
            TtsResponse::Ok(_) => panic!("expected Err variant"),
        }
    }

    #[test]
    fn response_parses_synthesize_ok() {
        let json = r#"{
            "ok": true,
            "timestamps": [
                {"word":"привет","start":0.0,"end":0.5,"original_pos":[0,6]},
                {"word":"мир","start":0.55,"end":0.9,"original_pos":[7,10]}
            ],
            "duration_sec": 0.9
        }"#;
        let resp: TtsResponse = serde_json::from_str(json).unwrap();
        match resp {
            TtsResponse::Ok(ok) => {
                let output: SynthesizeOutput = serde_json::from_value(ok.extra).unwrap();
                assert_eq!(output.timestamps.len(), 2);
                assert_eq!(output.timestamps[0].word, "привет");
                assert!((output.duration_sec - 0.9).abs() < f64::EPSILON);
            }
            TtsResponse::Err(_) => panic!("expected Ok variant"),
        }
    }

    // --- CharMappingEntry serialization ---

    #[test]
    fn char_mapping_entry_serializes_to_pydantic_shape() {
        let entry = CharMappingEntry {
            norm_start: 5,
            norm_end: 11,
            orig_start: 3,
            orig_end: 15,
        };
        let json = serde_json::to_string(&entry).unwrap();
        let v: Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["norm_start"], 5);
        assert_eq!(v["norm_end"], 11);
        assert_eq!(v["orig_start"], 3);
        assert_eq!(v["orig_end"], 15);
    }
}
