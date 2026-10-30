# Tasks

## 1. Bundle and release

- [x] 1.1 Add `teratts/export/` — a uv-managed maintainer script that assembles
  a bundle from a pinned Hugging Face snapshot: copies the four ONNX graphs,
  `unicode_indexer.json`, the ten voice styles, the RUAccent dictionary files
  and the licenses; validates graph I/O shapes and the opset; writes
  `manifest.json` (`format_version`, `bundle_version`, `model_id`,
  `model_version`, `upstream_repo`, `upstream_revision`, `opset`, `files[]`,
  `voices[]`) with sha256 and size per file; prints the total size. Its README
  documents the update procedure (check the upstream revision, assemble,
  publish a new tag). Verify: running it on the local HF snapshot produces a
  bundle whose manifest hashes verify with `sha256sum -c`, and the total is
  ~410 MB with no teacher sampler and no `ruaccent/nn`.
- [x] 1.2 Publish the `teratts-models-v1` GitHub Release (flat asset names from
  the manifest `asset` field, `manifest.json`, licenses) and verify it through
  `gh api repos/xilec/RuVox/releases/tags/teratts-models-v1`; download one
  small file back and compare its sha256. Verify: every manifest file has a
  matching asset, and a fresh download of the whole bundle verifies.
- [x] 1.3 Create the model-lifecycle issue (#317) with the agreed scope and
  link it from #308. Verify: the issue exists with the `enhancement` and
  `area:tts` labels.

## 2. Crate skeleton and inference

- [x] 2.1 Create the `teratts` crate: `Cargo.toml` with the same
  `ort`/`ort-sys` pins as `src-tauri` (`=2.0.0-rc.12`, `load-dynamic` +
  `disable-linking`), `tts-contract` path dependency, `hound`, `flate2`,
  `serde`, `serde_json`, `sha2`, `thiserror`, `tracing`, `rand`, `regex`;
  `README.md` (what the crate is for, which command checks it);
  `error.rs` with the typed error enum. Verify: `cargo check` and
  `cargo test --manifest-path teratts/Cargo.toml` (empty suite) pass; `cargo
  fetch` resolves every new dependency.
- [x] 2.2 `bundle.rs`: parse and verify `manifest.json` — `format_version`
  support, per-file size and sha256, missing file, incompatible version;
  expose `model_id`, `bundle_version`, `voices`. Verify: unit tests for a valid
  bundle, a corrupted file, a truncated file, an unsupported format and a
  missing bundle; no ONNX session is opened on any failure path.
- [x] 2.3 `indexer.rs`: load the 65 536-entry table, encode a string into
  `text_ids`/`text_mask` after NFKD, and report codepoints outside the table.
  Verify: unit tests for Cyrillic, Latin, punctuation, a codepoint outside the
  table, and a string that produces no tokens.
- [x] 2.4 `styles.rs`: read `style_ttl.npy` and `style_dp.npy` with an in-crate
  `.npy` parser (magic, header, little-endian f32) and validate the shapes
  `(1, 50, 256)` and `(1, 8, 16)`. Verify: unit tests on the real bundle files
  plus a malformed header and a wrong shape.
- [x] 2.5 `synth.rs`: open the four sessions with the CPU execution provider and
  intra-op threads at physical-core scale; run the encoder, the duration
  predictor, the seeded 8-step sampler and the causal vocoder in 16-frame chunks
  with 20-frame context; trim to the predicted duration and write PCM16 WAV at
  44 100 Hz. Verify: bundle-gated test asserts the WAV header, the duration
  within one vocoder frame, non-silence and no clipping; a test-only entry point
  accepts an injected latent.
- [x] 2.6 `text.rs`: tag wrapping and validation, RUAccent dictionary mode
  (four gzip dictionaries, case-preserving, `+`-preserving, `ё`-words), NFKD,
  and the counted unsupported-codepoint gate. Verify: unit tests for a known
  accented word, an explicit `+`, an unknown word, a `ё` word, a digit that
  survives normalization, and nested/stray tags.
- [x] 2.7 `chunk.rs`: split the text with the shared splitter using a
  calibrated codepoint budget for `Secs(20)`, validate every chunk with the
  duration predictor, and re-split an over-budget chunk on the last sentence
  boundary. Verify: unit tests for a long multi-sentence text, one sentence
  longer than the limit, a paragraph break, and an empty text.
- [x] 2.8 `timestamps.rs`: exact per-chunk ranges, cumulative-prefix duration
  prediction inside a sentence, scaling to the actual chunk duration, leading
  silence correction, and mapping into normalized-text coordinates. Verify:
  invariants (monotonic, non-overlapping, `end <= duration`, every word inside
  its chunk) over a multi-chunk text, plus a golden fixture.
- [x] 2.9 `engine.rs`: the public API — load a bundle, synthesize a text with a
  voice and a profile, return WAV bytes, timestamps and duration; expose model
  info and the voice catalogue; cancellation checked between chunks. Verify:
  bundle-gated end-to-end test on a two-paragraph text, plus a cancellation test
  that leaves no output file.
- [x] 2.10 Parity and perf: compare `text_emb` and the predicted duration to
  Python fixtures, compare sampler+vocoder against an injected Python latent,
  and add a release-mode perf smoke with an RTF budget. Verify:
  `cargo test --manifest-path teratts/Cargo.toml` with
  `TERATTS_BUNDLE=tmp/teratts-bundle` passes; RTF ≤ 0.15 on ≥ 4 threads and the
  peak RSS is recorded.

## 3. Application integration

- [x] 3.1 Add `TERATTS_PROFILE` (`languages: ["ru"]`, `Narrate`/`Narrate`,
  `Expanded` digits and units, `Manual` stress, `ModelAlphabet`,
  `RuEnTags`, `Secs(20)`, paragraph 250 ms / segment 200 ms) and register it in
  `profile_for`; add `EngineKind::TeraTts` (`ALL`, `as_str`, serde
  `"teratts"`). Verify: the registry test covers every variant and the profile
  passes `validate()`.
- [x] 3.2 `src-tauri/src/tts/teratts/`: the `TtsEngine` adapter — load through
  the crate, run synthesis on a blocking thread, write the WAV, map timestamps
  through `char_mapping`, translate errors to the Russian user-facing codes
  (`bundle_not_installed`, `bundle_corrupt`, `bundle_incompatible`,
  `voice_unknown`, `text_invalid_tags`, `synthesis_failed`,
  `teratts_cancelled`). Verify: adapter tests with a fake bundle and a
  cancellation test.
- [x] 3.3 Generalize the bundle downloader into
  `src-tauri/src/tts/bundle_download.rs` and rewrite the silero-native
  downloader on top of it; add the TeraTTS downloader and the
  `download_teratts_bundle` command with the `bundle_download_*` events
  (`engine: "teratts"`). Verify: existing silero-native downloader tests stay
  green unchanged; new tests cover resume, checksum failure and idempotent
  re-run.
- [x] 3.4 Switcher and startup: `kind_to_u8`/`u8_to_kind`/`parse_kind`, the
  TeraTTS factory, `apply_config`, warmup/shutdown; `data_dir/teratts` in
  `AppState`; the startup fallback when the persisted engine is `teratts` and
  its bundle is missing. Verify: switcher tests for a switch to and from
  TeraTTS and for the missing-bundle fallback.
- [x] 3.5 Availability: add `teratts` to `AvailableEngines` and a stat-only
  probe with `teratts.bundle_missing` / `teratts.bundle_incompatible` reasons.
  Verify: probe tests for present, missing, truncated and incompatible bundles.
- [x] 3.6 Config and voice: `teratts_voice` (default `ru_m5`) in the schema,
  defaults, service, `voice_for_engine`, and the generation-params snapshot
  (engine `"teratts"`, model identity). Verify: storage round-trip tests, a
  snapshot test, and a legacy-config parse test.
- [x] 3.7 Frontend: `EngineKind`/`AvailableEngines`/`UIConfig` types, the
  Settings engine option and RU voice list, the download action with progress,
  the availability reason, `engineSelection` coercion/fallback,
  `GenerationParamsDialog`, `QueueList`, both i18n catalogs. Verify:
  `pnpm typecheck` and `pnpm test:unit` green, including new tests for the
  fourth engine and the bundle prompt path.
- [x] 3.8 Docs and licensing: `teratts/README.md`, `teratts/NOTICE`,
  `README.md` + regenerated `README.en.md` (engine list, license section,
  attribution), `CHANGELOG.md` `[Unreleased]`, `docs/index.md`,
  `docs/development.md`, `justfile` `test-teratts`, the CI rust job and
  rust-cache workspaces. Verify: `just lint` green; the README pair differs only
  by language; the attribution string is present in both.

## 4. Validation

- [x] 4.1 Run the full gates: `nix develop -c just lint`, the Rust suites
  (`src-tauri`, `teratts`, `tts-contract`) in the docker builder, `pnpm
  typecheck`, `pnpm test:unit`, and the bundle-gated teratts suite with
  `TERATTS_BUNDLE`. Verify: all green; the perf smoke stays inside the RTF and
  RSS budgets.
- [x] 4.2 Manual listening pass (the agent runs the app or the example and hands
  the user a checklist): a paragraph with numbers, abbreviations, a URL and a
  code identifier; a three-minute text with chunk joins; `ru_m5` vs `ru_f1`; a
  dictionary word with `+`. Verify: the user accepts the sound or names the
  defect; fixes land on the same branch. Accepted 2026-10-30 on `ru_m5`,
  `ru_f1`, the mixed paragraph, stress words and the 4-minute text; the
  `IPv6` reading is deferred to the EN-routing work (#311).
- [ ] 4.3 Reviewer pass (`ruvox-reviewer`) over the branch diff, fix loop, then
  archive the change and sync the delta specs. Verify: the reviewer's accepted
  findings are committed, the archive commit contains the synced specs, and
  `openspec validate --all` passes.

## Manual test

The engine must be listened to before the branch is merged: the automated gates
cannot hear clicks, wrong stress or a truncated chunk. The agent starts the app
(or the `teratts/examples/synth.rs` example), produces the sample set from 4.2,
and waits for the user's verdict.
