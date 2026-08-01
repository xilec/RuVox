# Tasks: engine-aware-input-limit

## 1. Engine-aware validation

- [x] 1.1 Change `validate_input_length` in `src-tauri/src/commands/mod.rs` to take the active `EngineKind` and reject only when it is `EngineKind::Piper`; update the Russian message to name Piper and suggest shortening the text or switching to Silero; refresh the `MAX_INPUT_CHARS` doc comment (it is now a Piper-only guard)
- [x] 1.2 Pass `state.tts.kind()` at both call sites: `ingest_text` and `preview_normalize`

## 2. Test support

- [x] 2.1 Add `build_test_app_with_kind(kind: EngineKind)` in `src-tauri/src/test_support.rs` installing the `EngineSwitcher` with the given initial kind; make `build_test_app()` delegate with `EngineKind::Piper`

## 3. Tests

- [x] 3.1 Keep the existing oversized-input rejection tests green (they run on the Piper-kind default)
- [x] 3.2 Add a test: with `EngineKind::Silero`, `add_text_entry` accepts text longer than `MAX_INPUT_CHARS` and the entry reaches `ready`
- [x] 3.3 Add a test: with `EngineKind::Silero`, `preview_normalize` normalizes text longer than `MAX_INPUT_CHARS` (asserting full length, no truncation)
- [x] 3.4 Assert the Piper rejection message names the engine and the limit

## 4. Synthesis-time re-check (review fix)

- [x] 4.1 Centralize the check in `oversized_input_message(text, kind)`; add `SynthesisError::InputTooLong` and re-check the guard in `spawn_synthesis` against the engine active at synthesis start
- [x] 4.2 Add a test: an oversized entry inserted under Silero fails synthesis under Piper with the limit message and status `error`
- [x] 4.3 Update the `TtsEngine::kind()` doc comment — the kind is load-bearing for engine-aware decisions, not logs-only

## 5. Gates

- [x] 5.1 `nix develop -c just test` green
- [x] 5.2 `nix develop -c just lint` green
- [ ] 5.3 Manual: with Piper active, paste >100k chars — rejection toast names Piper and the Silero option; with Silero active, a >100k paste is ingested and synthesizes (chunked) without a hang
