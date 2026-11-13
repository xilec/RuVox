## 1. Preview player backend

- [x] 1.1 Add the `preview` plugin window label to `player/mod.rs` and expose `resolve_mpv_path` to the new module.
- [x] 1.2 Create `src-tauri/src/player/preview.rs`: `PreviewAudioError`, the `PreviewAudioBackend` trait (`play(&[u8])`, `stop()`), and `MpvPreviewAudio` writing `<cache>/preview/<uuid>.wav`, starting an audio-only mpv instance, and destroying it plus the file on stop.
- [x] 1.3 Serialize play/stop in `MpvPreviewAudio` and destroy the instance from `Drop`.
- [x] 1.4 Add `preview_audio: Arc<dyn PreviewAudioBackend>` to `AppState`, construct it in `lib.rs`, and destroy the instance on `RunEvent::Exit`.
- [x] 1.5 Add a fake backend to `test_support.rs` and expose it through `TestApp` beside the fake player.

## 2. IPC commands

- [x] 2.1 Implement `preview_play(wav_base64)` with base64 decoding, the 8 MiB cap, and the typed `preview.invalid_audio` / `preview.audio_too_large` / `preview.play_failed` errors.
- [x] 2.2 Implement idempotent `preview_stop` with `preview.stop_failed` while still deleting the WAV.
- [x] 2.3 Register both commands in `invoke_handler`.
- [x] 2.4 Add command tests: a valid payload reaches the fake and starts it, malformed and oversized payloads are rejected before playback, stop reaches the fake, stop with nothing playing succeeds, and a backend failure maps to the typed error.

## 3. Frontend

- [x] 3.1 Add `previewPlay` and `previewStop` wrappers to `src/lib/tauri.ts`.
- [x] 3.2 Replace the dialog's `<audio>` element, blob URL helpers and media refs with the commands; keep the in-memory cache and the pause-before-synthesis order.
- [x] 3.3 Stop the preview and clear its state after `duration_sec` plus a 500 ms margin, on a second click, on editor close and on unmount; guard the timer with the existing request sequence.
- [x] 3.4 Show the existing `dictionary.preview.error.play_failed` inline message when `preview_play` rejects, and ignore responses that arrive after close.
- [x] 3.5 Update the dialog tests: play/stop command calls, cache replay without a second synthesis, auto-stop after the duration, playback failure, close stops, and no `<audio>`/`<video>` element in the dialog.

## 4. Config and release notes

- [x] 4.1 Remove `media-src 'self' blob:` from the CSP in `src-tauri/tauri.conf.json`.
- [x] 4.2 Add a 1-2 line `[Unreleased]` / Fixed entry to `CHANGELOG.md` for the user-visible crash fix.

## 5. Validation

- [x] 5.1 `cargo fmt --check`, `cargo clippy -- -D warnings`, and the app crate's `cargo test`.
- [x] 5.2 `pnpm typecheck`, `pnpm eslint`, and `pnpm test:unit`.
- [x] 5.3 `openspec validate --specs --strict` (pinned 1.6.0) passes for this change.
- [ ] 5.4 Manual pass: start the app, play both preview controls, stop one mid-clip, close the editor while playing, and confirm the WebKit web process never crashes and no preview WAV remains in the cache directory.
