# ipc-commands Delta

## MODIFIED Requirements

### Requirement: Cache Management Commands

The system SHALL provide `clear_cache(args)`, `get_cache_stats()`, and
`get_cache_dir()`.

`clear_cache` takes `{ mode, delete_texts }` where `mode` is
`{ mode: "size_limit", target_mb }` or `{ mode: "all" }` and `delete_texts`
defaults to `false`. It SHALL always sweep orphan files in the audio directory,
then evict entries per the mode. With `delete_texts: false` evicted entries keep
their history records with `audio_path: null` and status reset to `pending`
(emitting `entry_updated` per entry); with `delete_texts: true` they are removed
from history (emitting `entry_removed` with `{ id }` per entry). Entries with
status `processing` SHALL be skipped. The command returns
`{ deleted_files, deleted_entries, freed_bytes }`.

`get_cache_stats` SHALL return `{ total_bytes, audio_file_count }`.
`get_cache_dir` SHALL return the absolute path of the per-user **data directory**
resolved at startup — the root holding `history.json` and `audio/`.

#### Scenario: size-limit eviction keeps texts
- GIVEN a cache exceeding `target_mb` and `delete_texts: false`
- WHEN `clear_cache` is invoked
- THEN oldest entries are evicted until the cache fits, each evicted entry emits `entry_updated` with status `pending`, and the result reports the counts and freed bytes

#### Scenario: full eviction removes texts
- GIVEN `mode: "all"` and `delete_texts: true`
- WHEN `clear_cache` is invoked
- THEN all audio is dropped, entries are removed from history, and `entry_removed` is emitted per removed entry
