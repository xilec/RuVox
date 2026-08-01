//! Model bundle loading: manifest parsing, per-file sha256 verification and
//! ONNX session initialization.
//!
//! The bundle format is produced by `silero-native/export` and documented in
//! `export/README.md`. Every file listed in `manifest.json` is hashed before
//! any session is opened — a corrupt file must never reach ONNX Runtime.

use std::any::Any;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::time::Instant;

use ort::session::Session;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tracing::{debug, info, instrument};

use crate::error::{EngineError, Result};

/// Render a scoped-thread panic payload for error messages (same downcast
/// as `synthesize_with_fallback` in `lib.rs`).
fn panic_message(payload: &Box<dyn Any + Send>) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|s| (*s).to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "unknown panic".to_string())
}

/// Names of the ONNX models the engine opens, relative to the bundle root.
pub const TTS_MAIN: &str = "tts_main.onnx";
pub const ISTFT: &str = "istft.onnx";
pub const PQMF_24K: &str = "pqmf_24k.onnx";
pub const PQMF_8K: &str = "pqmf_8k.onnx";
pub const HOMOSOLVER: &str = "homosolver.onnx";
pub const ACCENTOR_TENSOR: &str = "accentor_tensor.onnx";

/// `manifest.json` — integrity record written by the exporter.
#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
    pub model_id: String,
    // Provenance fields, kept so the manifest mirrors the exporter's schema
    // and stays debuggable by hand. Intentionally unused by the loader —
    // do not "clean up".
    pub opset: u32,
    pub export_date_utc: String,
    pub files: Vec<ManifestFile>,
}

/// One verified file entry.
#[derive(Debug, Clone, Deserialize)]
pub struct ManifestFile {
    pub path: String,
    pub size: u64,
    pub sha256: String,
}

/// sha256 of a single file, read in 1 MiB chunks to keep memory flat even
/// for the 117 MB homosolver model.
///
/// Deliberate duplicate of the async `sha256_file` in the app's downloader
/// (`src-tauri/src/tts/silero_native/download.rs`): this is the sync engine
/// side, and sharing one helper would drag a runtime dependency across the
/// crate boundary. Keep both.
fn sha256_file(path: &Path) -> Result<String> {
    let file = File::open(path)
        .map_err(|e| EngineError::Bundle(format!("cannot open {}: {e}", path.display())))?;
    let mut reader = BufReader::with_capacity(1 << 20, file);
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = reader
            .read(&mut buf)
            .map_err(|e| EngineError::Bundle(format!("cannot read {}: {e}", path.display())))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

impl Manifest {
    /// Load and parse `manifest.json` from the bundle directory.
    pub fn load(bundle_dir: &Path) -> Result<Self> {
        let path = bundle_dir.join("manifest.json");
        let bytes = std::fs::read(&path)
            .map_err(|e| EngineError::Bundle(format!("cannot read {}: {e}", path.display())))?;
        Self::parse(&bytes)
    }

    /// Parse manifest bytes (already fetched from disk or the network)
    /// without touching the file system. Rejects malformed JSON and empty
    /// file lists.
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let manifest: Self = serde_json::from_slice(bytes)
            .map_err(|e| EngineError::Bundle(format!("malformed manifest.json: {e}")))?;
        if manifest.files.is_empty() {
            return Err(EngineError::Bundle(
                "manifest.json lists no files".to_string(),
            ));
        }
        Ok(manifest)
    }

    /// Verify every listed file against size and sha256. Returns the first
    /// mismatch (in manifest order) as a typed `Bundle` error naming the
    /// file.
    ///
    /// Files are hashed concurrently: sha256 of ~230 MB of models is pure
    /// CPU once the page cache is warm, and sequentially it costs ~115 ms
    /// of engine load time.
    #[instrument(skip(self), fields(files = self.files.len()))]
    pub fn verify(&self, bundle_dir: &Path) -> Result<()> {
        std::thread::scope(|scope| {
            let handles: Vec<_> = self
                .files
                .iter()
                .map(|entry| scope.spawn(|| self.verify_entry(bundle_dir, entry)))
                .collect();
            handles
                .into_iter()
                .map(|h| {
                    h.join().map_err(|payload| {
                        EngineError::Bundle(format!(
                            "verify thread panicked: {}",
                            panic_message(&payload)
                        ))
                    })?
                })
                .collect::<Result<Vec<()>>>()
        })?;
        Ok(())
    }

    /// Size + sha256 check of one manifest entry.
    fn verify_entry(&self, bundle_dir: &Path, entry: &ManifestFile) -> Result<()> {
        let path = bundle_dir.join(&entry.path);
        let meta = std::fs::metadata(&path)
            .map_err(|e| EngineError::Bundle(format!("cannot stat {}: {e}", path.display())))?;
        if meta.len() != entry.size {
            return Err(EngineError::Bundle(format!(
                "size mismatch for {}: expected {}, got {}",
                entry.path,
                entry.size,
                meta.len()
            )));
        }
        let actual = sha256_file(&path)?;
        if !actual.eq_ignore_ascii_case(&entry.sha256) {
            return Err(EngineError::Bundle(format!(
                "sha256 mismatch for {}",
                entry.path
            )));
        }
        debug!(file = %entry.path, "bundle file verified");
        Ok(())
    }

    /// Absolute path of a bundle file, guarding against manifest entries that
    /// would escape the bundle directory.
    pub fn file_path(&self, bundle_dir: &Path, name: &str) -> Result<PathBuf> {
        if name.contains("..") || name.starts_with('/') {
            return Err(EngineError::Bundle(format!(
                "unsafe path in manifest: {name}"
            )));
        }
        Ok(bundle_dir.join(name))
    }
}

/// Open ONNX sessions for the always-needed models of the bundle.
///
/// The rate-specific PQMF filters are NOT opened here: they are tiny, but
/// each is only needed when synthesis at that sample rate is actually
/// requested, so the engine lazy-opens them on first use (see
/// `Engine::synthesize`).
pub struct Sessions {
    pub tts_main: Session,
    pub istft: Session,
    pub homosolver: Session,
    pub accentor_tensor: Session,
}

/// Open one ONNX session.
///
/// Measured (issue #165, `tmp/bundle-v5`, Ryzen 9 7900): graph optimization
/// level makes no difference to session-creation time — Level3 ≈ Level1 ≈
/// Disable at ~310 ms for all sessions — because model parse/arena init
/// dominates, not the optimizers. That also rules out the `.ort`
/// compiled-model cache (it only skips optimization). Keep the defaults.
///
/// Intra-op thread count is pinned to 8 (issue #164). ORT's default (one
/// thread per logical core) is catastrophically slow for these models:
/// the graphs are chains of many small ops, so per-op fork/join sync across
/// 24 threads costs more than the compute. Bench mean at 24k, full pipeline:
/// default ≈ 104 ms, 4 threads 40 ms, 6 threads 34 ms, **8 threads 35 ms**,
/// 12 threads 55 ms, 24 threads 115 ms; parallel execution mode and inter-op
/// threads made no positive difference.
///
/// Why 8 and not 4–6: changing the thread count changes float reduction
/// order, which drifts the waveform off the Python-ONNX parity fixtures
/// (generated at ORT defaults). Measured worst case across the 31-case
/// suite: 1.5e-3 at 4 threads, 2.2e-3 at 6, **9.8e-4 at 8** — 8 is the
/// only reduced count that keeps every case inside the 1e-3 budget, and
/// it ties 6 for speed within noise.
pub(crate) fn open_session(path: &Path) -> Result<Session> {
    let mut builder = Session::builder()
        .map_err(|e| EngineError::Bundle(format!("session builder: {}", e.message())))?
        .with_intra_threads(8)
        .map_err(|e| EngineError::Bundle(format!("with_intra_threads(8): {}", e.message())))?;
    builder
        .commit_from_file(path)
        .map_err(|e| EngineError::Bundle(format!("failed to open {}: {e}", path.display())))
}

impl Sessions {
    /// Open the always-needed model sessions. Call only after
    /// [`Manifest::verify`].
    ///
    /// The sessions are independent, so they are opened concurrently —
    /// sequentially the total is the *sum* of per-session times (~500 ms,
    /// dominated by tts_main and homosolver at ~220 ms each), concurrently
    /// it approaches the *max*. ONNX Runtime session creation is thread-safe
    /// across independent sessions, and `Session` is `Send`.
    #[instrument(skip_all)]
    pub fn open(bundle_dir: &Path, manifest: &Manifest) -> Result<Self> {
        let total = Instant::now();
        let open = |name: &str| -> Result<Session> {
            let path = manifest.file_path(bundle_dir, name)?;
            let t = Instant::now();
            let session = open_session(&path)?;
            info!(
                model = name,
                elapsed_ms = t.elapsed().as_secs_f64() * 1e3,
                "ONNX session opened"
            );
            Ok(session)
        };
        const NAMES: [&str; 4] = [TTS_MAIN, ISTFT, HOMOSOLVER, ACCENTOR_TENSOR];
        // Each thread tags its session with the model name so the struct is
        // assembled by name, not by spawn position — a reordered NAMES can
        // never silently swap sessions.
        let results = std::thread::scope(|scope| {
            let handles: Vec<_> = NAMES
                .iter()
                .map(|name| scope.spawn(move || open(name).map(|s| (*name, s))))
                .collect();
            handles
                .into_iter()
                .map(|h| {
                    h.join().map_err(|payload| {
                        EngineError::Bundle(format!(
                            "session open thread panicked: {}",
                            panic_message(&payload)
                        ))
                    })?
                })
                .collect::<Result<Vec<_>>>()
        })?;
        let mut results = results;
        let mut take = |name: &str| {
            let pos = results
                .iter()
                .position(|(n, _)| *n == name)
                .expect("one result per session");
            results.swap_remove(pos).1
        };
        let sessions = Self {
            tts_main: take(TTS_MAIN),
            istft: take(ISTFT),
            homosolver: take(HOMOSOLVER),
            accentor_tensor: take(ACCENTOR_TENSOR),
        };
        info!(
            elapsed_ms = total.elapsed().as_secs_f64() * 1e3,
            "ONNX sessions initialized"
        );
        Ok(sessions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_manifest(dir: &Path, files: &[(&str, &str, u64)]) -> PathBuf {
        let entries: Vec<serde_json::Value> = files
            .iter()
            .map(|(path, sha, size)| serde_json::json!({"path": path, "size": size, "sha256": sha}))
            .collect();
        let manifest = serde_json::json!({
            "model_id": "test",
            "opset": 17,
            "export_date_utc": "2026-01-01T00:00:00+00:00",
            "files": entries,
        });
        let path = dir.join("manifest.json");
        let mut f = File::create(&path).expect("create manifest");
        f.write_all(manifest.to_string().as_bytes())
            .expect("write manifest");
        path
    }

    #[test]
    fn verify_accepts_matching_file() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data = b"hello bundle";
        std::fs::write(tmp.path().join("a.bin"), data).expect("write file");
        let sha = format!("{:x}", Sha256::new().chain_update(data).finalize());
        write_manifest(tmp.path(), &[("a.bin", &sha, data.len() as u64)]);
        let manifest = Manifest::load(tmp.path()).expect("load manifest");
        manifest.verify(tmp.path()).expect("verify ok");
    }

    #[test]
    fn verify_accepts_multiple_matching_files() {
        // Multi-entry happy path exercises the concurrent hash fan-out.
        let tmp = tempfile::tempdir().expect("tempdir");
        let data_a = b"hello bundle";
        let data_b = b"other file contents";
        std::fs::write(tmp.path().join("a.bin"), data_a).expect("write file");
        std::fs::write(tmp.path().join("b.bin"), data_b).expect("write file");
        let sha_a = format!("{:x}", Sha256::new().chain_update(data_a).finalize());
        let sha_b = format!("{:x}", Sha256::new().chain_update(data_b).finalize());
        write_manifest(
            tmp.path(),
            &[
                ("a.bin", &sha_a, data_a.len() as u64),
                ("b.bin", &sha_b, data_b.len() as u64),
            ],
        );
        let manifest = Manifest::load(tmp.path()).expect("load manifest");
        manifest.verify(tmp.path()).expect("verify ok");
    }

    #[test]
    fn verify_reports_first_mismatch_in_manifest_order() {
        // Two corrupted entries: the error must name the first one in
        // manifest order (the documented contract of `verify`).
        let tmp = tempfile::tempdir().expect("tempdir");
        let data = b"hello bundle";
        std::fs::write(tmp.path().join("a.bin"), data).expect("write file");
        std::fs::write(tmp.path().join("b.bin"), data).expect("write file");
        write_manifest(
            tmp.path(),
            &[
                ("a.bin", &"1".repeat(64), data.len() as u64),
                ("b.bin", &"2".repeat(64), data.len() as u64),
            ],
        );
        let manifest = Manifest::load(tmp.path()).expect("load manifest");
        let err = manifest.verify(tmp.path()).expect_err("must fail");
        let msg = err.to_string();
        assert!(msg.contains("a.bin"), "expected first entry: {msg}");
    }

    #[test]
    fn verify_rejects_corrupted_file() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data = b"hello bundle";
        std::fs::write(tmp.path().join("a.bin"), data).expect("write file");
        write_manifest(tmp.path(), &[("a.bin", &"0".repeat(64), data.len() as u64)]);
        let manifest = Manifest::load(tmp.path()).expect("load manifest");
        let err = manifest.verify(tmp.path()).expect_err("must fail");
        let msg = err.to_string();
        assert!(msg.contains("sha256 mismatch"), "unexpected error: {msg}");
        assert!(msg.contains("a.bin"), "error must name the file: {msg}");
    }

    #[test]
    fn verify_rejects_size_mismatch() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data = b"hello bundle";
        std::fs::write(tmp.path().join("a.bin"), data).expect("write file");
        let sha = format!("{:x}", Sha256::new().chain_update(data).finalize());
        write_manifest(tmp.path(), &[("a.bin", &sha, data.len() as u64 + 1)]);
        let manifest = Manifest::load(tmp.path()).expect("load manifest");
        let err = manifest.verify(tmp.path()).expect_err("must fail");
        assert!(err.to_string().contains("size mismatch"));
    }

    #[test]
    fn verify_rejects_missing_file() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_manifest(tmp.path(), &[("missing.bin", &"0".repeat(64), 1)]);
        let manifest = Manifest::load(tmp.path()).expect("load manifest");
        let err = manifest.verify(tmp.path()).expect_err("must fail");
        assert!(matches!(err, EngineError::Bundle(_)));
    }

    #[test]
    fn parse_rejects_malformed_json() {
        let err = Manifest::parse(b"not json").expect_err("must fail");
        assert!(err.to_string().contains("malformed manifest.json"));
    }

    #[test]
    fn parse_rejects_empty_file_list() {
        let bytes = serde_json::json!({
            "model_id": "test",
            "opset": 17,
            "export_date_utc": "2026-01-01T00:00:00+00:00",
            "files": [],
        })
        .to_string();
        let err = Manifest::parse(bytes.as_bytes()).expect_err("must fail");
        assert!(err.to_string().contains("lists no files"));
    }

    #[test]
    fn parse_accepts_valid_manifest() {
        let bytes = serde_json::json!({
            "model_id": "test",
            "opset": 17,
            "export_date_utc": "2026-01-01T00:00:00+00:00",
            "files": [{"path": "a.onnx", "size": 3, "sha256": "abc"}],
        })
        .to_string();
        let manifest = Manifest::parse(bytes.as_bytes()).expect("must parse");
        assert_eq!(manifest.files.len(), 1);
        assert_eq!(manifest.files[0].path, "a.onnx");
    }
}
