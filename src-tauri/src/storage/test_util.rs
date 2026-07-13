//! Test-only helpers shared by the storage unit tests (`service.rs`,
//! `eviction.rs`) and by the synthesis tests in `crate::commands`.
//!
//! Unlike `tts::supervisor::test_helpers`, this module doesn't need a
//! `test-helpers` Cargo feature: every consumer lives inside this crate, so
//! plain `#[cfg(test)]` + `pub(crate)` (see the `mod` declaration in
//! `storage/mod.rs`) is enough.

use chrono::{Local, NaiveDateTime};
use tempfile::TempDir;

use crate::storage::schema::{EntryId, EntryStatus, TextEntry, WordTimestamp};
use crate::storage::service::StorageService;

/// Build a [`StorageService`] backed by a fresh temp dir. The returned
/// [`TempDir`] must be kept alive for as long as the service is used —
/// dropping it removes the directory from disk.
pub(crate) fn make_service() -> (StorageService, TempDir) {
    let dir = TempDir::new().unwrap();
    let svc = StorageService::with_cache_dir(dir.path().to_path_buf()).unwrap();
    (svc, dir)
}

/// Add a plain entry and immediately overwrite its `created_at` with an
/// explicit value. Lets tests build entries with well-ordered timestamps
/// without sleeping between `add_entry` calls.
pub(crate) fn add_entry_at(
    svc: &StorageService,
    text: &str,
    created_at: NaiveDateTime,
) -> TextEntry {
    let mut entry = svc.add_entry(text.to_string()).unwrap();
    entry.created_at = created_at;
    svc.update_entry(entry.clone()).unwrap();
    entry
}

/// Create an entry already populated with on-disk audio + timestamps of the
/// requested sizes, using `Local::now()` as `created_at`. Suitable when a
/// test doesn't assert anything about ordering between entries.
pub(crate) fn make_ready_entry(
    svc: &StorageService,
    audio_bytes: usize,
    ts_words: usize,
) -> EntryId {
    make_ready_entry_at(svc, audio_bytes, ts_words, Local::now().naive_local())
}

/// Same as [`make_ready_entry`] but with an explicit `created_at`, so tests
/// that assert oldest/newest ordering get distinct, deterministic timestamps
/// instead of relying on `thread::sleep` between calls.
pub(crate) fn make_ready_entry_at(
    svc: &StorageService,
    audio_bytes: usize,
    ts_words: usize,
    created_at: NaiveDateTime,
) -> EntryId {
    let entry = add_entry_at(svc, "test", created_at);
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

    let mut updated = svc.get_entry(&id).unwrap();
    updated.audio_path = Some(audio_filename);
    updated.timestamps_path = Some(ts_filename);
    updated.status = EntryStatus::Ready;
    svc.update_entry(updated).unwrap();
    id
}
