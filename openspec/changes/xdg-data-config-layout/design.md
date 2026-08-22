# Design: xdg-data-config-layout

## Context

`StorageService` resolves one per-user root via `crate::paths::storage_root()` and puts
everything there: Linux `~/.cache/ruvox/` (volatile — cache cleaners may wipe it),
Windows `%LOCALAPPDATA%\com.ruvox.app` (already correct since #200/#201).
`paths::voices_root()` already points at `dirs::data_local_dir()/ruvox/voices`, so the
data root this change introduces matches an existing convention. `history.json`
already has corrupted-file `.bak` recovery (`load_history`); `config.json` does not —
a torn write resets all user settings silently.

## Goals / Non-Goals

- Goals: XDG-correct locations on Linux; silent idempotent migration from the legacy
  path; config.json `.bak` recovery; zero persisted-format changes.
- Non-goals: see proposal (no Windows changes, no migration UI, no startup-failure
  handling — that is #223).

## Decisions

1. **Two roots, resolved in `paths.rs`:** `data_root()` = `dirs::data_local_dir()/ruvox`
   (Linux) / unchanged Windows identifier dir; `config_root()` = `dirs::config_dir()/ruvox`
   (Linux) / same Windows identifier dir. Alternative rejected: keep everything under
   `data_local_dir` — violates the issue's acceptance criteria and the XDG expectation
   that hand-editable state lives under `XDG_CONFIG_HOME`; also rejected: move Windows
   config to Roaming (`dirs::config_dir()`) — no cache-cleaner problem exists there,
   and keeping Windows untouched preserves the NSIS uninstaller contract verbatim.
2. **Constructor shape:** production `StorageService::new()` resolves both roots and
   runs migration; tests keep the single-dir `with_cache_dir(root)` (config colocated
   next to history, as today) plus a new `with_data_and_config_dirs(data, config)` for
   split-layout/migration tests. Alternative rejected: convert every call site to two
   dirs — large test churn with no behavioral gain.
3. **Migration mechanics (runs inside `new()` only):**
   - Triggered when the legacy root (`dirs::cache_dir()/ruvox`) exists; per item
     (`audio/`, `config.json`, then `history.json`) the move happens only when the
     destination is absent — idempotent and tolerant of partial earlier runs.
   - Move order audio → config → history: entries are validated against `audio/`
     contents during load, so audio must be in place before history loads, or a crash
     between the two moves would reset ready entries to pending.
   - `fs::rename` first; on cross-filesystem failure fall back to copy + source delete
     only after a complete successful copy (partial copies land in the destination and
     are later cleaned by the orphan sweep's 60-second grace logic — acceptable,
     logged).
   - After moving, delete the legacy dir if empty (`remove_dir`, non-recursive —
     anything unexpected left inside keeps the dir alive and visible for inspection).
   - Any per-item failure logs an error and continues startup without that item;
     degraded behavior equals today's post-cache-cleaner behavior (entries reset),
     which is strictly better than blocking launch.
4. **Config recovery mirrors history:** parse failure renames to `config.json.bak` and
   returns `UIConfig::default()`; IO read failure warns and returns defaults.
5. **IPC surface:** `get_cache_dir` keeps its name (frontend call sites and spec stay
   stable) but returns the data root; Settings label becomes "Папка данных".

## Risks / Trade-offs

- [Downgrade to a pre-change build shows empty history] → accepted: pre-release app,
  no external users yet; files remain on disk under the new root.
- [Cross-fs `$HOME` mounts make rename fail] → copy+delete fallback; orphan sweep
  absorbs any partial-copy debris.
- [Legacy dir removal races nothing] → single-process owns the dir; non-recursive
  remove cannot destroy unexpected content.

## Migration Plan

Automatic and silent on first launch of the new build. Rollback: none needed beyond
reverting the code; old builds simply see an empty legacy dir.

## Open Questions

None.
