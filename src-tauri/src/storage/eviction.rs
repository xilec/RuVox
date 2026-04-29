//! Cache hygiene: orphan sweep and size-based / wholesale eviction.
//!
//! Functions in this module operate on a [`StorageService`] borrow and rely on
//! its public API (and a few `pub(super)` fields) for filesystem and history
//! access. [`StorageService`] re-exports each of them as a method so callers
//! see one stable surface.

use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime};

use crate::storage::schema::{EntryId, EntryStatus, TextEntry};
use crate::storage::service::{Result, StorageService};

/// Files modified within this window are skipped by [`sweep_orphans`] and the
/// orphan branches of cache eviction. Protects in-flight synthesis output that
/// has been written to disk but not yet recorded in `history.json`.
const RECENT_FILE_GRACE: Duration = Duration::from_secs(60);

/// Outcome of [`sweep_orphans`].
#[derive(Debug, Default, Clone, Copy)]
pub struct SweepStats {
    /// Files in the audio directory that did not match any entry and were removed.
    pub deleted_files: u32,
    /// Total bytes reclaimed.
    pub freed_bytes: u64,
}

/// Outcome of [`evict_to_size`] / [`evict_all`].
#[derive(Debug, Default, Clone)]
pub struct EvictStats {
    /// Files removed (audio + timestamps).
    pub deleted_files: u32,
    /// Entries removed from `history.json` (only when `delete_texts = true`).
    pub deleted_entries: u32,
    /// Total bytes reclaimed.
    pub freed_bytes: u64,
    /// IDs of entries whose audio was reset to `null` (when `delete_texts = false`).
    pub updated_ids: Vec<EntryId>,
    /// IDs of entries removed from history (when `delete_texts = true`).
    pub removed_ids: Vec<EntryId>,
}

/// Outcome of [`run_startup_cleanup`].
#[derive(Debug, Default, Clone)]
pub struct StartupCleanupStats {
    pub sweep: SweepStats,
    pub evict: EvictStats,
}

/// Build the set of filenames in `audio/` that belong to a known entry
/// (either as `audio_path` or as `timestamps_path`).
fn build_valid_filename_set(svc: &StorageService) -> HashSet<String> {
    let map = svc.entries.read();
    let mut valid: HashSet<String> = HashSet::with_capacity(map.len() * 2);
    for entry in map.values() {
        if let Some(ref name) = entry.audio_path {
            valid.insert(name.clone());
        }
        if let Some(ref name) = entry.timestamps_path {
            valid.insert(name.clone());
        }
    }
    valid
}

/// Remove files in `audio/` that do not correspond to any entry in
/// `history.json`. Files modified within [`RECENT_FILE_GRACE`] are preserved
/// to avoid racing with active synthesis (the audio is on disk but the entry
/// hasn't been updated yet).
pub fn sweep_orphans(svc: &StorageService) -> Result<SweepStats> {
    let valid = build_valid_filename_set(svc);
    let mut stats = SweepStats::default();
    let now = SystemTime::now();

    for dir_entry in fs::read_dir(&svc.audio_dir)? {
        let dir_entry = dir_entry?;
        let path = dir_entry.path();
        let meta = match dir_entry.metadata() {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!("sweep_orphans: stat failed for {:?}: {e}", path);
                continue;
            }
        };
        if !meta.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if valid.contains(name) {
            continue;
        }
        if let Ok(modified) = meta.modified() {
            if now.duration_since(modified).unwrap_or(Duration::ZERO) < RECENT_FILE_GRACE {
                tracing::debug!("sweep_orphans: skipping recent file {name}");
                continue;
            }
        }

        let size = meta.len();
        match fs::remove_file(&path) {
            Ok(()) => {
                stats.deleted_files += 1;
                stats.freed_bytes += size;
                tracing::info!("sweep_orphans: removed orphan {name} ({size} bytes)");
            }
            Err(e) => {
                tracing::warn!("sweep_orphans: failed to remove {name}: {e}");
            }
        }
    }

    Ok(stats)
}

/// Total on-disk size of an entry's audio + timestamps files (existing files only).
pub(super) fn entry_files_size(audio_dir: &Path, entry: &TextEntry) -> u64 {
    let mut total: u64 = 0;
    if let Some(ref name) = entry.audio_path {
        if let Ok(meta) = fs::metadata(audio_dir.join(name)) {
            total += meta.len();
        }
    }
    if let Some(ref name) = entry.timestamps_path {
        if let Ok(meta) = fs::metadata(audio_dir.join(name)) {
            total += meta.len();
        }
    }
    total
}

/// Sum of sizes of every entry's audio + timestamps files.
pub(super) fn current_entries_size(svc: &StorageService) -> u64 {
    let entries: Vec<TextEntry> = svc.entries.read().values().cloned().collect();
    entries
        .iter()
        .map(|e| entry_files_size(&svc.audio_dir, e))
        .sum()
}

/// Drop the audio file for one entry as part of an eviction loop.
/// Returns the number of bytes reclaimed and the number of files removed.
/// `delete_texts = false` keeps the entry in history with audio fields nulled;
/// `delete_texts = true` removes the entry entirely.
fn evict_one(svc: &StorageService, id: &EntryId, delete_texts: bool) -> Result<(u64, u32)> {
    let entry = match svc.get_entry(id) {
        Some(e) => e,
        None => return Ok((0, 0)),
    };
    let freed = entry_files_size(&svc.audio_dir, &entry);
    let mut files = 0u32;
    if entry.audio_path.is_some() {
        files += 1;
    }
    if entry.timestamps_path.is_some() {
        files += 1;
    }

    if delete_texts {
        svc.delete_entry(id)?;
    } else {
        svc.delete_audio(id)?;
    }
    Ok((freed, files))
}

/// Evict the oldest entries until the cumulative on-disk size of all remaining
/// entries' audio + timestamps fits in `target_bytes`.
///
/// `delete_texts = false` (the default for startup cleanup) wipes only the
/// audio + timestamps files and resets the entry to `pending`.
/// `delete_texts = true` removes the entry record from `history.json` too.
///
/// `target_bytes = 0` is treated as "limit disabled" — no-op.
/// Entries currently `processing` are skipped to avoid clobbering an in-flight
/// synthesis.
pub fn evict_to_size(
    svc: &StorageService,
    target_bytes: u64,
    delete_texts: bool,
) -> Result<EvictStats> {
    let mut stats = EvictStats::default();
    if target_bytes == 0 {
        return Ok(stats);
    }

    let mut total = current_entries_size(svc);
    if total <= target_bytes {
        return Ok(stats);
    }

    // Oldest first.
    let mut candidates: Vec<TextEntry> = svc.entries.read().values().cloned().collect();
    candidates.sort_by_key(|e| e.created_at);

    for entry in candidates {
        if total <= target_bytes {
            break;
        }
        if entry.status == EntryStatus::Processing {
            continue;
        }
        // Nothing on disk to free — skipping saves a history write.
        if entry.audio_path.is_none() && entry.timestamps_path.is_none() {
            continue;
        }
        let (freed, files) = match evict_one(svc, &entry.id, delete_texts) {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!("evict_to_size: failed to evict {}: {e}", entry.id);
                continue;
            }
        };
        total = total.saturating_sub(freed);
        stats.deleted_files += files;
        stats.freed_bytes += freed;
        if delete_texts {
            stats.deleted_entries += 1;
            stats.removed_ids.push(entry.id);
        } else {
            stats.updated_ids.push(entry.id);
        }
    }

    Ok(stats)
}

/// Evict every entry's audio. With `delete_texts = true`, every entry is
/// removed from `history.json`; otherwise only the audio fields are reset.
/// Entries currently `processing` are skipped.
pub fn evict_all(svc: &StorageService, delete_texts: bool) -> Result<EvictStats> {
    let mut stats = EvictStats::default();
    let candidates: Vec<TextEntry> = svc.entries.read().values().cloned().collect();

    for entry in candidates {
        if entry.status == EntryStatus::Processing {
            continue;
        }
        if !delete_texts && entry.audio_path.is_none() && entry.timestamps_path.is_none() {
            continue;
        }
        let (freed, files) = match evict_one(svc, &entry.id, delete_texts) {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!("evict_all: failed to evict {}: {e}", entry.id);
                continue;
            }
        };
        stats.deleted_files += files;
        stats.freed_bytes += freed;
        if delete_texts {
            stats.deleted_entries += 1;
            stats.removed_ids.push(entry.id);
        } else {
            stats.updated_ids.push(entry.id);
        }
    }

    Ok(stats)
}

/// Run on app startup: drop orphan files in `audio/`, then trim the cache to
/// fit `target_bytes`. Always uses `delete_texts = false` — automatic deletion
/// of texts is too destructive without an explicit user gesture.
pub fn run_startup_cleanup(svc: &StorageService, target_bytes: u64) -> Result<StartupCleanupStats> {
    let sweep = sweep_orphans(svc)?;
    let evict = evict_to_size(svc, target_bytes, false)?;
    Ok(StartupCleanupStats { sweep, evict })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::schema::WordTimestamp;
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

    fn make_service() -> (StorageService, TempDir) {
        let dir = TempDir::new().unwrap();
        let svc = StorageService::with_cache_dir(dir.path().to_path_buf()).unwrap();
        (svc, dir)
    }

    /// Mark a file's mtime far enough in the past that the orphan sweep won't
    /// treat it as "recent" and skip it.
    fn age_file(path: &Path) {
        let aged = SystemTime::now() - (RECENT_FILE_GRACE * 2);
        // Setting access time too keeps the call cross-platform; we only care about mtime.
        let _ = filetime::set_file_mtime(path, filetime::FileTime::from_system_time(aged));
    }

    /// Create an entry already populated with on-disk audio + timestamps of
    /// the requested sizes. Returns the entry id.
    fn make_ready_entry(svc: &StorageService, audio_bytes: usize, ts_words: usize) -> EntryId {
        let entry = svc.add_entry("test".into()).unwrap();
        let id = entry.id;
        let audio_filename = svc.save_audio(&id, &vec![0u8; audio_bytes]).unwrap();
        let words: Vec<WordTimestamp> = (0..ts_words)
            .map(|i| WordTimestamp {
                word: format!("w{i}"),
                start: i as f64,
                end: i as f64 + 0.5,
                original_pos: (0, 1),
            })
            .collect();
        let ts_filename = svc.save_timestamps(&id, &words).unwrap();
        let mut updated = entry.clone();
        updated.audio_path = Some(audio_filename);
        updated.timestamps_path = Some(ts_filename);
        updated.status = EntryStatus::Ready;
        svc.update_entry(updated).unwrap();
        id
    }

    #[test]
    fn sweep_orphans_removes_unknown_files_only() {
        let (svc, _dir) = make_service();
        let id = make_ready_entry(&svc, 32, 1);
        let entry = svc.get_entry(&id).unwrap();
        let audio_dir = svc.cache_dir().join("audio");

        // Stranger file — should be removed.
        let orphan = audio_dir.join("stranger.opus");
        fs::write(&orphan, b"orphan").unwrap();
        age_file(&orphan);
        // Known files: keep their mtime as-is (recent), should still survive
        // because they're in the valid set.
        let stats = svc.sweep_orphans().unwrap();
        assert_eq!(stats.deleted_files, 1);
        assert!(stats.freed_bytes > 0);
        assert!(!orphan.exists());
        assert!(audio_dir.join(entry.audio_path.unwrap()).exists());
        assert!(audio_dir.join(entry.timestamps_path.unwrap()).exists());
    }

    #[test]
    fn sweep_orphans_preserves_recent_files() {
        let (svc, _dir) = make_service();
        let audio_dir = svc.cache_dir().join("audio");
        // Fresh orphan — sweep must keep it (race with active synthesis).
        let recent = audio_dir.join("in-flight.opus");
        fs::write(&recent, b"in flight").unwrap();
        let stats = svc.sweep_orphans().unwrap();
        assert_eq!(stats.deleted_files, 0);
        assert!(recent.exists());
    }

    #[test]
    fn sweep_orphans_is_idempotent() {
        let (svc, _dir) = make_service();
        let _id = make_ready_entry(&svc, 16, 1);
        // Run twice — second call must be a no-op.
        let _ = svc.sweep_orphans().unwrap();
        let stats = svc.sweep_orphans().unwrap();
        assert_eq!(stats.deleted_files, 0);
        assert_eq!(stats.freed_bytes, 0);
    }

    #[test]
    fn evict_to_size_drops_oldest_first_keeping_entry() {
        let (svc, _dir) = make_service();

        // Three entries with distinct created_at (sleep between adds).
        let id1 = make_ready_entry(&svc, 10_000, 0);
        std::thread::sleep(std::time::Duration::from_millis(5));
        let id2 = make_ready_entry(&svc, 10_000, 0);
        std::thread::sleep(std::time::Duration::from_millis(5));
        let id3 = make_ready_entry(&svc, 10_000, 0);

        // Target is small enough to force at least the oldest two to be evicted.
        let stats = svc.evict_to_size(15_000, false).unwrap();
        assert!(stats.deleted_files >= 2, "at least 2 audio files removed");
        assert!(stats.updated_ids.contains(&id1));
        assert_eq!(stats.deleted_entries, 0);

        // The newest must still have audio.
        let e3 = svc.get_entry(&id3).unwrap();
        assert!(e3.audio_path.is_some());
        // Older entries: audio cleared, status reset to pending.
        let e1 = svc.get_entry(&id1).unwrap();
        assert!(e1.audio_path.is_none());
        assert_eq!(e1.status, EntryStatus::Pending);
        // id2 may or may not have been evicted depending on rounding —
        // assert only the invariant: total <= target.
        let _ = id2;
        let total = current_entries_size(&svc);
        assert!(total <= 15_000, "total {total} must fit target");
    }

    #[test]
    fn evict_to_size_with_delete_texts_removes_entries() {
        let (svc, _dir) = make_service();
        let id1 = make_ready_entry(&svc, 5_000, 0);
        std::thread::sleep(std::time::Duration::from_millis(5));
        let id2 = make_ready_entry(&svc, 5_000, 0);

        let stats = svc.evict_to_size(1, true).unwrap();
        // target 1 byte → both entries must be removed.
        assert!(stats.deleted_entries >= 1);
        assert!(stats.removed_ids.contains(&id1));
        // Newest may survive only if total fit by then; with 0 target both must go.
        assert!(stats.removed_ids.contains(&id2) || svc.get_entry(&id2).is_none());
    }

    #[test]
    fn evict_to_size_target_zero_is_noop() {
        let (svc, _dir) = make_service();
        let id = make_ready_entry(&svc, 1_000, 0);
        let stats = svc.evict_to_size(0, false).unwrap();
        assert_eq!(stats.deleted_files, 0);
        assert_eq!(stats.freed_bytes, 0);
        assert!(svc.get_entry(&id).unwrap().audio_path.is_some());
    }

    #[test]
    fn evict_to_size_below_target_is_noop() {
        let (svc, _dir) = make_service();
        let id = make_ready_entry(&svc, 1_000, 0);
        // Target much higher than current total.
        let stats = svc.evict_to_size(100_000_000, false).unwrap();
        assert_eq!(stats.deleted_files, 0);
        assert!(svc.get_entry(&id).unwrap().audio_path.is_some());
    }

    #[test]
    fn evict_all_keeps_entries_when_delete_texts_false() {
        let (svc, _dir) = make_service();
        let id1 = make_ready_entry(&svc, 200, 0);
        let id2 = make_ready_entry(&svc, 200, 0);

        let stats = svc.evict_all(false).unwrap();
        assert!(stats.deleted_files >= 2);
        assert_eq!(stats.deleted_entries, 0);
        assert!(svc.get_entry(&id1).unwrap().audio_path.is_none());
        assert!(svc.get_entry(&id2).unwrap().audio_path.is_none());
    }

    #[test]
    fn evict_all_removes_history_when_delete_texts_true() {
        let (svc, _dir) = make_service();
        let _id1 = make_ready_entry(&svc, 200, 0);
        let _id2 = make_ready_entry(&svc, 200, 0);

        let stats = svc.evict_all(true).unwrap();
        assert!(stats.deleted_entries >= 2);
        assert!(svc.get_all_entries().is_empty());
    }

    #[test]
    fn run_startup_cleanup_combines_sweep_and_evict() {
        let (svc, _dir) = make_service();
        let id1 = make_ready_entry(&svc, 5_000, 0);
        let id2 = make_ready_entry(&svc, 5_000, 0);

        // Plant an aged orphan.
        let orphan = svc.cache_dir().join("audio").join("ghost.opus");
        fs::write(&orphan, b"ghost").unwrap();
        age_file(&orphan);

        let stats = svc.run_startup_cleanup(7_000).unwrap();
        assert_eq!(stats.sweep.deleted_files, 1);
        // At least one entry's audio was evicted (oldest first).
        assert!(stats.evict.deleted_files >= 1);
        assert!(!orphan.exists());
        // Both entries still in history (delete_texts=false).
        assert!(svc.get_entry(&id1).is_some());
        assert!(svc.get_entry(&id2).is_some());
    }

    #[test]
    fn evict_to_size_skips_processing_entries() {
        let (svc, _dir) = make_service();
        let id_old = make_ready_entry(&svc, 10_000, 0);
        std::thread::sleep(std::time::Duration::from_millis(5));
        let id_proc = make_ready_entry(&svc, 10_000, 0);
        // Force the older one into Processing — eviction loop must skip it
        // even though it would otherwise be picked first.
        let mut e = svc.get_entry(&id_old).unwrap();
        e.status = EntryStatus::Processing;
        svc.update_entry(e).unwrap();

        let _ = svc.evict_to_size(5_000, false).unwrap();
        // Processing entry's audio must remain.
        let still = svc.get_entry(&id_old).unwrap();
        assert!(still.audio_path.is_some());
        let _ = id_proc;
    }
}
