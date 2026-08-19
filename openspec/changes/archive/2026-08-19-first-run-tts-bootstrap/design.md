# Design: first-run-tts-bootstrap

## 1. Voice selection by active engine (backend)

Current code (`src-tauri/src/commands/mod.rs`, `synthesize_audio`):

```rust
let voice = if config.engine == "piper" {
    config.piper_voice.clone()
} else {
    config.speaker.clone()
};
```

`config` here is the **persisted** config. The engine serving the request,
however, is whatever the `EngineSwitcher` currently holds — which differs
whenever `build_engine` fell back (silero_native without bundle, silero
without ttsd). `synthesize_audio` already receives `tts: &dyn TtsEngine`,
and the caller passes the `EngineSwitcher`, whose `kind()` mirrors the
active engine through an `AtomicU8` (no lock needed).

Fix:

```rust
let voice = match tts.kind() {
    EngineKind::Piper => config.piper_voice.clone(),
    _ => config.speaker.clone(),
};
```

The auto-download retry arm changes its gate the same way:
`code == "voice_not_installed" && tts.kind() == EngineKind::Piper`
(was `config.engine == "piper"`). With voice selection fixed, the retry now
targets a voice that actually exists in the Piper catalog.

This also matches the existing precedent: the synthesis-time input-length
guard one call frame up already keys on `tts.kind()`.

### Tests

`synthesize_audio` is a private async fn with injectable deps
(`&dyn TtsEngine`, `&StorageService`, emitter). Add a `#[cfg(test)]` fake
`TtsEngine` that records the `voice` argument it receives (and can fail with
`voice_not_installed` on demand), construct it as a minimal `EngineSwitcher`
(or pass the fake directly — the signature takes `&dyn TtsEngine`), and
assert:

- kind = Piper + persisted `engine = "silero_native"` → voice passed to the
  engine is `config.piper_voice`, not `config.speaker`;
- kind = Piper + `voice_not_installed` → `download_voice` path is attempted
  (observable via the recording emitter's `voice_download_started` event —
  use a real temp voices dir; the download itself fails offline, which is
  fine: we assert the attempt, i.e. that the gate keyed on the active
  engine);
- kind = SileroNative + persisted `engine = "piper"` → voice is
  `config.speaker` (reverse coercion must not happen).

## 2. First-run bundle prompt (frontend)

### Trigger

`AppShell` already loads `UIConfig` once on mount. Extend that effect: after
the config resolves, if `cfg.engine === 'silero_native'`, call
`commands.getAvailableEngines()`; if `availability.silero_native.available`
is false → open the prompt. The condition is exactly "default config, bundle
missing" — users who explicitly picked Piper (`engine === 'piper'`) never
see it, and once the bundle is on disk the probe flips to available and the
prompt stops appearing.

Extract the decision as a pure function in `src/lib/` (e.g.
`shouldOfferBundleDownload(config, availability)`) so it is unit-testable
without a Tauri shell, mirroring the `engineSelection.ts` / `addFlow.ts`
pattern.

### Dialog

New `src/dialogs/SileroBundlePrompt.tsx` (Mantine `Modal`), Russian UI copy:

- Title: «Скачать движок Silero?»
- Body: explains that the default engine «Silero (нативный)» needs a
  one-time ~230 MB model download, and that until then the app runs on the
  built-in Piper engine.
- Buttons: «Скачать (~230 МБ)» (primary), «Остаться на Piper» (secondary,
  closes for this run).

Accepting calls `commands.downloadSileroNativeBundle()` and switches the
modal body to a `Progress` bar driven by the `bundle_download_*` events —
the same subscription pattern as `Settings.tsx` (started → show bar,
progress → percent per file index, finished → terminal state). On
`ok: true` the dialog calls `commands.updateConfig({ engine:
'silero_native' })` so the `EngineSwitcher` rebuilds onto the native engine
immediately (the persisted value is already `silero_native`, so the write is
a no-op on disk and only the swap matters), shows a green confirmation and
closes. On `ok: false` it shows the error message and re-enables the
download button.

The prompt subscribes to the events only while open, same as Settings.

### Why not a notification with an action

Mantine notifications with buttons are easy to miss and cannot host a
progress bar; a modal matches the weight of a 230 MB decision and reuses the
dialog conventions already in the codebase.

## 3. Piper auto-download (spec only)

`notificationBridge.ts` already renders `voice_download_*` events as a
per-voice toast («Загрузка голоса X», progress, terminal green/red), and
`synthesize_audio` already retries once after a successful fetch. The change
only re-aims the gate (part 1) and pins the behavior in the
`ipc-commands` spec so a future refactor does not silently drop it.

## Alternatives considered

- **Persist the fallback** (write `engine = "piper"` when falling back):
  rejected earlier in the Settings coercion design — it would kill the
  silero_native default before the user ever downloads the bundle, and the
  prompt in part 2 depends on the persisted value staying `silero_native`.
- **Auto-switch the engine inside `download_silero_native_bundle`**:
  tempting, but the command is also used from Settings where the user may be
  downloading the bundle while intentionally staying on Piper. The prompt
  does the switch explicitly instead.
- **Auto-download the bundle at startup without asking**: 230 MB of
  unsolicited traffic on possibly metered connections — no.
