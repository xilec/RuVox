//! Lightweight probe for which TTS engines can be selected on the running
//! system. Phase 3 of #42.
//!
//! Piper is in-process and always available — its model files may be
//! missing, but engine itself loads, and a missing voice surfaces from
//! synthesis as `voice_not_installed` (the app then offers to download it).
//! Silero requires the `ttsd/` Python package, the `uv` toolchain to drive
//! its venv, and (transitively) the torch + Silero model. The probe is
//! cheap on purpose: it only checks the directory + `uv --version` so app
//! startup does not pay for a torch import. Failure to actually load the
//! model still surfaces from `model_error` later.
//! Silero Native needs the model bundle in the app data dir. Its probe is
//! stat-only — manifest parses and every listed file exists with the
//! recorded size — so `get_available_engines` stays cheap on every Settings
//! open (no sha256 hashing; the full verification runs inside the
//! downloader and the engine's bundle loader).

use std::path::Path;
use std::process::Command;

use serde::Serialize;

/// Machine-readable, localizable reason text (same shape subset as
/// `CommandError`: the frontend translates `code` via its catalogs and falls
/// back to the raw `message` for unknown codes).
#[derive(Debug, Clone, Serialize)]
pub struct LocalizedText {
    pub code: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub params: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl LocalizedText {
    fn new(code: &str) -> Self {
        Self {
            code: code.to_string(),
            params: Vec::new(),
            message: None,
        }
    }

    fn with_param(mut self, param: impl Into<String>) -> Self {
        self.params.push(param.into());
        self
    }

    fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }
}

/// Per-engine availability: whether the user can pick the engine in the
/// Settings selector, and a machine-readable reason to surface when not.
#[derive(Debug, Clone, Serialize)]
pub struct EngineAvailability {
    pub available: bool,
    /// `Some` only when `available == false`.
    pub reason: Option<LocalizedText>,
}

/// Output of [`probe`]. Field names match the `AvailabilityMap` shape on
/// the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct AvailableEngines {
    pub piper: EngineAvailability,
    pub silero: EngineAvailability,
    pub silero_native: EngineAvailability,
}

/// Probe the running environment for engine availability. `ttsd_dir` is
/// the path resolved by `lib.rs::resolve_ttsd_dir` — the location at which
/// the `ttsd/` Python package would live if shipped.
/// `silero_native_bundle_dir` is the app-data directory the model bundle is
/// downloaded into by `download_silero_native_bundle`.
pub fn probe(ttsd_dir: &Path, silero_native_bundle_dir: &Path) -> AvailableEngines {
    AvailableEngines {
        piper: EngineAvailability {
            available: true,
            reason: None,
        },
        silero: probe_silero(ttsd_dir),
        silero_native: probe_silero_native(silero_native_bundle_dir),
    }
}

pub fn probe_silero(ttsd_dir: &Path) -> EngineAvailability {
    let pyproject = ttsd_dir.join("pyproject.toml");
    if !pyproject.exists() {
        return EngineAvailability {
            available: false,
            reason: Some(
                LocalizedText::new("silero.ttsd_missing")
                    .with_param(ttsd_dir.display().to_string()),
            ),
        };
    }
    match check_uv() {
        Ok(()) => EngineAvailability {
            available: true,
            reason: None,
        },
        Err(reason) => EngineAvailability {
            available: false,
            reason: Some(reason),
        },
    }
}

fn check_uv() -> Result<(), LocalizedText> {
    check_uv_binary("uv")
}

/// Split from `check_uv` so tests can probe a guaranteed-nonexistent binary
/// without depending on the host actually lacking `uv`.
fn check_uv_binary(binary: &str) -> Result<(), LocalizedText> {
    let out = Command::new(binary).arg("--version").output();
    match out {
        Ok(o) if o.status.success() => Ok(()),
        Ok(o) => Err(LocalizedText::new("silero.uv_check_failed").with_param(o.status.to_string())),
        // Covers spawn failure (binary missing) — the normal case on
        // Windows, where ttsd is not shipped: the engine must report
        // unavailable, not error the command.
        Err(_) => Err(LocalizedText::new("silero.uv_missing")),
    }
}

/// Stat-only bundle probe: the manifest parses and every file it lists is
/// present with the recorded size. Deliberately no sha256 here — hashing
/// ~230 MB on every Settings open would stall the dialog; integrity is
/// enforced where it matters (downloader checksums, the engine's bundle
/// loader).
pub fn probe_silero_native(bundle_dir: &Path) -> EngineAvailability {
    let unavailable = |reason: LocalizedText| EngineAvailability {
        available: false,
        reason: Some(reason),
    };

    if !bundle_dir.join("manifest.json").exists() {
        return unavailable(LocalizedText::new("native.bundle_missing"));
    }
    let manifest = match silero_native::bundle::Manifest::load(bundle_dir) {
        Ok(m) => m,
        Err(e) => {
            return unavailable(
                LocalizedText::new("native.bundle_manifest_corrupt").with_message(e.to_string()),
            );
        }
    };
    for entry in &manifest.files {
        let path = bundle_dir.join(&entry.path);
        match std::fs::metadata(&path) {
            Ok(meta) if meta.len() == entry.size => (),
            _ => {
                return unavailable(
                    LocalizedText::new("native.bundle_incomplete").with_param(entry.path.clone()),
                );
            }
        }
    }
    EngineAvailability {
        available: true,
        reason: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::write_fake_bundle;

    #[test]
    fn piper_is_always_available() {
        let dir = tempfile::TempDir::new().unwrap();
        let res = probe(dir.path(), dir.path());
        assert!(res.piper.available);
        assert!(res.piper.reason.is_none());
    }

    #[test]
    fn silero_unavailable_when_ttsd_dir_missing_pyproject() {
        let dir = tempfile::TempDir::new().unwrap();
        let probe_result = probe(dir.path(), dir.path());
        assert!(!probe_result.silero.available);
        let reason = probe_result.silero.reason.expect("reason set");
        assert_eq!(reason.code, "silero.ttsd_missing");
        assert_eq!(reason.params.len(), 1);
        assert_eq!(reason.params[0], dir.path().display().to_string());
    }

    #[test]
    fn silero_unavailable_when_uv_cannot_be_spawned() {
        // ttsd is not shipped on Windows, so `uv` will be missing there —
        // the probe must degrade to unavailable (with a coded reason),
        // not propagate an error.
        let res = check_uv_binary("/nonexistent/uv/binary/that/should/never/exist");
        let reason = res.expect_err("spawn failure must be an Err mapped to unavailable");
        assert_eq!(reason.code, "silero.uv_missing");
    }

    #[test]
    fn silero_native_unavailable_without_bundle() {
        let dir = tempfile::TempDir::new().unwrap();
        let res = probe(dir.path(), dir.path());
        assert!(!res.silero_native.available);
        let reason = res.silero_native.reason.expect("reason set");
        assert_eq!(reason.code, "native.bundle_missing");
    }

    #[test]
    fn silero_native_available_with_complete_bundle() {
        let dir = tempfile::TempDir::new().unwrap();
        write_fake_bundle(dir.path(), &[("a.onnx", b"aaa"), ("b.onnx", b"bbbb")]);
        let res = probe_silero_native(dir.path());
        assert!(res.available);
        assert!(res.reason.is_none());
    }

    #[test]
    fn silero_native_unavailable_when_a_file_is_missing() {
        let dir = tempfile::TempDir::new().unwrap();
        write_fake_bundle(dir.path(), &[("a.onnx", b"aaa"), ("b.onnx", b"bbbb")]);
        std::fs::remove_file(dir.path().join("b.onnx")).unwrap();
        let res = probe_silero_native(dir.path());
        assert!(!res.available);
        let reason = res.reason.expect("reason set");
        assert_eq!(reason.code, "native.bundle_incomplete");
        assert_eq!(reason.params, vec!["b.onnx".to_string()]);
    }

    #[test]
    fn silero_native_unavailable_when_file_size_differs() {
        let dir = tempfile::TempDir::new().unwrap();
        write_fake_bundle(dir.path(), &[("a.onnx", b"aaa")]);
        std::fs::write(dir.path().join("a.onnx"), b"trailing garbage appended").unwrap();
        let res = probe_silero_native(dir.path());
        assert!(!res.available);
    }

    #[test]
    fn silero_native_unavailable_when_manifest_malformed() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("manifest.json"), b"not json").unwrap();
        let res = probe_silero_native(dir.path());
        assert!(!res.available);
        let reason = res.reason.expect("reason set");
        assert_eq!(reason.code, "native.bundle_manifest_corrupt");
        assert!(reason.message.is_some());
    }
}
