# Tasks: first-run-tts-bootstrap

## Backend: voice follows the active engine

- [x] `synthesize_audio`: select voice via `tts.kind()` (Piper →
      `piper_voice`, otherwise `speaker`)
- [x] Auto-download retry gate keyed on `tts.kind() == EngineKind::Piper`
      instead of `config.engine == "piper"`
- [x] Unit tests with a fake `TtsEngine`: Piper-active + silero_native-config
      uses `piper_voice`; reverse coercion never happens; retry gate fires on
      active-Piper even when persisted engine is silero_native

## Frontend: first-run bundle prompt

- [x] Pure helper `shouldOfferBundleDownload(config, availability)` in
      `src/lib/` + unit tests (engine piper → false; silero_native + bundle
      available → false; silero_native + bundle missing → true)
- [x] `src/dialogs/SileroBundlePrompt.tsx`: modal with download/decline
      actions, inline progress via `bundle_download_*` events, success →
      `updateConfig({ engine: 'silero_native' })` + green confirmation,
      failure → inline error + retry-enabled button
- [x] Wire into `AppShell` config-load effect (opens after config resolves
      and the probe says the bundle is missing)
- [x] `pnpm typecheck` + `pnpm test:unit` green

## OpenSpec

- [x] Delta specs: `ipc-commands` (synthesis voice selection by active
      engine; Piper auto-download retry), `ui` (first-run bundle prompt)
- [x] `openspec validate` green
- [x] Verify change vs implementation, then archive

## Gates

- [x] `just test` green (Rust + TS + Python)
- [x] `just lint` green
- [x] ruvox-reviewer pass over the branch diff
- [x] Manual VM pass is tracked in the epic plan (task F), not here
