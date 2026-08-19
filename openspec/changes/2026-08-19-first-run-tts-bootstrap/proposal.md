# Proposal: first-run-tts-bootstrap

Closes #196. Part of #185 (Windows support), blocks the v0.3.0 release.

## Problem

On a fresh install (observed on the v0.3.0 Win10 VM pass) the default
configuration (`engine = "silero_native"`, no model bundle downloaded yet)
makes the very first synthesis fail:

1. `build_engine` (`src-tauri/src/lib.rs`) cannot serve Silero Native without
   the bundle, so it silently falls back to Piper **for this run** — the
   persisted config is untouched.
2. `synthesize_audio` (`src-tauri/src/commands/mod.rs`) picks the voice by the
   **persisted** `config.engine`, not by the engine actually running: Piper
   receives the Silero speaker id (`aidar`) and answers
   `voice_not_installed`.
3. The Piper auto-download recovery then tries to fetch `aidar` from the
   Piper voice catalog, where it does not exist — the retry fails and the
   user sees `Piper voice "aidar" не установлен` on the very first Add.

Two more gaps around the same first-run experience:

- Nothing tells the user that the default engine needs a one-time ~230 MB
  bundle download; the app just silently runs on Piper.
- When a Piper voice is missing, the existing silent auto-download works but
  its behavior is unspec'd; the failure above shows what happens when the
  recovery is aimed at the wrong voice.

## Change

1. **Voice follows the active engine.** `synthesize_audio` selects the voice
   (`piper_voice` vs `speaker`) and gates the auto-download retry on
   `tts.kind()` — the engine actually serving the request — instead of the
   persisted `config.engine`.
2. **First-run bundle prompt.** On startup, when the persisted engine is
   `silero_native` and the bundle probe reports unavailable, the app shows a
   modal offering to download the Silero Native bundle (~230 MB, one time) or
   continue on Piper. Accepting downloads with an inline progress bar (the
   existing `bundle_download_*` events); on success the engine is switched to
   Silero Native via `update_config`. Declining closes the prompt for this
   run; it reappears on the next launch while the condition holds.
3. **Spec the Piper auto-download.** No code change beyond (1): when
   synthesis on Piper hits `voice_not_installed`, the voice is fetched
   automatically with visible progress notifications, and only a failed
   download surfaces an error.

## Scope

- Backend: `src-tauri/src/commands/mod.rs` (`synthesize_audio` voice
  selection + retry gate) and unit tests.
- Frontend: new `src/dialogs/SileroBundlePrompt.tsx`, wired into
  `AppShell`'s config-load effect; pure decision helper in `src/lib/` for
  testability.
- OpenSpec deltas: `ipc-commands` (voice selection, auto-download), `ui`
  (first-run prompt).

## Non-goals

- No "don't ask again" persistence for the prompt (per-launch nag is the
  agreed behavior for 0.3.x).
- No changes to the Settings dialog's engine coercion logic.
- ttsd/Silero fallback chains on Linux are unchanged.

## Risks

- `tts.kind()` is read before the synthesize call; a mid-flight engine swap
  could theoretically desync voice and engine. Benign: the same race exists
  today for the input-length guard, and a wrong-voice synthesis fails loudly
  rather than corrupting output.
- The prompt subscribes to `bundle_download_*` events that Settings also
  listens to when open; both consumers are independent notification/progress
  views and do not conflict.
