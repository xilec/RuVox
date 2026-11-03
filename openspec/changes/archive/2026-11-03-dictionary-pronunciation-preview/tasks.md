# Tasks — dictionary pronunciation preview

## 1. OpenSpec

- [x] 1.1 Proposal, design, tasks and deltas for `user-dictionary`
  ("Editor pronunciation preview") and `ipc-commands`
  ("Pronunciation Preview Command").
- [x] 1.2 `openspec validate dictionary-pronunciation-preview --strict` and
  `openspec validate --specs --strict`.

## 2. Backend

- [x] 2.1 `commands/synthesis.rs`: `voice_for_engine` becomes `pub(super)`.
- [x] 2.2 `state.rs`: `preview_dir` (resolved from the app cache dir at
  startup, a `TempDir` in tests) and `preview_lock` (`tokio::sync::Mutex`).
- [x] 2.3 `commands/mod.rs`: `preview_pronunciation(text, mode)` —
  `normalize` via `preview_normalization`, `spoken` via
  `tts_normalize::dictionary::spoken_form` plus the profile's stress rule;
  busy guard on `synthesize_entered`; synthesize through `state.tts.synthesize`
  into a unique file under `preview_dir`, read, delete, return
  `{ spoken_text, wav_base64, duration_sec }`.
- [x] 2.4 `lib.rs`: register the command and wire the new state fields;
  `test_support.rs`: the same for the mock app.
- [x] 2.5 `Cargo.toml`: `base64`.
- [x] 2.6 `tauri.conf.json`: `media-src 'self' blob:`.

## 3. Frontend

- [x] 3.1 `lib/tauri.ts`: `PreviewPronunciationResult` type and
  `previewPronunciation(text, mode)` binding.
- [x] 3.2 `lib/engineSelection.ts`: `voiceForEngine(engine, config)` for the
  cache key.
- [x] 3.3 `DictionaryModal.tsx`: play/stop control in `rightSection` of both
  fields (next to the help icon), disabled for empty/invalid, spinner blocking
  both controls, stop while playing; one `<audio>` element with object-URL
  revocation; narration paused through `pause_playback`; cache; inline error;
  active-engine hint for a foreign scope; stop and release on editor/modal
  close.
- [x] 3.4 `i18n/ru.ts`, `i18n/en.ts`: tooltips, aria-labels, the engine hint,
  the failure line and its short details.

## 4. Tests

- [x] 4.1 Rust: `normalize` mode returns the pipeline reading; `spoken` mode
  keeps the mark for a stress-capable profile and strips it for a
  `Stress::None` profile; malformed mark rejected; response shape (base64 WAV,
  duration); no history entry, timestamps, queue marker or `entry_updated`
  event; busy guard while a queue entry is inside `synthesize`; the temp file
  is gone after the call.
- [x] 4.2 Frontend: controls disabled for empty and invalid fields; play calls
  the command with the right mode; spinner blocks both controls; second click
  stops; repeat click uses the cache; failure renders the inline line and keeps
  the values; closing the editor stops the audio and revokes the URL; narration
  is paused and not resumed; the engine hint appears for a foreign scope and
  not for the active one.

## 5. Validation

- [x] 5.1 `cargo test --manifest-path src-tauri/Cargo.toml`
- [x] 5.2 `pnpm typecheck`
- [x] 5.3 `pnpm test:unit`
- [x] 5.4 `just lint`
- [x] 5.5 `openspec validate --specs --strict`, then archive the change and
  sync the specs.
- [x] 5.6 CHANGELOG `[Unreleased]` note.
