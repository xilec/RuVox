# Design: add-teratts-engine

## Context

See `proposal.md` — Why. The state that shapes the approach:

- `normalize-per-engine-blocks` (archived) already ships `TextProfile`, the
  fragment route, span annotations, `process_for_engine`, and one shared
  splitter in `tts-contract`. All shipping profiles declare `Chars` limits and
  `Markup::None`; the Latin-native profile exists only in tests.
- `TtsEngine::synthesize(text, voice, sample_rate, out_wav, char_mapping)` is
  the only synthesis entry point; `EngineKind` has three variants serialized
  `snake_case` into history and generation snapshots.
- `silero-native` is a separate crate path-depended on by `src-tauri`, with its
  own bundle manifest, loader, and downloader; Piper is a module inside
  `src-tauri`.
- The Opus encoder (`src-tauri/src/audio/mod.rs`) resamples an off-list WAV
  rate to the nearest libopus-native rate; Piper's 22 050 Hz already takes that
  path.
- The TeraTTSv2 upstream release is a Hugging Face repository
  (`TeraSpace/TeraTTSv2`, revision `75983ac…`) with four ONNX graphs, a 65 536
  entry `unicode_indexer.json`, `.npy` voice styles, and the RUAccent dictionary
  files. Its reference Python wrapper (`teratts.py`) is the behavioral oracle.

## Goals / Non-Goals

**Goals:**

- A fourth engine that is commercially usable, full-band, in-process, and
  independent of Python at synthesis time.
- The first shipped consumer of the per-engine text contract: tags, RUAccent
  dictionary stress, charset gate, `Secs` limit — all engine-side, no duplicated
  normalization.
- A bundle format that is versioned and traceable to an upstream revision, so
  the lifecycle work (#317) can build updates on it without a format change.
- Zero behavior change for Silero, `ttsd` and Piper.

**Non-Goals:**

- English-span routing, per-language voices and audio joins — the follow-up
  change `route-latin-spans-to-engines`.
- Making TeraTTSv2 the default engine.
- Teacher sampler, RUAccent full mode, voice cloning.
- Update checks, deletion, and any lifecycle UI (#317).
- Extracting a shared flow-matching crate for #309.

## Decisions

### D1. A separate `teratts` crate; the flow core is an internal module

The engine goes into a new `teratts/` crate (mirroring `silero-native/`) that
`src-tauri` path-depends on. Inference is split into `bundle`, `indexer`,
`styles`, `text`, `chunk`, `synth`, `timestamps`, `engine`; the reusable
flow-matching part (`synth` + `timestamps`) is kept free of `EngineKind`,
Tauri and UI types so #309 can extract or adapt it later.

- *Alternative — a module under `src-tauri/src/tts/teratts/`:* rejected. The
  ONNX sessions, the `.npy`/gzip parsers and the parity tests would live in the
  application crate, and the crate-level test command (`cargo test
  --manifest-path teratts/Cargo.toml`) would disappear. The previous engine
  (`silero-native`) established the crate boundary for exactly this reason.
- *Alternative — a shared `flow-matching-onnx` crate now:* rejected as
  premature. The two models share an architecture but not file names, style
  formats, tag syntax or frontends; extracting before the second consumer
  exists would guess the seam. #309 can move the module out when it actually
  needs it.

### D2. A versioned bundle manifest, pinned to one release tag for now

`manifest.json` carries `format_version`, `bundle_version`, `model_id`,
`model_version`, `upstream_repo`, `upstream_revision`, `opset`, `sample_rate`,
`created_at_utc`, `files[]` (`path`, `asset`, `size`, `sha256`) and
`voices[]` (`id`, `language`). The loader verifies size and sha256 for every
file before opening a session and rejects an unsupported `format_version`.

GitHub release assets cannot contain `/`, so every manifest file carries both
the bundle-relative `path` and the flat release-asset `asset` name
(`models__text_encoder.onnx`). The downloader builds the URL from `asset` and
writes to `path`.

The app pins `teratts-models-v1` in a `BUNDLE_RELEASE_BASE_URL` constant, like
`silero-native` pins its tag. Version discovery, update and deletion are #317;
this change only guarantees that the format makes them possible without
re-downloading unchanged files.

- *Alternative — reuse the silero-native manifest schema:* rejected. It has no
  version, upstream revision or asset mapping, and the two bundles have
  different invariants; sharing the type would force flags into both.
- *Alternative — a single archive asset (`.tar.zst`):* rejected for now.
  Per-file resume and idempotent re-runs are simpler and match the existing
  downloader; an archive would need extraction and a second integrity layer.

### D3. The text frontend is an adapter: wrap, accent, gate — never normalize

The profile declares `languages: ["ru"]`, `route: Narrate/Narrate`,
`digits: Expanded`, `units: Expanded`, `stress: Manual`,
`charset: ModelAlphabet`, `markup: RuEnTags`, `chunk: Secs(20)` with pauses.
The adapter:

1. wraps the whole text in `<ru>…</ru>` (the model requires tags even for
   Russian-only input) and rejects nested or unknown tags;
2. applies RUAccent dictionary mode to each `<ru>` span, preserving words that
   already carry a `+` and the source casing;
3. applies NFKD and drops codepoints absent from `unicode_indexer.json` with
   one counted `tracing::warn` listing them (never silently, never a failure
   for ordinary punctuation; a surviving digit is a normalization gap and is
   reported as such);
4. hands the text to the indexer — no `num2words`, no punctuation rewriting, no
   symbol substitution, no transliteration.

- *Alternative — let the model's own `normalize_text` run:* rejected. It drops
  `— % № °`, loses numbers at sentence ends, and runs `num2words` a second time
  on text the pipeline already expanded.
- *Alternative — fail on any unsupported codepoint:* rejected for shipping
  text: `«»`, `…` and similar punctuation are legitimate pipeline output, and a
  hard failure would make the engine unusable on ordinary articles. A counted
  warning keeps the information loss observable.

### D4. The seconds limit is honored by the engine, not by the splitter

`tts-contract`'s splitter stays codepoint-based. The engine converts the
profile's `Secs(20)` into a conservative codepoint budget (a calibrated
chars-per-second constant, measured on the golden corpus), calls
`split_with_limit`, then runs the duration predictor on every chunk; a chunk
predicted above the hard 25 s call limit is re-split at the last sentence
boundary (clause, then whitespace, then hard split) and re-checked. The
predictor call is ~1 ms, so the validation is cheap.

- *Alternative — pass a measuring closure into `tts-contract`:* rejected. It
  would couple the shared crate to an inference call (or add a generic
  parameter) for one engine, while the splitter's contract ("split by a
  codepoint window") stays intact.
- *Alternative — trust a fixed chars-per-second constant without validation:*
  rejected. Rate varies with punctuation, digits and voice; a single outlier
  would silently truncate a chunk at 25 s, which the research measured as
  producing gibberish.

### D5. Timestamps: exact sentence boundaries, prefix-predictor words

Each chunk is a sentence-bounded unit, so its audio range is exact. Inside a
chunk, the engine runs `duration_predictor` on cumulative word prefixes
(punctuation preserved), scales the predicted boundaries to the actual chunk
duration, and shifts them past leading silence (the first sample above a
threshold). Timestamps are monotonic and non-overlapping, and the last `end` is
at most the chunk's audio end. Word ranges are in normalized-text coordinates;
the app maps them to the original document through `char_mapping`.

The approximation is documented in the capability spec: pauses are attributed
to the preceding word, so word boundaries lag an ASR reference by ~0.2–0.4 s.
Sentence-level highlighting stays exact.

- *Alternative — a forced aligner (WhisperX/CTC):* rejected for the first
  version: it adds a second model and 0.1–0.3 RTF for a reading aid where
  approximate word highlighting is acceptable.

### D6. 44.1 kHz WAV, no in-engine resampling

The engine writes 44 100 Hz mono PCM16 WAV and returns the actual duration; the
shared Opus encoder resamples 44.1 → 48 kHz on its existing off-list path, the
same way it handles Piper's 22 050 Hz. The `sample_rate` config value is
ignored by the engine (Piper already does this).

- *Alternative — resample to the configured rate inside the engine:* rejected
  for now: it duplicates a shared audio concern, and the encoder must resample
  anyway for any non-native rate. If listening finds the linear resampler
  audible, a better shared resampler is a separate change.

### D7. One bundle downloader, generalized from silero-native

`src-tauri/src/tts/bundle_download.rs` becomes a manifest-driven downloader:
list of `(path, asset, size, sha256)`, `.partial` streaming, sha256 while
streaming, skip-if-valid, progress events throttled to 256 KB, typed failures.
`silero_native::download` is rewritten on top of it (behavior-preserving), and
`teratts::download` supplies the TeraTTS manifest and release base URL. The
`bundle_download_*` payloads keep their shape and carry `engine: "teratts"`.

- *Alternative — copy the silero-native downloader:* rejected. This is the third
  download block (Piper voices, Silero Native, TeraTTS) and #317 will extend
  all of them with update/delete; one implementation is the only way that stays
  consistent.

### D8. `EngineKind::TeraTts`, `teratts_voice`, and a stat-only probe

`EngineKind::TeraTts` serializes as `"teratts"`; the profile registry, the
switcher's atomic kind, `parse_kind`, the availability probe and the frontend
union all gain it. The config gains `teratts_voice` (default `ru_m5`) so
flipping engines preserves each engine's voice, like `piper_voice` and
`speaker`.

The availability probe is stat-only: parse the manifest, check
`format_version`, and require every listed file to exist with the recorded size
(no hashing — that runs in the downloader and the loader). A missing or
incompatible bundle yields `teratts.bundle_missing` /
`teratts.bundle_incompatible` as a machine-readable reason.

- *Alternative — reuse `speaker` for the TeraTTS voice:* rejected. The value
  spaces do not overlap (`ru_m5` vs `xenia`), and a shared field would make an
  engine flip silently hand an invalid voice to one side.
- *Alternative — hash the bundle on every Settings open:* rejected. 410 MB of
  hashing on the UI path is unacceptable; size + manifest parsing is the
  established silero-native contract.

### D9. Parity is checked against Python fixtures, not against seeded audio

The production engine uses its own PRNG for the initial latent (numpy's
`default_rng` is not reproduced), so a full-audio comparison against the Python
reference would compare different noise. Parity is therefore split:

- `text_emb` and the predicted duration are compared to Python fixtures with a
  tight tolerance (deterministic given the same text/style);
- the sampler and vocoder are compared by injecting a Python-generated latent
  fixture through a test-only entry point, with an SNR/MSE tolerance;
- full-pipeline golden tests assert invariants (duration, sample rate,
  non-silence, no clipping, monotonic timestamps), not hashes, because ONNX
  Runtime kernels may vary across versions and platforms.

- *Alternative — reimplement numpy's PCG64/ziggurat:* rejected. It is a large,
  test-only dependency on numpy internals for no production benefit.
- *Alternative — hash golden WAVs:* rejected. A hash would fail on an ORT
  version bump without a real regression, producing noise instead of a signal.

### D10. Licensing is a shipped artifact, not a README note

The bundle ships `LICENSE`, `LICENSE-OPENRAIL` and `RUACCENT_NOTICE.txt`; the
repository ships `teratts/NOTICE`; `README.md` and `README.en.md` carry the
mandatory commercial attribution ("Text-to-speech powered by TeraTTSv2 by
TeraSpace", with a link) and an accurate license section (the default engine
stays non-commercial; TeraTTSv2 is the commercially usable option, with
OpenRAIL-M use-based restrictions). Modifications (fine-tuning provenance,
bundle re-hosting) are identified in `teratts/NOTICE`.

- *Alternative — put the attribution only in the About dialog:* rejected. The
  license clause asks for the product's public description; the READMEs are the
  public description, and the About dialog is a second, cheap place.

## Risks

| Risk | Mitigation |
|---|---|
| ORT 1.20.1 (CI) cannot load the graphs | The model is opset 19; add a smoke test that opens all four sessions on the CI ORT version; if it fails, bump the ORT pin in a separate PR |
| ~390 MB of weights plus activations raise RSS | Measure peak RSS in the perf smoke; keep chunks ≤ 20 s; drop sessions on shutdown; one engine instance |
| Clicks or truncation at chunk joins | 150–250 ms silence, sentence boundaries, duration-predictor validation, listening pass, spectral check of joins |
| RUAccent dictionary mode misses unknown words and homographs (`замок`) | Documented limitation; manual `+` from the user dictionary always wins; full mode is a separate decision |
| 44.1 → 48 linear resample is audible | Measure the spectrum; if needed, a better shared resampler is a follow-up change |
| A 410 MB download is interrupted | `.partial` + resume + sha256 + per-file skip; interruption test; CDN retries |
| The generic downloader refactor regresses Silero Native | Behavior-preserving tests around the shared downloader, and the silero-native refactor is its own commit |
| Test environment (ld.so, ORT dylib, long worktree paths) | Rust tests in the docker builder; `ORT_DYLIB_PATH` for host runs; `CARGO_TARGET_DIR` for the worktree |

## Migration

- No existing behavior changes: the three shipping profiles, the pipeline output
  for them, and their golden fixtures stay byte-identical.
- Old configs parse unchanged: `teratts_voice` defaults to `ru_m5`; an unknown
  persisted engine falls back through the existing coercion path.
- Old history entries parse unchanged; `engine: "teratts"` appears only on new
  generations.
- The new bundle is an external download; a machine without it keeps the
  current engine and sees TeraTTS as unavailable with a typed reason.
