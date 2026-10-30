//! Bundle manifest: parsing, format-version gate, and per-file verification.
//!
//! The manifest is the root of trust for everything else: the loader verifies
//! every listed file (size, then sha256) before any ONNX session is opened, so
//! a truncated download or a partially copied bundle fails with a typed error
//! instead of a protobuf or ONNX Runtime crash. The downloader and the
//! availability probe reuse the same parser; the probe checks sizes only.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::{Error, Result};

/// Manifest schema version this build understands.
pub const FORMAT_VERSION: u32 = 1;
/// File name of the manifest inside a bundle directory.
pub const MANIFEST_NAME: &str = "manifest.json";

/// One file listed in the manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestFile {
    /// Bundle-relative path, e.g. `models/vocoder.onnx`.
    pub path: String,
    /// Flat GitHub release asset name, e.g. `models__vocoder.onnx`.
    pub asset: String,
    pub size: u64,
    /// Lowercase hex sha256.
    pub sha256: String,
}

/// One voice style directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Voice {
    pub id: String,
    /// BCP-47-ish language tag: `ru` or `en`.
    pub language: String,
}

/// Parsed `manifest.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub format_version: u32,
    pub bundle_version: u32,
    pub model_id: String,
    pub model_version: String,
    pub upstream_repo: String,
    pub upstream_revision: String,
    pub opset: u32,
    pub sample_rate: u32,
    pub created_at_utc: String,
    pub voices: Vec<Voice>,
    pub files: Vec<ManifestFile>,
}

impl Manifest {
    /// Read and parse `dir/manifest.json`, enforcing the format version.
    ///
    /// Does not touch the listed files — see [`Self::check_sizes`] and
    /// [`Bundle::verify`].
    pub fn load(dir: &Path) -> Result<Self> {
        let path = dir.join(MANIFEST_NAME);
        if !path.is_file() {
            return Err(Error::BundleNotInstalled(dir.to_path_buf()));
        }
        let bytes = fs::read(&path)
            .map_err(|e| Error::BundleCorrupt(format!("cannot read {}: {e}", path.display())))?;
        let manifest: Self = serde_json::from_slice(&bytes)
            .map_err(|e| Error::BundleCorrupt(format!("{}: {e}", path.display())))?;
        if manifest.format_version != FORMAT_VERSION {
            return Err(Error::BundleIncompatible {
                found: manifest.format_version,
                expected: FORMAT_VERSION,
            });
        }
        Ok(manifest)
    }

    /// Stat-only check used by the availability probe: every listed file
    /// exists and has the recorded size. No hashing (410 MiB on the UI path).
    pub fn check_sizes(&self, dir: &Path) -> Result<()> {
        for file in &self.files {
            let path = dir.join(&file.path);
            let meta = fs::metadata(&path)
                .map_err(|e| Error::BundleCorrupt(format!("missing {}: {e}", file.path)))?;
            if meta.len() != file.size {
                return Err(Error::BundleCorrupt(format!(
                    "{}: size {} != manifest {}",
                    file.path,
                    meta.len(),
                    file.size
                )));
            }
        }
        Ok(())
    }

    /// Voice ids for one language, in manifest order.
    pub fn voices_for(&self, language: &str) -> Vec<&Voice> {
        self.voices
            .iter()
            .filter(|voice| voice.language == language)
            .collect()
    }

    /// All voice ids, for error messages.
    pub fn voice_ids(&self) -> Vec<&str> {
        self.voices.iter().map(|voice| voice.id.as_str()).collect()
    }
}

/// A verified bundle directory plus its parsed manifest.
#[derive(Debug, Clone)]
pub struct Bundle {
    dir: PathBuf,
    manifest: Manifest,
    manifest_sha256: String,
}

impl Bundle {
    /// Parse the manifest and verify every listed file (size, then sha256).
    ///
    /// This is the expensive path and runs once per engine load, never on the
    /// UI thread and never in the availability probe.
    pub fn load(dir: &Path) -> Result<Self> {
        let manifest_path = dir.join(MANIFEST_NAME);
        let manifest = Manifest::load(dir)?;
        let manifest_sha256 = sha256_file(&manifest_path)?;
        let bundle = Self {
            dir: dir.to_path_buf(),
            manifest,
            manifest_sha256,
        };
        bundle.verify()?;
        Ok(bundle)
    }

    /// Verify every file's size and checksum.
    pub fn verify(&self) -> Result<()> {
        for file in &self.manifest.files {
            let path = self.dir.join(&file.path);
            let meta = fs::metadata(&path)
                .map_err(|e| Error::BundleCorrupt(format!("missing {}: {e}", file.path)))?;
            if meta.len() != file.size {
                return Err(Error::BundleCorrupt(format!(
                    "{}: size {} != manifest {}",
                    file.path,
                    meta.len(),
                    file.size
                )));
            }
            if file.sha256.len() != 64 || !file.sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err(Error::BundleCorrupt(format!(
                    "{}: malformed sha256 in manifest",
                    file.path
                )));
            }
            let actual = sha256_file(&path)?;
            if actual != file.sha256.to_ascii_lowercase() {
                return Err(Error::BundleCorrupt(format!(
                    "{}: sha256 {} != manifest {}",
                    file.path, actual, file.sha256
                )));
            }
        }
        Ok(())
    }

    /// Bundle root.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Parsed manifest.
    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    /// sha256 of `manifest.json` (model identity in the generation snapshot).
    pub fn manifest_sha256(&self) -> &str {
        &self.manifest_sha256
    }

    /// Absolute path of a bundle-relative file.
    pub fn file(&self, relative: &str) -> PathBuf {
        self.dir.join(relative)
    }

    /// Resolve a voice id to its style directory, rejecting path separators and
    /// ids the manifest does not list.
    pub fn voice_dir(&self, voice: &str) -> Result<PathBuf> {
        if !self.manifest.voices.iter().any(|v| v.id == voice) {
            return Err(Error::VoiceUnknown {
                voice: voice.to_string(),
                available: self.manifest.voice_ids().join(", "),
            });
        }
        if voice.is_empty()
            || !voice
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
        {
            return Err(Error::VoiceUnknown {
                voice: voice.to_string(),
                available: self.manifest.voice_ids().join(", "),
            });
        }
        Ok(self.dir.join("styles").join(voice))
    }
}

/// Streaming sha256 of a file, lowercase hex.
pub fn sha256_file(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)
        .map_err(|e| Error::BundleCorrupt(format!("cannot open {}: {e}", path.display())))?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 1 << 20];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|e| Error::BundleCorrupt(format!("cannot read {}: {e}", path.display())))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_bundle(dir: &Path, content: &[u8]) -> Manifest {
        fs::create_dir_all(dir.join("models")).unwrap();
        fs::write(dir.join("models/a.bin"), content).unwrap();
        let manifest = Manifest {
            format_version: FORMAT_VERSION,
            bundle_version: 1,
            model_id: "test".into(),
            model_version: "1".into(),
            upstream_repo: "test/repo".into(),
            upstream_revision: "abc".into(),
            opset: 19,
            sample_rate: 44_100,
            created_at_utc: "2026-01-01T00:00:00Z".into(),
            voices: vec![Voice {
                id: "ru_m5".into(),
                language: "ru".into(),
            }],
            files: vec![ManifestFile {
                path: "models/a.bin".into(),
                asset: "models__a.bin".into(),
                size: content.len() as u64,
                sha256: format!("{:x}", Sha256::digest(content)),
            }],
        };
        fs::write(
            dir.join(MANIFEST_NAME),
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();
        manifest
    }

    #[test]
    fn loads_a_valid_bundle() {
        let tmp = tempfile::tempdir().unwrap();
        write_bundle(tmp.path(), b"hello");
        let bundle = Bundle::load(tmp.path()).unwrap();
        assert_eq!(bundle.manifest().model_id, "test");
        assert_eq!(bundle.manifest().voices_for("ru")[0].id, "ru_m5");
    }

    #[test]
    fn rejects_a_missing_bundle() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(matches!(
            Bundle::load(tmp.path()),
            Err(Error::BundleNotInstalled(_))
        ));
    }

    #[test]
    fn rejects_an_unsupported_format() {
        let tmp = tempfile::tempdir().unwrap();
        let mut manifest = write_bundle(tmp.path(), b"hello");
        manifest.format_version = FORMAT_VERSION + 1;
        fs::write(
            tmp.path().join(MANIFEST_NAME),
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();
        assert!(matches!(
            Bundle::load(tmp.path()),
            Err(Error::BundleIncompatible { .. })
        ));
    }

    #[test]
    fn rejects_a_truncated_file_before_hashing() {
        let tmp = tempfile::tempdir().unwrap();
        let manifest = write_bundle(tmp.path(), b"hello");
        fs::write(tmp.path().join("models/a.bin"), b"hell").unwrap();
        let err = Bundle::load(tmp.path()).unwrap_err();
        assert!(err.to_string().contains("size 4 != manifest 5"), "{err}");
        assert_eq!(manifest.files[0].size, 5);
    }

    #[test]
    fn rejects_a_corrupted_file() {
        let tmp = tempfile::tempdir().unwrap();
        write_bundle(tmp.path(), b"hello");
        fs::write(tmp.path().join("models/a.bin"), b"world").unwrap();
        let err = Bundle::load(tmp.path()).unwrap_err();
        assert!(err.to_string().contains("sha256"), "{err}");
    }

    #[test]
    fn size_probe_does_not_hash() {
        let tmp = tempfile::tempdir().unwrap();
        write_bundle(tmp.path(), b"hello");
        fs::write(tmp.path().join("models/a.bin"), b"world").unwrap();
        let manifest = Manifest::load(tmp.path()).unwrap();
        // Same size, different content: the stat-only probe passes, the full
        // load fails.
        manifest.check_sizes(tmp.path()).unwrap();
        assert!(Bundle::load(tmp.path()).is_err());
    }

    #[test]
    fn rejects_path_traversal_in_voice_ids() {
        let tmp = tempfile::tempdir().unwrap();
        write_bundle(tmp.path(), b"hello");
        let bundle = Bundle::load(tmp.path()).unwrap();
        assert!(bundle.voice_dir("../../etc").is_err());
        assert!(bundle.voice_dir("ru_m9").is_err());
        assert!(bundle.voice_dir("ru_m5").is_ok());
    }
}
