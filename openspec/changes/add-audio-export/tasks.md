# Tasks: add-audio-export

## 1. Backend

- [x] 1.1 `src-tauri/src/commands/mod.rs` (or a sibling module): add
      `pick_export_audio_path(entry_id)` — storage read of the entry under
      the lock, `rfd::FileDialog::save_file` with the stored-format filter
      and `ruvox-<entry_id>.<ext>` default name on `spawn_blocking`;
      `entry.not_found` / `export.no_audio` errors; `None` on cancel.
- [x] 1.2 Add the dialog-free `export_audio_to(storage, entry_id, path)`
      core (resolve `audio/<audio_path>` under the lock, blocking copy) and
      the `export_audio(entry_id, path)` command wrapper; errors
      `entry.not_found` / `export.no_audio` / `export.copy_failed`; register
      both commands in the invoke handler.

## 2. Backend tests

- [x] 2.1 Unit tests for `export_audio_to` via the storage test util:
      success copies bytes and leaves the cache file intact; missing
      `audio_path` / missing file → `export.no_audio`; missing entry →
      `entry.not_found`; copy into a nonexistent directory →
      `export.copy_failed`.

## 3. Frontend

- [x] 3.1 `src/lib/tauri.ts`: `pickExportAudioPath(entryId)` and
      `exportAudio(entryId, path)` wrappers.
- [x] 3.2 `src/i18n/{ru,en}.ts`: `queue.menu.export_audio`
      («Сохранить аудио как…»), `notify.export.ok` («Аудио сохранено: {0}»),
      `errors.export.no_audio`, `errors.export.copy_failed`.
- [x] 3.3 `src/components/QueueList.tsx`: menu item between Play and
      Regenerate, gated like Play; handler pick → cancel no-op → export →
      success notification with path / red error via `formatError`.

## 4. Frontend tests

- [x] 4.1 `QueueList.test.tsx`: menu action invokes pick + export with the
      entry id and chosen path, shows the success notification; cancelled
      pick invokes nothing else; rejected export shows the red error;
      disabled for non-ready/non-playing entries.

## 5. Validation

- [x] 5.1 `nix develop -c just lint` and `nix develop -c just test` green.
- [ ] 5.2 Manual pass (checklist to the user): export a ready entry on real
       storage — dialog pre-filled, file lands on disk and plays; cancel is
       a no-op; error paths surface localized messages; no `entry_updated`
       side effects.
- [ ] 5.3 Propose the additive `CHANGELOG.md` `[Unreleased]` entry as a diff
      for user approval (human-owned file).
