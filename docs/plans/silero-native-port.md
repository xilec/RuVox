# Plan: Silero Native engine (ONNX + Rust port of Silero TTS v5)

> Working plan for the "silero-native" subproject. Persistent across sessions;
> the phase tracker at the bottom reflects current progress.
> Will be superseded by an OpenSpec change (Phase 0) once proposed.
>
> Feasibility spike: `tmp/onnx-spike/` (report: `tmp/onnx-spike/REPORT.md`).
> The spike code is reference material only — production code is rewritten
> from scratch in Phase 2/3.

## Decisions (agreed with the user)

- **Model distribution:** pre-exported ONNX bundle published as a GitHub
  Releases artifact; the app downloads it on demand (like the torch model
  today). Not bundled into the installer.
- **Accentor scope:** full parity with v5 — ngram accentor (Rust + dicts +
  `accentor_tensor.onnx`) **and** the HomoSolver BERT (`homosolver.onnx`,
  ~117 MB) for homographs.
- **Engine role:** third engine option. UI name: **«Silero (нативный)»**.
  Piper stays the default; the Python `ttsd` engine remains untouched as
  fallback.
- **Sample rates:** all three (8000 / 24000 / 48000) — PQMF export is **in
  scope** for v1. Default sample rate for the native engine: **24000**.
- **Word timestamps from `dur_hat`:** **out of v1 scope** — tracked as a
  separate GitHub issue (see Appendix A).
- **Spike artifacts** (`tmp/onnx-spike/`) are not ported as-is; rewritten
  cleanly in Phase 2, used as reference.
- **First step of Phase 1:** validate `ort` + onnxruntime linkage inside the
  nix dev shell and the flake production build before any other work.

## Background (from the spike)

- `v5_ru.pt` is a `torch.package`; the pipeline is non-autoregressive
  (FastPitch-like): `dur_predictor → pitch_predictor → tacotron encoder →
  LengthRegulator (repeat_interleave) → HourGlass decoder → ConvNeXt vocoder
  → ISTFT (n_fft 2400, hop 600)`. PQMF resampling is used for 24k/8k.
- All components exported to ONNX with parity: `tts_main.onnx` (81 MB,
  waveform e2e max abs diff 1.7e-4), `istft.onnx` (23 MB, 8.5e-6),
  `homosolver.onnx` (117 MB, 4.2e-7), `accentor_tensor.onnx` (8 MB).
- Not exportable by nature: the string part of the accentor (ngram lookup +
  exceptions dict) — ported to Rust from `ngrams.gz` / `exceptions.gz`.
- Export requires programmatic JIT-graph surgery (custom symbolics for
  `_native_multi_head_attention`, `repeat_interleave`, removal of
  `aten::format` fast paths) — no model weights or code are patched.
- `return_ts=True` is unsupported by v5_ru; `dur_hat` is the 4th output of
  the main ONNX graph (frame hop 600 @ 48k = 12.5 ms).
- Known edge cases: zero durations in `repeat_interleave`; SSML
  `symbol_durs` branches not covered by the export.
- CPU speedup vs Python `apply_tts`: ~10x.

## Phase 0. OpenSpec proposal

- [ ] Run `openspec-propose`; design doc based on `tmp/onnx-spike/REPORT.md`
      and this plan; delta specs for: new engine, model bundle download,
      settings UI.
- [ ] User approval of the proposal.

## Phase 1. Subproject scaffolding

- [ ] **(FIRST, blocker check)** `ort` + nix validation: minimal crate that
      loads an ONNX model with `pykeio/ort` inside `nix develop`, plus a
      successful production build via the flake (`.#ruvox`). Decide the
      linkage strategy (system onnxruntime vs `load-dynamic`) and document
      it in `nix/devshell.nix` comments.
- [ ] `silero-native/` crate: own `Cargo.toml` (no root workspace;
      `src-tauri` consumes it via path dependency), `src/`, `export/`,
      `tests/`, `docs/`, `README.md`.
- [ ] CI: crate tests wired into `.github/workflows/ci.yml`; slim-build gate
      untouched.

## Phase 2. Production model exporter

- [ ] `silero-native/export/` uv project: single `export.py` producing the
      bundle: `tts_main.onnx`, `istft.onnx`, `pqmf.onnx` (24k/8k),
      `homosolver.onnx`, `accentor_tensor.onnx`, `ngrams`/`exceptions`
      dicts, `manifest.json` (model version, per-file sha256, opset, source
      `.pt` hash, export date).
- [ ] Pin the exact model version from `models.yml` (no `latest`).
- [ ] Self-check inside the exporter: waveform parity ≤ 1e-3 on a fixed
      phrase set; non-zero exit on failure.
- [ ] GitHub Actions workflow: run exporter, upload bundle to Releases
      (`silero-models-v5.x` tag).
- [ ] `export/README.md`: how to re-export on a new Silero release.

## Phase 3. Rust engine core

- [ ] Text frontend port (from unpacked package sources): `prepare_text_input`
      (lowercase, symbol filtering, dash normalization), symbol→id mapping.
      Non-goals: SSML, `[[...]]` phonetic inserts (recorded in the spec).
- [ ] Accentor: Rust ngram lookup (`ngrams.gz`, `exceptions.gz`) +
      `accentor_tensor.onnx` via `ort`.
- [ ] HomoSolver: BERT tokenizer port
      (`custom_tokenizers/bert_tokenizer.py` → Rust; most delicate part) +
      `homosolver.onnx`.
- [ ] Synthesis: `tts_main.onnx` + `istft.onnx` (+ `pqmf.onnx` for
      24k/8k), speaker_id 0–4, all three sample rates, default 24000.
- [ ] Public API: `SileroNative::load(bundle_dir)` /
      `synthesize(text, speaker, sample_rate) -> SynthesisResult`
      (wav + char-proportional timestamps, same shape as `ttsd` today);
      called via `spawn_blocking`, panic isolation.
- [ ] Reuse existing chunking/sanitization on the `src-tauri` side (no
      duplication in the crate).

## Phase 4. Testing

- [ ] Golden parity suite (~20–30 phrases: tech text, numbers, homographs,
      ё, punctuation): identical post-frontend text, waveform max abs diff
      within threshold, stress marks identical to `apply_tts`.
- [ ] Unit tests: ngram lookup, BERT tokenizer, edge cases (empty string,
      single vowel, zero durations).
- [ ] Integration test: full engine on the downloaded bundle (CI: cache or
      fetch from Releases).
- [ ] Benchmark reproducing spike numbers (~10x vs `apply_tts`); recorded
      in report, not a hard gate.
- [ ] Listening checklist (ref vs native wavs) for the manual pre-PR pass.

## Phase 5. App integration (third engine)

- [ ] Bundle downloader: fetch from GitHub Releases into data dir, sha256
      verification against manifest, progress in UI, offline/error states.
- [ ] `src-tauri/src/tts/`: `SileroNative` backend next to `Ttsd`; engine
      selection Piper (default) / Silero (Python) / «Silero (нативный)» in
      `src/dialogs/Settings.tsx`; Russian UI strings.
- [ ] Graceful degradation: engine unavailable until the bundle is
      downloaded, with a clear message and a download action.

## Phase 6. Docs and licensing

- [ ] `silero-native/README.md`: overview, architecture diagram, build/test,
      model download/re-export.
- [ ] `silero-native/docs/architecture.md`: full pipeline map, bundle/manifest
      format, **debugging section** (dumping intermediate tensors, comparing
      against the Python reference, known edge cases).
- [ ] License notices: model files are CC BY-NC-SA 4.0 © Silero Team —
      `NOTICE` file in `silero-native/`, section in README and app docs;
      ONNX is a technical format conversion (Section 2(a)(4) of the
      license), non-commercial use, link to the upstream repo. Our code
      (crate + exporter) stays under the repo license.
- [ ] Update `AGENTS.md` (layout, test commands), `docs/`; archive the
      OpenSpec change (syncs delta specs).

## Phase 7. Pre-PR gate and PR

- [ ] Archive the change → `ruvox-reviewer` over the diff → manual pass
      (engine switching, bundle download, listening parity) → draft PR
      description for approval → PR, merge commit after green CI.

## Estimates

- Phase 1: 2–4 days (mostly the nix/ort check).
- Phase 2: 3–5 days.
- Phase 3: 5–7 days.
- Phase 4: 3–4 days.
- Phase 5: 3–4 days.
- Phase 6: 1–2 days.
- Total: ~3–4 weeks.

## Risks

- `ort` / onnxruntime linkage in the nix bundle — checked first (Phase 1).
- BERT tokenizer port drift — covered by golden parity tests.
- Upstream model updates breaking the exporter — mitigated by version
  pinning + exporter self-check.

## Epic operating rules (agreed — in force for the whole epic)

User authorization: «реализуй этот план автономно, кроме этапов, которые
требуют ручного тестирования» — this lifts the draft-approval rule for
commits/pushes on the task branch for the duration of this epic.

1. **GitHub autonomy.** I compose English Conventional Commit messages
   (`<type>(<module>): <desc>`), commit per task group, and push the task
   branch `xilec/silero-native` to origin as a backup — without pausing.
   **No PR, no merge**: the pre-PR gate (ruvox-reviewer + manual test,
   task 8.1) requires the user, so the epic ends with a reviewed, green,
   pushed branch and stops there.
2. **One change = one branch = one PR** (project convention overrides the
   per-task-PR default). Work happens in the current workspace on
   `xilec/silero-native` (branched off fresh `origin/main`); no worktrees —
   work is sequential.
3. **Who codes.** Orchestration, OpenSpec artifacts, docs/licensing — main
   session. Heavy well-scoped chunks (exporter, engine core, integration)
   — `coder` subagents with `[routine]`/`[complex]` prefix in the
   description, explicitly authorized to commit on the branch with narrow
   `git add <paths>`.
4. **Review after every task group** — a reviewer subagent over the diff of
   that group, with the right to fix and re-run gates.
5. **Environment.** All commands via `nix develop -c "..."` (cargo, pnpm,
   uv exist only in the dev shell); `tmp/` (not `/tmp`) for scratch.
6. **Logging.** After every stage: append outcome, gate results, and notes
   to `tmp/port_log.md`; update the State tracker below; check tasks in
   `openspec/changes/add-silero-native-engine/tasks.md`.
7. **No stopping between task groups.** Stop only at the pre-PR gate.

## Shared technical reference (survives /clear)

- **Gates:** `nix develop -c cargo test --manifest-path silero-native/Cargo.toml`,
  `nix develop -c cargo clippy --manifest-path silero-native/Cargo.toml -- -D warnings`,
  `nix develop -c cargo fmt --check`, `nix develop -c just lint` (full, incl.
  src-tauri clippy, cargo-deny, eslint, knip, tsc, ruff),
  `nix develop -c cargo test --manifest-path src-tauri/Cargo.toml` (when
  src-tauri is touched), `nix develop -c pnpm typecheck` (when src/ touched),
  `nix develop -c pnpm dlx @fission-ai/openspec validate add-silero-native-engine --strict`.
- **Conventions:** `ai/rules/conventions.md` + `ai/rules/code-quality.md`
  (English code/docs, Russian UI strings, no unwrap in prod paths,
  thiserror domain errors, tracing).
- **Feasibility reference:** `tmp/onnx-spike/REPORT.md` + scripts (rewrite
  cleanly; do not port spike code as-is). Model cache:
  `~/.cache/torch/hub/.../v5_ru.pt`. Unpacked package: `tmp/onnx-spike/pkg/`.
- **Spec/source of truth:** `openspec/changes/add-silero-native-engine/`
  (proposal, design, specs, tasks).
- **Known edge cases:** zero durations in `repeat_interleave` (clamp to
  model minimum), `return_ts=True` unsupported by v5_ru (use char-
  proportional timestamps in v1; issue #145 later), SSML/`symbol_durs`
  branches not exported.
- **License:** model bundle = CC BY-NC-SA 4.0 (Silero Team), format
  conversion per §2(a)(4); `NOTICE` + attribution required.

## State tracker (source of truth after /clear)

| Task group | Status | Notes |
|---|---|---|
| 0. OpenSpec proposal | done | `openspec/changes/add-silero-native-engine/`, validated; issue #145 filed |
| 1. Nix/ort validation | done | ort+onnxruntime already in place via piper-rs; `load-dynamic` + `ORT_DYLIB_PATH` (nixpkgs onnxruntime 1.26.0); `nix build .#ruvox` green; commit 52c23e5 |
| 2. Scaffolding | pending | |
| 3. Exporter | done | bundle in `tmp/bundle-v5/` (230 MB, 12 files + manifest); self-check parity ≤ 8.2e-4 all phrases; commit 5371a2b; GH workflow written, not run on GH |
| 4. Rust core | done | full frontend (text/accentor/homosolver) + engine + API; 28 tests green; e2e parity vs torch 9.2e-5; commits c82bc5c..2a00bf3 |
| 5. Testing | done | parity suite 31 cases (ONNX refs, worst 9.8e-4); +13 unit tests; no-bundle CI path green; bench ~1.3x vs warmed apply_tts (spike ~10x was cold-vs-partial); commits d2b37e0.. |
| 6. Integration | done | third engine end-to-end (backend, downloader, IPC, Settings UI); gates green; commits 8b081ed..d044113 |
| 7. Docs & licensing | done | README, NOTICE (CC BY-NC-SA attribution), architecture.md (pipeline/bundle/debugging/error codes), AGENTS.md, CHANGELOG |
| 8. Manual verification | manual gate | user, pre-PR |

## Progress tracker (phases, superseded by the State tracker above)

| Phase | Status | Notes |
|---|---|---|
| 0. OpenSpec proposal | artifacts created | `openspec/changes/add-silero-native-engine/`, validated; awaiting user approval |
| 1. Scaffolding | pending | starts with ort+nix check |
| 2. Exporter | pending | |
| 3. Rust core | pending | |
| 4. Testing | pending | |
| 5. Integration | pending | |
| 6. Docs & licensing | pending | |
| 7. Pre-PR & PR | pending | |

## Appendix A. Deferred issue draft (dur_hat timestamps)

Title: `feat(tts): precise word-level timestamps from dur_hat in Silero Native`

Filed as <https://github.com/xilec/RuVox/issues/145> (text approved by the
user beforehand).
