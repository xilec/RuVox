## 1. Reproduction and command changes

- [x] 1.1 Add the failing orchestration test first: a `ready` entry with a
  sentinel audio file, `StubEngine::fail_with`, `regenerate_entry` → assert
  the current code loses the file (red), then keep the test as the regression
  lock with the fixed expectations.
- [x] 1.2 `regenerate_entry`: stop calling `delete_audio`; keep `audio_path`,
  `timestamps_path`, `duration_sec`, `audio_generated_at` and `generation`;
  set the status to `pending` explicitly (previously a side effect of
  `delete_audio`), keep `was_regenerated: true` / `error_message: null`, and
  update the doc comment to state the non-destructive contract.
- [x] 1.3 `apply_error_if_current`: pick the status from the entry's own
  `audio_path` — `ready` when it still owns a recording, `error` otherwise;
  keep the message and the `tts_error` event path.
- [x] 1.4 `cancel_entry`: restore `ready` for an entry that still owns a
  recording, `cancelled` otherwise; update the function's doc comment.
- [x] 1.5 `discard_late_files`: take the entry id and skip candidate names the
  entry still references (`audio_path` / `timestamps_path`); update both call
  sites.
- [x] 1.6 Toast routing (`src/lib/notificationBridge.ts`): a `ready`
  transition only announces a completion when `generation_count` moved since
  the spinner appeared; otherwise hide the spinner — a restored recording is
  not a new synthesis.

## 2. Tests

- [x] 2.1 Failed regeneration: sentinel bytes survive, status returns to
  `ready`, `error_message` carries the engine message, `was_regenerated` is
  true, and `tts_error` + `entry_updated` are emitted; the restored audio is
  playable again.
- [x] 2.2 A failed first synthesis (no recording) still lands on `error`
  (regression guard for D2's other branch) — pinned by the existing
  `tts_failure_emits_entry_updated_error_then_tts_error`.
- [x] 2.3 Cancelled regeneration (`block_synthesis` + `cancel_synthesis`):
  status returns to `ready`, sentinel bytes intact, synthesis task aborted,
  and `play_entry` accepts the entry.
- [x] 2.4 `discard_late_files`: referenced name survives, unreferenced name is
  removed.
- [x] 2.5 Update the existing `regenerate_entry_replaces_audio_and_resynthesizes`
  test comments to describe in-place replacement instead of a pre-synthesis
  delete.
- [x] 2.6 Frontend unit test (`notificationBridge.test.ts`): a moved counter
  announces the completion, an unchanged counter hides the spinner, an `error`
  transition hides it.

## 3. Documentation and validation

- [x] 3.1 CHANGELOG `[Unreleased]` entry (user-visible: a failed or cancelled
  regeneration no longer loses the previous recording).
- [ ] 3.2 `pnpm dlx @fission-ai/openspec@1.6.0 validate
  preserve-audio-on-regeneration --strict`.
- [ ] 3.3 `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo test --manifest-path src-tauri/Cargo.toml`, `pnpm typecheck` and
  `pnpm test:unit`.
- [ ] 3.4 Manual pass on the dev app: with TeraTTS active, regenerate an entry
  whose text makes the engine fail and confirm the old audio still plays; then
  cancel a regeneration and confirm the entry stays playable.
