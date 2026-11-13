## Why

The pronunciation preview plays its WAV through an HTML `<audio>` element — the
only HTML media in the app, and a second audio stack beside the mpv player we
already bundle and control. On Linux, WebKitGTK hard-aborts the whole web
process when GStreamer has no audio sink plugin (`RELEASE_ASSERT` in
`createAudioSink`), so clicking either preview control kills the window instead
of returning an error. The app ships mpv on every platform and drives it for
narration; the preview has no reason to depend on the webview's media stack.

## What Changes

- Play previews in a dedicated audio-only mpv instance (plugin window label
  `preview`), separate from the narration player: created lazily on play,
  destroyed when playback is stopped or the editor closes. The narration
  player's loaded entry, position and events stay untouched.
- Add `preview_play(wav_base64)` and `preview_stop()` commands.
  `preview_pronunciation` keeps its synthesis contract (`spoken_text`,
  `wav_base64`, `duration_sec`).
- Replace the dialog's `<audio>` element, blob URLs and `URL.createObjectURL`
  with those commands; the existing in-memory preview cache stays.
- Return the control to its play state when the clip's duration elapses, on a
  second click, and when the editor closes; the backend destroys the player
  instance and deletes the temporary WAV in all three cases.
- Remove `media-src 'self' blob:` from the CSP: the webview no longer decodes
  media. The app's only remaining audio path is mpv.

## Non-goals

- Changing synthesis, the preview cache rules, engine serialization, the error
  catalog or the editor's UI.
- Reusing the narration player, or touching its state and events.
- Keeping preview audio on disk between plays (a path/token cache), or changing
  `preview_pronunciation`.
- Adding dependencies: mpv is already bundled and resolved for playback.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `user-dictionary`: the "Editor pronunciation preview" requirement now
  mandates playback through the application's player stack and defines what
  stop and close release.
- `ipc-commands`: new preview playback commands beside the existing
  `preview_pronunciation` contract.

## Impact

- Rust: new `src-tauri/src/player/preview.rs`; `player/mod.rs` (label,
  `resolve_mpv_path` visibility), `state.rs` (`preview_audio`), `commands/mod.rs`
  (commands), `lib.rs` (construction, exit), `test_support.rs` (fake).
- Frontend: `src/dialogs/DictionaryModal.tsx` and its tests,
  `src/lib/tauri.ts`.
- Config: `src-tauri/tauri.conf.json` (CSP).
- Specs: `user-dictionary`, `ipc-commands`; `CHANGELOG.md`.
