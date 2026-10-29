# Tasks

## 1. Contract crate and registry

- [x] 1.1 Create the `tts-contract` crate (library, no external dependencies)
  with the data types `TextProfile`, `Route`, `Markup`, `Digits`, `Units`,
  `Stress`, `Charset`, `Chunk`, `Pauses`, plus `Markup::wrap(lang, text)`.
  Verify: `cargo test --manifest-path tts-contract/Cargo.toml` covers each
  `Markup` variant; `cargo fmt --check` and `clippy -D warnings` clean.
- [x] 1.2 Move the splitter from `src-tauri/src/tts/chunking.rs` into
  `tts-contract` unchanged, carrying its unit tests over; leave
  `tts/chunking.rs` as a re-export shim so no caller breaks in this task.
  Verify: the moved tests pass unchanged and the diff of the algorithm body is
  empty apart from the module path.
- [x] 1.3 Wire the new crate into both cargo workspaces and the tooling
  (`flake.nix`, `justfile`, `lefthook.yml`, `.github/workflows/ci.yml`) so
  fmt, clippy and tests cover it. Verify: `nix develop -c just lint` and
  `nix develop -c just test` green, and the CI config lists the third crate.
- [x] 1.4 Add `src-tauri/src/tts/profiles.rs` with the Silero, Silero-native and
  Piper profiles and `profile_for(EngineKind)`, plus a test asserting every
  `EngineKind` variant resolves and that no profile routes a Latin fragment
  `Native` without a non-Russian language. Verify: the registry test fails when
  a variant is added without a profile.
- [x] 1.5 Record the per-crate README rule in `ai/rules/conventions.md` and
  apply it: a crate README says what the crate is for and which command checks
  it, then links to where the details live (`openspec/specs/`, `docs/`,
  `AGENTS.md`) instead of duplicating them. Verify: `src-tauri/`,
  `silero-native/` and `tts-contract/` each carry such a README and none of
  them repeats a file tree, an API table or a field-by-field description.

## 2. Profile reaches the pipeline

- [x] 2.1 Add `TtsEngine::text_profile()` with a default that resolves through
  `profile_for(self.kind())`; make `TtsSwitcher` delegate to the active engine.
  Verify: unit test asserts the switcher reports the active engine's profile
  after a switch.
- [x] 2.2 Thread the active profile through `run_pipeline_normalization` and
  `preview_normalize`, captured in the same critical section as
  `code_block_mode`. Verify: test asserts the generation snapshot and the
  preview both use the profile captured with the text, not a later config read.

## 3. Route-aware normalization

- [x] 3.1 Introduce `ProcessedText { text, mapping, spans }` and
  `process_for_engine(input, &profile)`; make `process_with_char_mapping` a
  thin wrapper over the Silero profile and delete `process()`; update
  `commands/mod.rs` and `commands/dictionary.rs`. Verify: every existing golden
  fixture stays byte-identical and `cargo test --manifest-path
  src-tauri/Cargo.toml` green.
- [x] 3.2 Gate transliteration and Russian narration on the fragment route:
  with `latin_prose: Native` a prose Latin run survives, with `identifier:
  Native` an identifier survives, otherwise today's output is produced. Verify:
  unit tests per block, including "routed Native leaves the fragment
  untouched" and "the narrating profile reproduces the existing reading".
- [x] 3.3 Gate the number and unit phases: `digits: Native` skips number,
  float, range, version, date and time expansion; `units: Native` skips
  percentages and sizes; both leave their fragments untouched and no later
  phase re-reads them. Verify: unit tests with `digits`/`units` in each of the
  four combinations, plus the existing number fixtures under the Silero
  profile.
- [ ] 3.4 Add "no double normalization" assertions: idempotence for
  transliteration and number expansion, and "already narrated fragment is not
  re-read" for URLs and identifiers. Verify: the new tests plus a fixture that
  reproduces a double-application bug if the gating regresses.

## 4. Span annotations

- [ ] 4.1 Record spans in original coordinates as fragments are detected and
  resolve them in one pass after the final trim; drop spans that no longer map
  to output. Verify: unit tests for identifier, dictionary-override and native
  prose-run spans, and for a fragment deleted by a later phase.
- [ ] 4.2 Cover the span invariants with a property test over every golden
  fixture under every harness profile: sorted, non-overlapping, within bounds,
  `start < end`, and `char_map` invariants untouched. Verify: the property test
  fails on a deliberately overlapping span in a scratch run.
- [ ] 4.3 Add the span kinds to the pipeline result consumed by adapters, with
  the language present only on native spans. Verify: serialization/diff test of
  a `ProcessedText` value for a mixed input.

## 5. Chunker, limits and profiles' consumers

- [ ] 5.1 Piper: obtain the chunk limit and paragraph pause from its profile,
  call the shared splitter, and delete the re-export shim and the local copy.
  Verify: the chunked-synthesis tests pass with profile-provided values, and a
  test with a different profile limit changes the chunk boundaries without
  touching engine code.
- [ ] 5.2 silero-native: replace the local splitter with the shared crate
  parameterized by the Silero profile, keeping `sanitize_for_silero` as an
  engine-side step. Verify: chunk tests pass and the bundle-gated parity suite
  is unchanged when `SILERO_NATIVE_BUNDLE` is set.
- [ ] 5.3 ttsd: accept optional `chunk_limit` (default 900), validate it
  (positive integer), and use it in the sidecar's chunker. Verify:
  `cd ttsd && uv run python -m pytest` covers the custom limit, the default and
  a rejected limit; the mock sidecars still satisfy the Tauri-side tests.
- [ ] 5.4 Profile-parameterized golden harness: run the corpus under a list of
  profiles from `RUVOX_GOLDEN_PROFILES`, add the `#[cfg(test)]` `native_en`
  profile exercising every gated path, run the classifier once, and move
  profile-specific fixtures into `tests/fixtures/pipeline/<profile>/` while
  profile-invariant ones stay in place. Verify: the harness runs common
  fixtures under every profile and each profile folder under its own; the
  classification run is reproducible (same move list on a second run).

## 6. Integration checks

- [ ] 6.1 Full gates: `nix develop -c just test` and `nix develop -c just lint`
  green, including `cargo deny` and the Python suite.
- [ ] 6.2 Non-regression: Silero, `ttsd` and Piper golden output byte-identical
  (diff of every expected fixture empty) and the 1 MB performance budget
  unchanged in `tests/perf.rs`.
- [ ] 6.3 `openspec validate normalize-per-engine-blocks --strict` reports no
  errors (length warnings only).
- [ ] 6.4 Manual pass: run the app, synthesize a mixed technical text with the
  default engine, and confirm audio and per-word highlighting match the
  pre-change behavior.

## Workflow follow-up

- Sync the delta specs into `openspec/specs/` and archive the change.
- Update the checklists in #307 and reference #310 and #311 as the deferred
  scopes.
- Open the PR with the three commits (contract crate, route and spans, chunker
  and fixtures) after the manual pass is accepted.
