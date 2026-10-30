# Proposal: add-teratts-engine

## Why

The two shipped Russian engines sit at uncomfortable ends of the licensing and
quality trade-off: Silero v5 (both the `ttsd` sidecar and the in-process
`silero-native`) is CC BY-NC-SA 4.0, so it cannot back a commercial build, and
Piper is MIT but capped at 22.05 kHz and sounds narrow. TeraTTSv2 — a
99M-parameter SupertonicTTS flow-matching model fine-tuned for Russian and
English — is the first engine that is both commercially usable (OpenRAIL-M with
a commercial attribution clause) and full-band 44.1 kHz, runs in-process on the
ONNX Runtime the repository already vendors, and needs no Python or PyTorch at
synthesis time.

It is also the first engine that exercises the per-engine text contract added in
`normalize-per-engine-blocks`: it requires language tags, brings its own Russian
accentuation (RUAccent dictionary mode), silently drops codepoints outside its
indexer, and cannot read digits or units — so the pipeline keeps ownership of
normalization and the adapter only wraps, accents and gates. Adding it now turns
that contract from a test-only promise into a shipped path and unblocks the
English-span routing shared with Supertonic 3 (#309).

## What Changes

- **New `teratts-engine` capability** — a versioned model bundle (manifest,
  sha256/size verification, explicit `format_version`/`bundle_version`/
  `upstream_revision`), four ONNX graphs driven in-process, the RUAccent
  dictionary frontend, the unsupported-codepoint gate, sentence chunking under
  the model's 25-second call limit, approximate word timestamps, voices,
  cancellation, typed errors, and licensing/attribution.
- **`EngineKind::TeraTts` end to end** — text profile, engine switcher,
  availability probe, config field `teratts_voice`, Settings engine option and
  voice list, bundle-download action and progress, generation-params snapshot.
- **One bundle download block** — the silero-native downloader is generalized
  (manifest-driven, per-file sha256, resume, progress events) and reused for
  TeraTTS instead of copied a third time.
- **The first consumer of `Limit::Secs`** — the shared splitter stays
  codepoint-based; the engine derives a conservative codepoint budget from the
  profile's seconds limit and validates every chunk with its duration predictor,
  re-splitting an over-budget chunk on a sentence boundary.
- **Documentation and licensing** — README (RU + EN) engine list and license
  section, the mandatory TeraTTSv2 attribution, `teratts/NOTICE`, CHANGELOG
  entry, crate README and engine page.

## Non-goals

- **English-span routing** (mixed RU/EN audio, per-language voices, the join
  with a pause and continuous highlighting). It is delivered as a separate
  change, `route-latin-spans-to-engines`, in the same issue #308, because it
  changes the `TtsEngine` contract for every engine and must be validated by
  the listening probe #311.
- **Making TeraTTSv2 the default engine** — a separate decision after
  listening on real articles.
- The teacher sampler, RUAccent full mode (483 MB of neural models), and voice
  cloning.
- **Model lifecycle** — update checks, in-app updates and deletion of downloaded
  bundles and voices are issue #317; this change only ships the versioned
  bundle format that issue builds on.
- Supertonic 3 (#309) and any shared flow-matching crate; the inference core is
  structured to be extracted later, not extracted now.
- Any behavior change for Silero, `ttsd` or Piper: their output stays
  byte-identical on the golden corpus.

## Capabilities

### New Capabilities

- `teratts-engine`: the TeraTTSv2 engine — bundle format and verification,
  inference, text frontend, chunking, synthesis output, timestamps, voices,
  cancellation, errors, licensing.

### Modified Capabilities

- `ipc-commands`: `AvailableEngines` gains `teratts`; a
  `download_teratts_bundle` command; bundle-download events carry
  `engine: "teratts"`; `UIConfig` gains `teratts_voice`; the generation
  snapshot accepts the `teratts` engine and its model identity.
- `ui`: the Settings engine selector gains a fourth option, the TeraTTS voice
  list and a bundle-download action with progress.
- `storage`: the config schema gains `teratts_voice` with a default; the
  history schema's generation snapshot accepts the `teratts` engine.
- `text-chunking`: the seconds limit stops being unexercised data — the first
  engine that declares it honors it with a measured codepoint budget and a
  duration-estimate validation.
- `word-highlight`: approximate word timestamps for an engine without native
  alignment — exact sentence boundaries, monotonic word intervals, a documented
  lag.

## Impact

- New crate `teratts/` at the repository root (library, path-dep of
  `src-tauri`), plus wiring in `flake.nix`, `justfile`, `lefthook.yml` and the
  CI workflows; `teratts/Cargo.lock` and `src-tauri/Cargo.lock`.
- `src-tauri/src/tts/{engine,profiles,switcher,availability,mod}.rs`, a new
  `src-tauri/src/tts/teratts/` adapter, a shared
  `src-tauri/src/tts/bundle_download.rs`, `state.rs`, `lib.rs`,
  `commands/mod.rs`, `storage/{schema,service}.rs`.
- `src/lib/{tauri,engineSelection,bundlePrompt,bundleDownload}.ts`,
  `src/dialogs/{Settings,SileroBundlePrompt,GenerationParamsDialog}.tsx`,
  `src/components/AppShell.tsx`, both i18n catalogs, and their tests.
- `README.md` + `README.en.md`, `CHANGELOG.md`, `docs/index.md`,
  `docs/development.md`, `teratts/README.md`, `teratts/NOTICE`.
- An external artifact: the `teratts-models-v1` GitHub Release (~410 MB). It is
  not part of the source tree and is downloaded on demand; a 404 or an offline
  machine leaves the engine unavailable with a typed reason, never a broken
  install.
- No new runtime dependency for the existing engines; the `ort`/`ort-sys` pins
  stay exactly as they are.
