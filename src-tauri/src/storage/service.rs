use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::Local;
use parking_lot::RwLock;
use thiserror::Error;
use uuid::Uuid;

use crate::storage::schema::{
    EntryId, EntryStatus, HistoryFile, TextEntry, Timestamps, UIConfig, WordTimestamp,
};

const HISTORY_VERSION: u32 = 1;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json parse error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("entry not found: {0}")]
    NotFound(EntryId),
    #[error("cache dir unavailable: dirs::cache_dir() returned None")]
    NoCacheDir,
}

pub type Result<T> = std::result::Result<T, StorageError>;

pub struct StorageService {
    cache_dir: PathBuf,
    audio_dir: PathBuf,
    history_path: PathBuf,
    config_path: PathBuf,
    entries: Arc<RwLock<HashMap<EntryId, TextEntry>>>,
}

impl StorageService {
    /// Construct using the default cache dir (`~/.cache/ruvox/`).
    pub fn new() -> Result<Self> {
        let cache_dir = dirs::cache_dir()
            .ok_or(StorageError::NoCacheDir)?
            .join("ruvox");
        Self::with_cache_dir(cache_dir)
    }

    /// Construct with a custom cache dir (used in tests).
    pub fn with_cache_dir(cache_dir: PathBuf) -> Result<Self> {
        let audio_dir = cache_dir.join("audio");
        let history_path = cache_dir.join("history.json");
        let config_path = cache_dir.join("config.json");

        fs::create_dir_all(&audio_dir)?;

        let entries = Arc::new(RwLock::new(HashMap::new()));

        let service = Self {
            cache_dir,
            audio_dir,
            history_path,
            config_path,
            entries,
        };

        service.load_history()?;
        Ok(service)
    }

    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    // ── History persistence ────────────────────────────────────────────────

    fn load_history(&self) -> Result<()> {
        if !self.history_path.exists() {
            return Ok(());
        }

        let raw = match fs::read_to_string(&self.history_path) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("failed to read history.json: {e}");
                return Ok(());
            }
        };

        let history_file: HistoryFile = match serde_json::from_str(&raw) {
            Ok(h) => h,
            Err(e) => {
                tracing::warn!("history.json is corrupted ({e}), backing up and starting fresh");
                let bak = self.history_path.with_extension("json.bak");
                if let Err(re) = fs::rename(&self.history_path, &bak) {
                    tracing::error!("failed to back up corrupted history.json: {re}");
                }
                return Ok(());
            }
        };

        // Stub: just check version; real migration lives here when HISTORY_VERSION increments.
        if history_file.version > HISTORY_VERSION {
            tracing::warn!(
                "history.json version {} is newer than supported {}; loading anyway",
                history_file.version,
                HISTORY_VERSION
            );
        }

        let mut map = self.entries.write();
        let mut needs_save = false;

        for mut entry in history_file.entries {
            if self.validate_entry_status(&mut entry) {
                needs_save = true;
            }
            map.insert(entry.id, entry);
        }
        drop(map);

        if needs_save {
            self.save_history()?;
        }

        Ok(())
    }

    /// Returns true when the entry was modified (status fixed).
    fn validate_entry_status(&self, entry: &mut TextEntry) -> bool {
        if let Some(ref audio_filename) = entry.audio_path.clone() {
            let audio_path = self.audio_dir.join(audio_filename);
            if audio_path.exists() {
                if entry.status != EntryStatus::Ready {
                    entry.status = EntryStatus::Ready;
                    return true;
                }
            } else {
                // File is gone — reset audio metadata.
                entry.audio_path = None;
                entry.timestamps_path = None;
                entry.duration_sec = None;
                entry.audio_generated_at = None;
                if entry.status == EntryStatus::Ready {
                    entry.status = EntryStatus::Pending;
                    return true;
                }
            }
        } else {
            match entry.status {
                // Was synthesising when the process died — reset.
                EntryStatus::Processing => {
                    entry.status = EntryStatus::Pending;
                    return true;
                }
                // Ready but no audio_path — inconsistent state, reset.
                EntryStatus::Ready => {
                    entry.status = EntryStatus::Pending;
                    return true;
                }
                _ => {}
            }
        }
        false
    }

    fn save_history(&self) -> Result<()> {
        let map = self.entries.read();

        // Normalise runtime-only Playing status to Ready before persisting.
        let entries: Vec<TextEntry> = map
            .values()
            .map(|e| {
                let mut e = e.clone();
                if e.status == EntryStatus::Playing {
                    e.status = EntryStatus::Ready;
                }
                e
            })
            .collect();

        let history_file = HistoryFile {
            version: HISTORY_VERSION,
            entries,
        };

        let json = serde_json::to_string_pretty(&history_file)?;
        write_atomic(&self.history_path, json.as_bytes())?;
        Ok(())
    }

    // ── CRUD ───────────────────────────────────────────────────────────────

    /// Add a new entry with an auto-generated UUID. Persists history.
    pub fn add_entry(&self, original_text: String) -> Result<TextEntry> {
        // Strip the UTF-8 BOM if present (matches what the prior Qt-based build did,
        // keeping cached entries identical between the two implementations).
        let clean_text = original_text
            .strip_prefix('\u{feff}')
            .unwrap_or(&original_text)
            .to_string();

        let entry = TextEntry {
            id: Uuid::new_v4(),
            original_text: clean_text,
            normalized_text: None,
            status: EntryStatus::Pending,
            created_at: Local::now().naive_local(),
            audio_path: None,
            timestamps_path: None,
            duration_sec: None,
            audio_generated_at: None,
            was_regenerated: false,
            error_message: None,
        };

        self.entries.write().insert(entry.id, entry.clone());
        self.save_history()?;
        Ok(entry)
    }

    pub fn get_entry(&self, id: &EntryId) -> Option<TextEntry> {
        self.entries.read().get(id).cloned()
    }

    /// Replace an existing entry. Saves history.
    pub fn update_entry(&self, entry: TextEntry) -> Result<()> {
        self.entries.write().insert(entry.id, entry);
        self.save_history()
    }

    /// Delete entry and its audio + timestamps files.
    pub fn delete_entry(&self, id: &EntryId) -> Result<()> {
        let entry = {
            let map = self.entries.read();
            map.get(id).cloned()
        };

        if let Some(entry) = entry {
            if let Some(ref filename) = entry.audio_path {
                remove_file_if_exists(&self.audio_dir.join(filename));
            }
            if let Some(ref filename) = entry.timestamps_path {
                remove_file_if_exists(&self.audio_dir.join(filename));
            }
            self.entries.write().remove(id);
            self.save_history()?;
        }
        Ok(())
    }

    /// Delete only the audio + timestamps files; keeps the text entry as Pending.
    pub fn delete_audio(&self, id: &EntryId) -> Result<()> {
        let mut map = self.entries.write();
        let entry = map.get_mut(id).ok_or(StorageError::NotFound(*id))?;

        if let Some(ref filename) = entry.audio_path.take() {
            remove_file_if_exists(&self.audio_dir.join(filename));
        }
        if let Some(ref filename) = entry.timestamps_path.take() {
            remove_file_if_exists(&self.audio_dir.join(filename));
        }
        entry.status = EntryStatus::Pending;
        entry.audio_generated_at = None;
        entry.duration_sec = None;
        drop(map);

        self.save_history()
    }

    /// All entries sorted by `created_at`, newest first.
    pub fn get_all_entries(&self) -> Vec<TextEntry> {
        let mut entries: Vec<TextEntry> = self.entries.read().values().cloned().collect();
        entries.sort_by_key(|e| std::cmp::Reverse(e.created_at));
        entries
    }

    // ── Audio / Timestamps ─────────────────────────────────────────────────

    /// Write raw WAV bytes. Returns the relative filename (for `TextEntry.audio_path`).
    pub fn save_audio(&self, id: &EntryId, audio_bytes: &[u8]) -> Result<String> {
        let filename = format!("{id}.wav");
        let path = self.audio_dir.join(&filename);
        fs::write(path, audio_bytes)?;
        Ok(filename)
    }

    /// Write word timestamps. Returns the relative filename (for `TextEntry.timestamps_path`).
    pub fn save_timestamps(&self, id: &EntryId, timestamps: &[WordTimestamp]) -> Result<String> {
        let filename = format!("{id}.timestamps.json");
        let path = self.audio_dir.join(&filename);
        let wrapper = Timestamps {
            words: timestamps.to_vec(),
        };
        let json = serde_json::to_string_pretty(&wrapper)?;
        write_atomic(&path, json.as_bytes())?;
        Ok(filename)
    }

    pub fn load_timestamps(&self, id: &EntryId) -> Result<Option<Vec<WordTimestamp>>> {
        let timestamps_path = {
            let map = self.entries.read();
            match map.get(id) {
                None => return Ok(None),
                Some(e) => e.timestamps_path.clone(),
            }
        };

        let Some(filename) = timestamps_path else {
            return Ok(None);
        };

        let path = self.audio_dir.join(&filename);
        if !path.exists() {
            return Ok(None);
        }

        let raw = fs::read_to_string(&path)?;
        let wrapper: Timestamps = serde_json::from_str(&raw)?;
        Ok(Some(wrapper.words))
    }

    /// Resolve the full path to the audio WAV file, if it exists on disk.
    pub fn get_audio_path(&self, id: &EntryId) -> Option<PathBuf> {
        let map = self.entries.read();
        let filename = map.get(id)?.audio_path.as_ref()?.clone();
        let full = self.audio_dir.join(filename);
        if full.exists() {
            Some(full)
        } else {
            None
        }
    }

    // ── Stats ──────────────────────────────────────────────────────────────

    /// Total size of all files in the audio directory, in bytes.
    pub fn get_cache_size(&self) -> Result<u64> {
        let mut total: u64 = 0;
        for entry in fs::read_dir(&self.audio_dir)? {
            let entry = entry?;
            let meta = entry.metadata()?;
            if meta.is_file() {
                total += meta.len();
            }
        }
        Ok(total)
    }

    /// Number of `.wav` files in the audio directory.
    pub fn get_audio_count(&self) -> Result<u32> {
        let mut count: u32 = 0;
        for entry in fs::read_dir(&self.audio_dir)? {
            let entry = entry?;
            if entry.path().extension().and_then(|e| e.to_str()) == Some("wav") {
                count += 1;
            }
        }
        Ok(count)
    }

    // ── Config ─────────────────────────────────────────────────────────────

    pub fn load_config(&self) -> Result<UIConfig> {
        if !self.config_path.exists() {
            return Ok(UIConfig::default());
        }
        let raw = fs::read_to_string(&self.config_path)?;
        let config: UIConfig = serde_json::from_str(&raw)?;
        Ok(config)
    }

    pub fn save_config(&self, config: &UIConfig) -> Result<()> {
        let json = serde_json::to_string_pretty(config)?;
        write_atomic(&self.config_path, json.as_bytes())?;
        Ok(())
    }
}

// ── Helpers ────────────────────────────────────────────────────────────────

/// Write to `<path>.tmp` then atomically rename to `<path>`.
fn write_atomic(path: &Path, data: &[u8]) -> Result<()> {
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, data)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

fn remove_file_if_exists(path: &Path) {
    if path.exists() {
        if let Err(e) = fs::remove_file(path) {
            tracing::warn!("failed to remove file {}: {e}", path.display());
        }
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_service() -> (StorageService, TempDir) {
        let dir = TempDir::new().unwrap();
        let svc = StorageService::with_cache_dir(dir.path().to_path_buf()).unwrap();
        (svc, dir)
    }

    #[test]
    fn new_creates_dirs() {
        let dir = TempDir::new().unwrap();
        let cache = dir.path().join("ruvox_test");
        StorageService::with_cache_dir(cache.clone()).unwrap();
        assert!(cache.exists());
        assert!(cache.join("audio").exists());
    }

    #[test]
    fn add_and_get_entry_roundtrip() {
        let (svc, _dir) = make_service();
        let entry = svc.add_entry("Привет мир".to_string()).unwrap();
        let fetched = svc.get_entry(&entry.id).unwrap();
        assert_eq!(fetched.original_text, "Привет мир");
        assert_eq!(fetched.status, EntryStatus::Pending);
    }

    #[test]
    fn add_entry_strips_bom() {
        let (svc, _dir) = make_service();
        let text_with_bom = "\u{feff}hello".to_string();
        let entry = svc.add_entry(text_with_bom).unwrap();
        assert_eq!(entry.original_text, "hello");
    }

    #[test]
    fn update_entry_persists() {
        let (svc, dir) = make_service();
        let mut entry = svc.add_entry("original".to_string()).unwrap();
        entry.normalized_text = Some("normalized".to_string());
        entry.status = EntryStatus::Ready;
        svc.update_entry(entry.clone()).unwrap();

        // Reload from disk.
        let svc2 = StorageService::with_cache_dir(dir.path().to_path_buf()).unwrap();
        let loaded = svc2.get_entry(&entry.id).unwrap();
        assert_eq!(loaded.normalized_text.as_deref(), Some("normalized"));
    }

    #[test]
    fn delete_entry_removes_entry_and_files() {
        let (svc, _dir) = make_service();
        let entry = svc.add_entry("to delete".to_string()).unwrap();
        let id = entry.id;

        // Save a fake audio file.
        let audio_filename = svc.save_audio(&id, b"RIFF fake wav").unwrap();
        let audio_path = svc.cache_dir().join("audio").join(&audio_filename);
        assert!(audio_path.exists());

        // Update entry with audio path.
        let mut updated = entry.clone();
        updated.audio_path = Some(audio_filename.clone());
        updated.status = EntryStatus::Ready;
        svc.update_entry(updated).unwrap();

        svc.delete_entry(&id).unwrap();

        assert!(svc.get_entry(&id).is_none());
        assert!(!audio_path.exists());
    }

    #[test]
    fn get_all_entries_newest_first() {
        let (svc, _dir) = make_service();
        let e1 = svc.add_entry("first".to_string()).unwrap();
        // Small sleep to ensure distinct timestamps.
        std::thread::sleep(std::time::Duration::from_millis(5));
        let e2 = svc.add_entry("second".to_string()).unwrap();

        let all = svc.get_all_entries();
        assert_eq!(all.len(), 2);
        // Newest first.
        assert_eq!(all[0].id, e2.id);
        assert_eq!(all[1].id, e1.id);
    }

    #[test]
    fn save_and_get_audio_path() {
        let (svc, _dir) = make_service();
        let entry = svc.add_entry("audio test".to_string()).unwrap();
        let id = entry.id;

        let filename = svc.save_audio(&id, b"RIFF fake").unwrap();
        assert_eq!(filename, format!("{id}.wav"));

        let mut updated = entry.clone();
        updated.audio_path = Some(filename);
        svc.update_entry(updated).unwrap();

        let path = svc.get_audio_path(&id).unwrap();
        assert!(path.exists());
    }

    #[test]
    fn save_and_load_timestamps_roundtrip() {
        let (svc, _dir) = make_service();
        let entry = svc.add_entry("timestamps test".to_string()).unwrap();
        let id = entry.id;

        let words = vec![
            WordTimestamp {
                word: "привет".to_string(),
                start: 0.0,
                end: 0.4,
                original_pos: (0, 6),
            },
            WordTimestamp {
                word: "мир".to_string(),
                start: 0.5,
                end: 0.9,
                original_pos: (7, 10),
            },
        ];

        let filename = svc.save_timestamps(&id, &words).unwrap();
        assert_eq!(filename, format!("{id}.timestamps.json"));

        let mut updated = entry.clone();
        updated.timestamps_path = Some(filename);
        svc.update_entry(updated).unwrap();

        let loaded = svc.load_timestamps(&id).unwrap().unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].word, "привет");
        assert_eq!(loaded[1].original_pos, (7, 10));
    }

    #[test]
    fn load_history_json_from_disk() {
        let dir = TempDir::new().unwrap();
        let cache = dir.path().to_path_buf();
        fs::create_dir_all(cache.join("audio")).unwrap();

        // Sample history.json in the on-disk format.
        let on_disk_json = r#"{
            "version": 1,
            "entries": [
                {
                    "id": "550e8400-e29b-41d4-a716-446655440000",
                    "original_text": "Вызови getUserData() через API",
                    "normalized_text": "Вызови гет юзер дата через эй пи ай",
                    "status": "ready",
                    "created_at": "2025-01-15T10:30:00.123456",
                    "audio_path": null,
                    "timestamps_path": null,
                    "duration_sec": null,
                    "audio_generated_at": null,
                    "was_regenerated": false,
                    "error_message": null
                }
            ]
        }"#;
        fs::write(cache.join("history.json"), on_disk_json).unwrap();

        let svc = StorageService::with_cache_dir(cache).unwrap();
        let all = svc.get_all_entries();
        assert_eq!(all.len(), 1);
        assert_eq!(
            all[0].normalized_text.as_deref(),
            Some("Вызови гет юзер дата через эй пи ай")
        );
        // status was "ready" but audio_path is null → validate_statuses resets to pending.
        assert_eq!(all[0].status, EntryStatus::Pending);
    }

    #[test]
    fn validate_statuses_resets_missing_audio() {
        let dir = TempDir::new().unwrap();
        let cache = dir.path().to_path_buf();
        fs::create_dir_all(cache.join("audio")).unwrap();

        let missing_filename = "00000000-0000-0000-0000-000000000001.wav";
        let history_json = format!(
            r#"{{
                "version": 1,
                "entries": [
                    {{
                        "id": "00000000-0000-0000-0000-000000000001",
                        "original_text": "test",
                        "status": "ready",
                        "created_at": "2025-01-01T00:00:00.000000",
                        "audio_path": "{missing_filename}"
                    }}
                ]
            }}"#
        );
        fs::write(cache.join("history.json"), history_json).unwrap();

        // Audio file does NOT exist on disk.
        let svc = StorageService::with_cache_dir(cache).unwrap();
        let id: Uuid = "00000000-0000-0000-0000-000000000001".parse().unwrap();
        let entry = svc.get_entry(&id).unwrap();
        assert_eq!(entry.status, EntryStatus::Pending);
        assert!(entry.audio_path.is_none());
    }

    #[test]
    fn corrupted_history_json_starts_fresh_with_backup() {
        let dir = TempDir::new().unwrap();
        let cache = dir.path().to_path_buf();
        fs::create_dir_all(cache.join("audio")).unwrap();

        fs::write(cache.join("history.json"), b"this is not json!!!").unwrap();

        let svc = StorageService::with_cache_dir(cache.clone()).unwrap();
        // Should start with an empty store.
        assert_eq!(svc.get_all_entries().len(), 0);
        // Backup must exist.
        assert!(cache.join("history.json.bak").exists());
    }

    #[test]
    fn save_and_load_config() {
        let (svc, _dir) = make_service();

        let cfg = UIConfig {
            speaker: "aidar".to_string(),
            sample_rate: 24000,
            ..UIConfig::default()
        };
        svc.save_config(&cfg).unwrap();

        let loaded = svc.load_config().unwrap();
        assert_eq!(loaded.speaker, "aidar");
        assert_eq!(loaded.sample_rate, 24000);
    }

    #[test]
    fn load_config_returns_default_when_missing() {
        let (svc, _dir) = make_service();
        let cfg = svc.load_config().unwrap();
        assert_eq!(cfg.speaker, "xenia");
        assert_eq!(cfg.sample_rate, 48000);
    }

    #[test]
    fn delete_audio_keeps_entry_as_pending() {
        let (svc, _dir) = make_service();
        let entry = svc.add_entry("delete audio test".to_string()).unwrap();
        let id = entry.id;

        let audio_filename = svc.save_audio(&id, b"RIFF data").unwrap();
        let ts_filename = svc
            .save_timestamps(
                &id,
                &[WordTimestamp {
                    word: "тест".to_string(),
                    start: 0.0,
                    end: 0.5,
                    original_pos: (0, 4),
                }],
            )
            .unwrap();

        let mut updated = entry.clone();
        updated.audio_path = Some(audio_filename.clone());
        updated.timestamps_path = Some(ts_filename.clone());
        updated.status = EntryStatus::Ready;
        svc.update_entry(updated).unwrap();

        svc.delete_audio(&id).unwrap();

        let remaining = svc.get_entry(&id).unwrap();
        assert_eq!(remaining.status, EntryStatus::Pending);
        assert!(remaining.audio_path.is_none());
        assert!(remaining.timestamps_path.is_none());

        // Files must be gone.
        let audio_path = svc.cache_dir().join("audio").join(&audio_filename);
        let ts_path = svc.cache_dir().join("audio").join(&ts_filename);
        assert!(!audio_path.exists());
        assert!(!ts_path.exists());
    }

    #[test]
    fn get_cache_size_and_audio_count() {
        let (svc, _dir) = make_service();
        let e1 = svc.add_entry("a".to_string()).unwrap();
        let e2 = svc.add_entry("b".to_string()).unwrap();

        svc.save_audio(&e1.id, b"RIFF AAAA").unwrap();
        svc.save_audio(&e2.id, b"RIFF BBBB").unwrap();

        let count = svc.get_audio_count().unwrap();
        assert_eq!(count, 2);

        let size = svc.get_cache_size().unwrap();
        // history.json and timestamps may also be in the audio dir — we only need size > 0.
        assert!(size > 0);
    }
}
