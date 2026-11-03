# Design — dictionary pronunciation preview

## Context

The dictionary editor (`src/dialogs/DictionaryModal.tsx`) is one shared,
always-mounted modal in `AppShell`; its add/edit form is `editor !== null`.
Both fields («Слово», «Как произносить») already render a help `ActionIcon` in
`rightSection`.

Backend pieces that exist today:

- `preview_normalization` (in `commands/mod.rs`) runs the Rust pipeline on a
  blocking thread and returns the normalized text plus the char mapping. It
  takes no storage/TTS handles, so it cannot create entries.
- `tts_normalize::dictionary::spoken_form(to)` validates the optional `+` mark
  (one mark, immediately before a Russian vowel) and returns the text with the
  mark removed. `UserDictionary::resolve(word, mark_stress)` is the pipeline's
  insertion rule: with `mark_stress` the stored `to` is used as typed, without
  it the mark is stripped. `TTSPipeline` sets `mark_stress` from
  `profile.stress != Stress::None`.
- `TtsEngine::synthesize(text, voice, sample_rate, out_wav, char_mapping)`
  writes a WAV to `out_wav` and returns `SynthesizeOutput { timestamps,
  duration_sec }`. `voice_for_engine(kind, config)` (in `commands/synthesis.rs`)
  resolves the voice id per engine; `config.sample_rate` is the shared rate.
- `synthesize_audio` is the *queue* path: it stages the file, registers the
  entry, writes timestamps, emits events and runs the Opus transcode. The
  preview must not touch any of that.

## Goals / Non-Goals

**Goals:**

- Hear «Слово» as the pipeline reads it today, and «Как произносить» as the
  typed spoken form will sound on the active engine.
- No side effects: no entry, no timestamps, no queue state, no synthesis
  events, no player involvement.
- The preview never fights the queue and never leaves a file behind.
- The control's state machine (disabled → spinner → stop) is testable without
  a real engine.

**Non-Goals:**

- Playing through mpv or the entry lifecycle.
- A second engine instance or an engine switch for the preview.
- Persisting the audio anywhere.

## Decisions

**The command runs the pipeline and the engine directly.** `mode="normalize"`
reuses `preview_normalization` with `state.tts.text_profile()` — exactly what
`preview_normalize` does — and speaks the returned text as-is (it is already
stressed for engines that render stress). `mode="spoken"` validates with
`spoken_form` and then applies the same decision the pipeline applies when it
inserts a dictionary replacement: keep the typed text (mark included) when
`state.tts.text_profile().stress != Stress::None`, use the stripped form
otherwise. The engine is called through `state.tts.synthesize(...)`, never
`synthesize_audio`. Rejected: synthesizing a throwaway entry and playing it
(writes storage and timestamps, emits queue/synthesis events, leaves debris if
the app closes mid-preview); reusing `preview_normalize` + a second command for
the audio (two IPC round trips and a window where the pipeline output and the
spoken audio disagree).

**Audio travels as base64 and is played from a blob URL.** The engine API
writes a file, so the command synthesizes into
`<app cache>/preview/<uuid>.wav`, reads the bytes, deletes the file and returns
`{ spoken_text, wav_base64, duration_sec }`. The webview turns the base64 into
a `Blob` and an object URL; the CSP only needs `media-src 'self' blob:`.
Rejected: returning a path and serving it through the asset protocol (needs
`assetProtocol` plus its own CSP scope, and leaves the file on disk until the
next cleanup). The cache directory is resolved once at startup into
`AppState.preview_dir` (a `TempDir` in tests) so the command never guesses a
platform path and tests stay hermetic.

**The preview refuses to start while the queue is synthesizing.** A
`synthesize_entered` entry means the engine is inside `tts.synthesize` for a
queue entry. The engines do tolerate a second caller structurally — the `ttsd`
driver serializes requests over one pipe, Piper holds a model mutex for the
whole chunk loop, both ONNX engines mutex their sessions — but the preview
would wait behind a narration that can run for minutes with only a spinner to
show for it, and TeraTTS shares one `cancel` flag between runs
(`run_synthesis` resets it at start, `kill_current` sets it): a concurrent
preview would clear a narration's cancellation, and cancelling the narration
would abort the preview. The command therefore returns a typed
`preview.engine_busy` error while `synthesize_entered` is non-empty, and the
form renders it as a short inline line («движок занят озвучиванием»). Previews
are additionally serialized among themselves by a dedicated
`tokio::sync::Mutex` in `AppState`. Rejected: a shared mutex taken by both the
queue path and the preview (it would serialize queue syntheses with each
other — TeraTTS deliberately runs several lanes — and a preview would hold a
narration up for its duration); disabling the controls in the UI while an
entry is processing (racy, and the modal cannot see a synthesis started from
the tray).

The remaining race — a queue synthesis that starts after the busy check — is
accepted and documented: every engine survives the overlap structurally, and
the worst case is a cancel of that narration aborting the preview with a typed
error that surfaces inline and is retryable. Nothing in storage or the queue is
involved.

**One `<audio>` element per dialog, cache keyed by the request.** The dialog
owns one `<audio ref>`; starting a preview revokes the previous object URL,
sets the new one, pauses narration through the existing `pause_playback`
command (never `stop_playback`, and no resume afterwards) and calls `play()`.
`ended` and `error` clear the playing state; closing the editor or the modal
stops the element and revokes the URL.

The in-memory cache is keyed by
`(engine, voice, mode, request text, dictionary revision)`. The issue names
`(engine, voice, spoken_text)`, but the response's `spoken_text` is only known
after the round trip — using it as the key would require a second synthesis to
look it up. The request inputs plus the dictionary state determine the audio
deterministically, and a save/import bumps the revision so a cached «Слово»
reading cannot outlive a dictionary write. Rejected: caching by `spoken_text`
after the response only (does not satisfy "a second click does not
synthesize"); no cache invalidation on save (a stale reading after the user
just added the entry).

**The engine hint reuses the Settings fallback logic.** The active engine can
differ from the persisted `config.engine` when the startup probe fell back
(e.g. a missing Silero bundle serves Piper this session). The modal fetches
`get_config` + `get_available_engines` on open and runs
`computeEngineFormState(config, availability).engine` — the same reducer
Settings uses — so the hint names the engine that will actually speak. The
voice for the cache key comes from a small `voiceForEngine(engine, config)`
helper mirroring the backend `voice_for_engine`. Rejected: passing
`config.engine` from `AppShell` (would name the persisted engine, not the
serving one).

**Failures are inline, never a toast.** The form keeps the entered values and
renders one short line under the fields: the preview error prefix plus a short
detail for the known cases (`bundle_not_installed` / `model_not_loaded` →
«модель не установлена», `voice_not_installed` → «голос не установлен»,
`preview.engine_busy` → «движок занят озвучиванием»), otherwise the localized
`formatError` detail. A notification would steal attention from a form the
user is still filling and could hide the entered values behind a modal stack.
