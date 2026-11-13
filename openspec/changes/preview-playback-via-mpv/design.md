## Context

The dictionary editor's pronunciation preview (issue #349) synthesizes a WAV in
the backend and returns it as base64; the dialog plays it through one hidden
`<audio>` element. That element is the only HTML media in the app and a second
audio stack beside mpv, which RuVox already bundles, resolves and drives through
`tauri-plugin-mpv` for narration.

WebKitGTK 2.54 aborts the web process when its GStreamer backend cannot create
an audio sink: `MediaPlayerPrivateGStreamer::createAudioSink()` holds
`RELEASE_ASSERT(audioSink)`, and `autoaudiosink` lives in `gst-plugins-good`,
which the nixpkgs webkitgtk does not pull in. A missing plugin therefore kills
the window on the first preview click, and no JavaScript can catch the abort. A
first fix exported the plugin path in the dev shell and the nix package wrapper;
it was reverted in favour of removing the webview media path entirely, because
the environment fix only patches the distributions we control and leaves the
second audio stack in place.

`tauri-plugin-mpv` keys its instances by window label, so a second instance is
an independent mpv process; `Mpv::destroy(label)` kills it and removes it from
the plugin's map. The narration player keeps the `main` label.

## Goals / Non-Goals

**Goals:**

- Play previews through the same mpv stack as narration, in an instance of its
  own.
- Keep the existing synthesis contract, in-memory cache, engine serialization,
  error catalog and UI unchanged.
- Never touch the narration player's loaded entry, position or events.
- Leave no mpv process and no WAV behind on stop, editor close or app exit.

**Non-Goals:**

- Reworking `preview_pronunciation`, the cache key/eviction rules, or the
  engine lock.
- A disk-backed preview cache or a token/path IPC contract.
- Reusing the narration player or adding playback-state events for previews.
- Changing the preview UI (labels, placement, spinner, inline errors).

## Decisions

**A dedicated mpv instance with the plugin label `preview`, created per
playback and destroyed on stop.** The dialog already pauses narration before a
preview, but it must not take over the narration player: `Player::load` would
replace the loaded entry, its position and the `playback_*` events that drive
the queue UI, and restoring the narration afterwards would require remembering
and reloading the file. A separate instance has none of that state — it loads
one WAV, plays it and dies. Keeping it alive between previews was rejected: an
idle mpv process costs memory for a control used in short bursts, while the
spawn costs roughly 100-150 ms once the audio is already synthesized. Rejected:
a sentinel entry id inside the narration player (corrupts `current_entry_id`
and the UI), and a second long-lived instance (same memory argument).

The instance is configured audio-only (`--no-video`, `--no-ytdl`, `--idle=yes`)
and uses the same `resolve_mpv_path` as the player, so the bundled AppImage /
NSIS mpv and the PATH fallback behave identically. `--idle=yes` keeps the
process alive after the WAV ends, so the stop path always kills a live process
instead of racing a process that already exited. The plugin's map entry is
removed by `destroy` on every path; `Drop` and the app's `RunEvent::Exit`
destroy it as a backstop.

**The WAV still travels as base64; only playback moved.** `preview_play`
decodes the payload, writes it to `<app cache>/preview/<uuid>.wav` and starts
mpv. This keeps `preview_pronunciation` and the dialog's in-memory cache (and
its "no re-synthesis on repeat" guarantee) exactly as specified. A
token/path-based contract was rejected: it would force the frontend's LRU
eviction and dictionary-revision invalidation to reach into backend files, add
an "expired" error path for a swept or replaced file, and make a cache hit a
disk-existence check. A backend-owned cache keyed by request text was rejected
too: the key includes the dictionary revision, which lives on the frontend, so
invalidation would need another command and a second source of truth. The
extra round trip carries at most a few hundred kilobytes over local IPC.

**Playback end is a dialog timer, not a backend event.** The dialog knows
`duration_sec` from the synthesis response; it stops the preview after that
duration plus a 500 ms margin, and the stop command destroys mpv and deletes
the file. Polling `eof-reached` in a spawned task and emitting a
`preview_finished` event was rejected: it adds a per-preview async task, a
second event contract and destroy-vs-poll races inside the plugin's instance
map, for a control whose playing state is purely local. The margin covers mpv's
startup and audio-device latency; a second click always stops immediately.
Because `--idle=yes` keeps mpv alive after EOF, a throttled webview timer can
leave an idle process until the next stop, editor close or app exit — bounded,
and never audible.

**A `PreviewAudioBackend` trait in `AppState`, mirroring `PlayerBackend`.** The
trait exposes `play(&[u8])` and `stop()`; production holds the mpv
implementation, tests hold a fake that records payloads and stop calls. The
command layer (`base64` decode, size cap, typed errors) is then testable without
mpv, a display or a real process — the same seam the player tests already use.
File writing and mpv control stay in the production implementation, which is
thin and exercised manually.

**Errors are typed and non-fatal.** Malformed base64 answers
`preview.invalid_audio`; a decoded payload above 8 MiB answers
`preview.audio_too_large` (the command is reachable directly, and a 200-word
preview is under 2 MB); an mpv failure answers `preview.play_failed` with the
plugin message, and a failed destroy answers `preview.stop_failed` while the
WAV is still deleted. The dialog keeps showing its single inline
`dictionary.preview.error.play_failed` line for playback failures, as today.

**The CSP loses `media-src 'self' blob:`.** Nothing in the webview decodes
audio or video any more; `default-src 'self'` covers the remaining needs and the
`img-src ... blob:` entry stays for pasted images.

## Risks / Trade-offs

- [mpv spawn latency per preview] → the instance is created only after
  synthesis finishes, when the user is already waiting; measured startup is
  ~100-150 ms.
- [A second mpv process while a preview plays] → one process per active
  preview, destroyed on stop/close/exit; no idle instance is kept.
- [Webview timer throttling delays the automatic stop] → mpv stays idle after
  EOF (no sound), and the next preview, editor close or app exit destroys it;
  the startup sweep deletes a leftover WAV after a crash.
- [Engine-reported `duration_sec` drives the auto-stop] → the 500 ms margin
  absorbs clock differences; a manual click stops instantly, and a second
  preview stops the first regardless of the timer.
- [Preview mpv missing from PATH in a dev run] → the same failure the
  narration player already has; the inline play-failed message appears.
- [The plugin's command dispatcher unwraps its instance map] → play and stop
  are serialized by one mutex, the `preview` label has no window (so the
  plugin's close handler never removes it), and every exit path removes the
  instance through `destroy`.
