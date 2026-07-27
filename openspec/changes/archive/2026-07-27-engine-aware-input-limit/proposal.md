# Proposal: engine-aware-input-limit

## Why

The 100 000-codepoint input limit was introduced in `fix-pipeline-quadratic` as
a guard against Piper's unchunked one-shot ONNX inference (CPU wedge + OOM risk,
see issue #155). But the limit currently applies unconditionally, including when
the active engine is Silero — which already chunks text (`ttsd` splits into
≤900-char chunks and synthesizes chunk by chunk), so long texts are safe there.
Users on Silero are blocked from ingesting long articles for no reason.

## What Changes

- The 100 000-codepoint input length check in `ingest_text` (covers
  `add_text_entry` / `add_clipboard_entry`) and `preview_normalize` is enforced
  only when the currently-active TTS engine is Piper. With Silero active, input
  of any length is accepted (bounded in practice by normalization time, which is
  near-linear since `fix-pipeline-quadratic`).
- The rejection message is updated to name the Piper engine and suggest
  switching to Silero in Settings or shortening the text.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `text-pipeline`: the "Input length limit" requirement becomes engine-aware —
  the limit applies only when the active TTS engine is Piper.
- `ipc-commands`: the oversized-input rejection scenarios for
  `add_text_entry`/`add_clipboard_entry` and `preview_normalize` apply only with
  Piper active; the message names the engine and points to the Silero option.

## Impact

- `src-tauri/src/commands/mod.rs`: `validate_input_length` gains the active
  engine kind (`AppState.tts.kind()` / `EngineSwitcher` atomic kind) and gates
  the rejection on Piper; message text updated.
- `src-tauri/src/commands/orchestration_tests.rs`: existing oversized-input
  tests now run against a Piper-kind stub; new tests cover Silero-kind acceptance
  of oversized input.
- No IPC signature changes; no frontend changes (errors still surface via the
  same toast path).

## Non-goals

- Fixing Piper's OOM itself (chunking before Piper) — tracked in issue #155.
- Re-tuning the 100 000 value or adding per-engine configurable limits.
- Any change to Silero-side chunking (`ttsd/ttsd/chunking.py`).
