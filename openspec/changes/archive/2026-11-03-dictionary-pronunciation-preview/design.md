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
  `profile.stress != Stress::None`, and the preview's `spoken` mode uses the
  same test.
- `TtsEngine::synthesize(text, voice, sample_rate, out_wav, char_mapping)`
  writes a WAV to `out_wav` and returns `SynthesizeOutput { timestamps,
  duration_sec }`. `voice_for_engine(kind, config)` (in `commands/synthesis.rs`)
  resolves the voice id per engine; `config.sample_rate` is the shared rate.
- `synthesize_audio` is the *queue* path: it stages the file, registers the
  entry, writes timestamps, emits events and runs the Opus transcode. The
  preview must not touch any of that.

The stress policy is easy to mix up — it is per profile, not per engine
generation:

| Engine kind     | Profile                 | Stress   | Explicit `+` |
|-----------------|-------------------------|----------|--------------|
| `silero` (ttsd) | `SILERO_PROFILE`        | `None`   | stripped     |
| `silero_native` | `SILERO_NATIVE_PROFILE` | `Auto`   | kept         |
| `piper`         | `PIPER_PROFILE`         | `None`   | stripped     |
| `teratts`       | `TERATTS_PROFILE`       | `Manual` | kept         |

An explicit mark reaches the engine only when `stress != Stress::None`: `Auto`
lets it override the predicted stress, `Manual` renders only explicit marks.

## Goals / Non-Goals

**Goals:**

- Hear «Слово» as the pipeline reads it today, and «Как произносить» as the
  typed spoken form will sound on the active engine.
- No side effects: no entry, no timestamps, no queue state, no synthesis
  events, no player involvement.
- The preview never queues behind a narration and never leaves a file
  behind.
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

**The preview and the queue share one engine lock.** `AppState.preview_lock`
serializes every engine call. The queue's `synthesize_audio` holds it around
`tts.synthesize_spans` (and around the Piper voice-download retry); the preview
holds it around its own `tts.synthesize`. The preview takes it with `try_lock`,
so it never waits behind a narration that can run for minutes: when the lock is
held it returns a typed `preview.engine_busy` error, and the form renders it as
a short inline line («движок занят озвучиванием»). In the other direction the
queue waits for at most one short preview, which keeps a single engine from
serving two callers at once — with TeraTTS a second caller would otherwise race
the shared cancellation flag (`run_synthesis` resets it at start,
`kill_current` sets it), clearing a narration's cancellation or aborting the
preview. The `synthesize_entered` marker is set inside the lock, so "marker
non-empty" stays equivalent to "the engine is really taken" and a preview
cannot slip between its check and the engine call. Concurrent previews are
serialized by the same lock.

Rejected: letting the preview wait on the lock (the user would watch a spinner
behind a whole narration); a shared lock around the whole queue synthesis
instead of just the engine call (normalization and post-processing would
serialize too); disabling the controls in the UI while an entry is processing
(racy, and the modal cannot see a synthesis started from the tray).

**One `<audio>` element per dialog, cache keyed by the request.** The dialog
owns one `<audio ref>`. Starting a preview stops the current audio and revokes
its object URL, pauses narration through the existing `pause_playback` command
before the command call — the click is the user asking to listen now, and a
cache miss can take seconds — then sets the new URL, pauses again right before
`play()` (a cache hit skips the synthesis wait, and narration may have started
in between), and never resumes. `ended` and `error` clear the playing state;
closing the editor or the modal stops the element and revokes the URL, and a
response that arrives after the close is dropped by the request sequence
counter.

The command rejects input longer than 200 Unicode code points with
`preview.text_too_long` before the pipeline or the engine runs: the controls
only ever send one word or one spoken form, and the command is reachable
directly, so a pasted paragraph would keep the engine busy for seconds. The
temporary WAV is owned by an RAII guard that deletes it on every exit path
(success, engine error, read failure, aborted task); a startup sweep removes
files older than ten minutes left behind by a crash. A failed read of the WAV
surfaces as `preview.audio_read_failed` with the IO error instead of a generic
message.

The in-memory cache is keyed by
`(engine, voice, mode, request text, dictionary revision)`. The issue names
`(engine, voice, spoken_text)`, but the response's `spoken_text` is only known
after the round trip — using it as the key would require a second synthesis to
look it up. The request inputs plus the dictionary state determine the audio
deterministically, and any dictionary write (save or import) bumps the revision
and clears the map, so a cached «Слово» reading cannot outlive a dictionary
write. The cache is bounded to 12 entries with LRU eviction: every entry holds
a whole WAV and the editor only replays a handful of words before a save
retires it. Rejected: caching by `spoken_text` after the response only (does
not satisfy "a second click does not synthesize"); no cache invalidation on
save (a stale reading after the user just added the entry).

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
renders one short line under the fields with the localized `formatError`
detail. Every failure goes through the shared catalog: the `preview.*` codes
(`preview.engine_busy`, `preview.invalid_stress`, `preview.invalid_mode`,
`preview.empty_text`, `preview.text_too_long`) and the engine site codes the
catalog knows (`voice_not_installed`, `bundle_not_installed`, `native.*`,
`teratts_cancelled`) have short messages; anything else falls back to the
backend `message`. The command's own validation failures serialize as
`type = "internal"`, engine failures as `type = "synthesis_error"` with the
engine's site code kept. A notification would steal attention from a form the
user is still filling and could hide the entered values behind a modal stack.
