## Why

The dictionary editor asks for the spoken form («Как произносить») but gives no
way to hear it, so authoring an entry is guesswork: whether the stress landed on
the right syllable is discovered only after saving the entry, pasting a text and
letting it narrate. The same editor also never answers the question that decides
whether an entry is needed at all — how the source word reads *today* with the
current pipeline and the saved dictionary. The editor is the moment the user
knows exactly which word and which reading they mean, so it is the place to
listen.

The pieces already exist: `preview_normalize` runs the pipeline without side
effects, `tts_normalize::dictionary::spoken_form` validates and strips the
stress mark, and every engine implements `TtsEngine::synthesize`. Only the
preview command and the editor controls are missing.

## What Changes

- A new `preview_pronunciation(text, mode)` IPC command returns
  `{ spoken_text, wav_base64, duration_sec }` for one word. `mode = "normalize"`
  speaks the current reading of the token (the same pipeline run
  `preview_normalize` performs, with the saved dictionary applied and the entry
  being added absent); `mode = "spoken"` speaks the typed spoken form with the
  `+` mark kept only for engines whose text profile supports stress. The
  preview uses the active engine only — it never switches engines, loads a
  second model, or writes an entry, timestamps, queue state or synthesis
  events. Audio is synthesized into a temporary file under the app cache
  directory, read into memory and deleted immediately.
- The command is serialized against the queue: a preview refuses to start while
  a queue entry is inside `tts.synthesize` (`preview.engine_busy`), so a
  one-word preview never queues behind a minutes-long narration or shares
  TeraTTS's single cancellation flag with it.
- `src-tauri/tauri.conf.json` allows the webview to play the returned WAV
  (`media-src 'self' blob:`).
- The add/edit form in `DictionaryModal` gains a play/stop control at the right
  edge of both «Слово» and «Как произносить», next to the help icon: disabled
  while the field is empty or invalid, a spinner while synthesizing (both
  controls blocked), a stop square while playing, and one `<audio>` element per
  dialog with the previous object URL revoked. Starting a preview pauses a
  playing narration (pause, never stop, and no automatic resume afterwards).
  Repeated previews of identical text are served from an in-memory cache keyed
  by engine, voice and the request, so a second click does not re-synthesize.
  A failing preview shows one short inline message inside the form and keeps
  the entered values; closing the editor stops the preview and releases the
  audio resource. When the selected scope belongs to an engine other than the
  active one, the form states which engine is speaking.
- RU and EN catalogs get the tooltips, aria-labels, the engine hint and the
  failure line.
- The behavior is specified as deltas of `user-dictionary` (editor preview) and
  `ipc-commands` (the new command).

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `user-dictionary`: a new "Editor pronunciation preview" requirement defines
  the two play controls, their disabled/spinner/stop states, the active-engine
  rule and hint, the narration pause, the cache, the inline failure and the
  close-time cleanup.
- `ipc-commands`: a new "Pronunciation Preview Command" requirement defines
  `preview_pronunciation`, both modes, the response shape, the temp-file
  lifecycle, the no-side-effects contract and the busy guard.

## Impact

- Affected code: `src-tauri/src/commands/mod.rs` (command),
  `src-tauri/src/commands/synthesis.rs` (`voice_for_engine` visibility),
  `src-tauri/src/state.rs` (preview cache dir + preview guard),
  `src-tauri/src/lib.rs` (registration + state wiring),
  `src-tauri/src/test_support.rs` (test state),
  `src-tauri/Cargo.toml` (`base64`), `src-tauri/tauri.conf.json` (CSP),
  `src/lib/tauri.ts`, `src/dialogs/DictionaryModal.tsx`,
  `src/lib/engineSelection.ts` (voice for the active engine),
  `src/i18n/{ru,en}.ts`, tests, `CHANGELOG.md`.
- No storage, queue or synthesis-pipeline change: the preview never enters the
  entry lifecycle.
- User-visible: an entry's pronunciation can be heard before saving, and the
  current reading of a word can be checked without pasting it into a document.

## Non-goals

- Play controls in the entry table (same command, natural follow-up).
- Previewing the token inside a sentence: a reading can depend on the route the
  token takes, while the preview normalizes it standing alone.
- Previewing `forms` separately — every form shares the entry's `to`.
- Showing the normalized text next to the audio, or any voice/prosody choice
  inside the preview.
- Playing the preview through the mpv player or the entry/queue lifecycle: the
  preview is ephemeral by contract.
