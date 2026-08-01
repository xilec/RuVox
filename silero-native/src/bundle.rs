//! Model bundle loading: manifest parsing, per-file sha256 verification and
//! ONNX session initialization.
//!
//! The bundle format is produced by `silero-native/export` and documented in
//! `export/README.md`. Every file listed in `manifest.json` is hashed before
//! any session is opened — a corrupt file must never reach ONNX Runtime.

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::time::Instant;

use ort::session::Session;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tracing::{debug, info, instrument};

use crate::error::{EngineError, Result};

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
                    h.join()
                        .map_err(|_| EngineError::Bundle("verify thread panicked".to_string()))?
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

/// Open one ONNX session with default settings.
///
/// Measured (issue #165, `tmp/bundle-v5`, Ryzen 9 7900): graph optimization
/// level makes no difference to session-creation time — Level3 ≈ Level1 ≈
/// Disable at ~310 ms for all sessions — because model parse/arena init
/// dominates, not the optimizers. That also rules out the `.ort`
/// compiled-model cache (it only skips optimization). Keep the defaults.
pub(crate) fn open_session(path: &Path) -> Result<Session> {
    Session::builder()
        .and_then(|mut b| b.commit_from_file(path))
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
        let results = std::thread::scope(|scope| {
            let handles: Vec<_> = NAMES
                .iter()
                .map(|name| scope.spawn(|| open(name)))
                .collect();
            handles
                .into_iter()
                .map(|h| {
                    h.join()
                        .map_err(|_| {
                            EngineError::Bundle("session open thread panicked".to_string())
                        })?
                        .map(Some)
                })
                .collect::<Result<Vec<_>>>()
        })?;
        let mut results = results.into_iter();
        let mut next = || results.next().flatten().expect("one result per session");
        let sessions = Self {
            // `results` is in NAMES order.
            tts_main: next(),
            istft: next(),
            homosolver: next(),
            accentor_tensor: next(),
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
