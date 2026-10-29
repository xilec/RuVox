# Design

## Context

See `proposal.md` — Why. The current state that shapes the approach:

- `TTSPipeline` (`src-tauri/src/pipeline/mod.rs`) owns seven normalizers and
  runs 22 phases in a fixed order; `TrackedText` keeps `char_map` as
  *transformed → original* (spec `position-mapping`), so the original span of
  any current position is a direct lookup, while the reverse direction needs a
  pass.
- `TtsEngine` (`src-tauri/src/tts/engine.rs`) is the only engine-facing trait;
  `EngineKind` has three variants and is `snake_case`-serialized into history
  and generation snapshots. `TtsSwitcher` implements the trait by delegating.
- Chunking exists three times: 900 codepoints in `silero-native`, 900 in
  `ttsd`, 500 in Piper, plus a 450 ms paragraph pause that only Piper applies.
- `src-tauri` and `silero-native` are two independent cargo workspaces;
  `src-tauri` path-depends on `silero-native`, never the other way round.
- There is no language metadata for voices anywhere: `speaker` is an opaque
  string, and the Piper catalogue holds four `ru_RU` voices.

## Goals / Non-Goals

**Goals:**

- One declarative contract per engine: adding an engine means a profile and an
  adapter, with no edits to shared blocks.
- Every normalization rule has exactly one owner, runs at most once, and that
  fact is asserted by tests.
- Zero behavior change for the three existing engines, with a test-only
  profile that proves every gated path is reachable and correct.

**Non-Goals:**

- Segmentation and audio joining for language spans (separate change).
- Per-voice capability negotiation, engine-scoped dictionaries (#310), and
  route flips that need listening rather than reasoning (probe issue).

## Decisions

### D1. Shared contract types live in a new `tts-contract` crate; the registry stays in `src-tauri`

`TextProfile`, `Route`, `Markup`, the chunker and the chunk/pause limits are
data and pure functions needed by two workspaces. `silero-native` cannot
depend on `src-tauri` (it is the application), so the shared code has to sit
below both.

- *Alternative — keep the splitter in `silero-native` and have `src-tauri`
  import it:* rejected. Piper and every later engine would depend on the Silero
  crate, which is exactly the cross-engine coupling `tts/chunking.rs` explicitly
  refused. Deleting `silero-native` would delete the shared code with it.
- *Alternative — keep three copies:* rejected. It fails the single-owner
  acceptance criterion, and the copies have already diverged (900/900/500,
  pauses only in Piper).

The crate knows nothing about `EngineKind`: the static registry
`fn profile_for(kind: EngineKind) -> &'static TextProfile` stays in
`src-tauri/src/tts/profiles.rs`, and `TtsEngine::text_profile()` delegates to
it (the switcher delegates to the active engine).

### D2. Languages are an engine-level capability; voices inherit them

A profile declares which language codes its engine accepts (`["ru"]` today,
`["ru", "en"]` for a multilingual engine). Every voice of that engine inherits
the declaration; Silero exposes Russian voices only and keeps emulating English
through transliteration.

- *Alternative — per-voice languages with an effective profile computed as an
  intersection on every call:* rejected for now. No voice carries language
  metadata today, the intersection adds a moving part with zero consumers, and
  the first multilingual engine (#308) ships its voices in one language set.
  When a voice catalogue gains real per-voice languages, the effective profile
  becomes a cheap intersection without changing the contract.

### D3. The fragment route is an explicit per-kind table, never derived from `languages`

`languages` answers "which languages can this engine speak"; the route answers
"who reads this fragment". They are different questions: an engine can read
English prose natively and still produce worse identifiers than the pipeline's
identifier narration.

- *Alternative — derive `Native` from `languages.contains("en")`:* rejected.
  It silently forces every Latin-native engine to hand over identifiers as
  well, removing the one decision an engine author most needs to make.

### D4. Only fragment kinds with a decided answer are routed now

This change routes `latin_prose` and `identifier`. Abbreviations travel with
`latin_prose` rather than getting a field of their own: reading "API" in Cyrillic
is the same compensation as transliterating "Kubernetes", so a profile that
reads prose natively reads abbreviations natively too, and splitting them would
serve a combination no engine has asked for. URLs/emails and fenced blocks stay
narrate-only until the probe issue produces evidence — their narration is
structural ("точка", "слэш", block markers), not a transliteration of prose.
`Route` is a struct with an explicit field per kind, so adding a kind is a code
change, not a data change.

Known gap, deferred to #311: URL narration still transliterates Latin path
components ("example.com/path" → "экзампл точка ком слэш пат") even under a
`Native` prose route, because the URL block owns that fragment. Leaving URLs
verbatim is what the URL route will decide.

- *Alternative — declare all five kinds and implement two:* rejected. An enum
  field whose `Native` value is unimplemented invites a caller to set it and
  silently get narrate behavior.

### D5. `Markup` is three named variants with a shared `wrap()` helper

`Markup::{None, RuEnTags, LangTag}`; `wrap(&self, lang, text)` lives in
`tts-contract` so adapters render tags identically. The pipeline never emits
tags, and `CharMapping` never sees them.

- *Alternative — parameterized `{open, close}`:* rejected. It moves the
  knowledge of what an engine needs into configuration strings and loses the
  exhaustiveness a new syntax variant should force.

### D6. Spans are stored in original coordinates internally and exposed in output coordinates

A phase detects a fragment in the coordinates of the text as it stands and
immediately converts to the original range using the current `char_map` (a
direct lookup). At the end, one pass over the final `char_map` resolves each
span into output coordinates. There is no second range bookkeeping and no
reverse index.

- *Known limitation:* an identifier is replaced by one mapping entry covering the
  whole identifier, so an override applied to one of its parts has no distinct
  output range; part-level annotations wait for part-wise replacement (#308).

- *Alternative — store spans in output coordinates from the start:* rejected.
  Later phases shift output positions, so every phase would need to re-base
  every recorded span, creating a second range system that must agree with
  `char_map` — a direct violation of the single-owner rule.
- *Alternative — a profile field selecting the coordinate space per engine:*
  rejected. The public type would carry two spaces, every consumer would have
  to ask the profile which one it got, and the invariant "spans agree with
  `char_map`" would become conditional. The engine does not influence where the
  pipeline counts.

### D7. `digits` and `units` split at the existing normalizer entry points

`digits` gates `normalize_number`, `normalize_float`, `normalize_range`,
`normalize_version`, `normalize_date`, `normalize_time`; `units` gates
`normalize_percentage` and `normalize_size` (plus currency and `№` handled in
the same phase). The split is already visible in the code and needs no new
classification.

### D8. One chunker for in-process engines; `ttsd` keeps its algorithm and receives the limit

`tts-contract::chunk` replaces `tts/chunking.rs` and
`silero-native/src/chunking.rs`; the profile supplies the limit unit
(`Chars` or `Secs`) and the pause policy. `ttsd` keeps its Python splitter —
a Rust crate cannot be imported there — but the limit travels in the
`synthesize` request instead of a Python constant.

- *Alternative — move `ttsd` chunking into Rust:* rejected. It changes the
  timestamp source (Python estimates timestamps per chunk today) and the
  byte-identical acceptance criterion for `ttsd` becomes unreachable; it is a
  separate change.
- *Alternative — leave the Python constant:* rejected. Two homes for one
  number is exactly what this change exists to remove.

### D9. Stress is a structural dictionary field, never a `+` in pipeline text

`to` may carry one `+` before a vowel; it is parsed at load into the entry and
preserved verbatim on import/export. The profile declares `stress: None | Auto
| Manual`; an explicit mark vetoes automatic stress for that word; the adapter
renders marks in its own syntax.

- *Alternative — the pipeline emits `+` into the text:* rejected. `+` would
  leak into `CharMapping`, the symbol phase and charset filtering, and every
  downstream consumer would need to know it is not content.

### D10. Per-call profile, reference wrapper, no `process()`

`process_for_engine(&mut self, input: &str, profile: &TextProfile) ->
ProcessedText` is the primary entry point. `process_with_char_mapping` stays as
a wrapper over `SILERO_PROFILE` (the reference profile: golden corpus and
dictionary-entry validation). `process()` is removed — production never used
it. Callers pass `tts.text_profile()` captured in the same critical section as
`code_block_mode`, so a racing engine switch cannot falsify the snapshot.

- *Alternative — keep the profile in a pipeline field set by a `set_profile()`
  setter:* rejected. It duplicates the active-engine state that
  `TtsEngine::kind()` already owns and reintroduces the
  state-synchronization bug class the `code_block_mode` snapshot exists to
  avoid.

### D11. A test-only profile parameterizes the golden harness

`native_en` (`["ru", "en"]`, `digits: Native`, `units: Native`, every route
`Native`) is a `#[cfg(test)]` constant, not a registry entry, so `EngineKind`
does not grow dead variants. The harness takes the profile list from an
environment variable; running the corpus under a substituted profile is what
classifies a fixture as profile-specific, and fixtures move into
`tests/fixtures/pipeline/<profile>/` mechanically rather than by inspection.

### D12. `charset` is declared by the profile, enforced by the adapter, asserted by a test

The pipeline never filters. The adapter filters defensively against the
declared alphabet, and a per-profile test asserts that the pipeline's output is
already a subset of it — so a rule emitting a codepoint the model silently
drops fails a test instead of losing characters in audio.
`sanitize_for_silero` (newline collapsing before the model tokenizer) is an
adapter step for the same reason: it is about the model's tokenizer, not about
a text decision.

## Risks / Trade-offs

- **[An abstraction with no live consumer in this change]** → The test-only
  `native_en` profile exercises every gated path, the probe issue supplies the
  evidence for the remaining kinds, and flipping a real engine's route is a
  one-line profile change — the cheapest possible proof.
- **[A third cargo workspace entry breaks `nix`/CI builds]** → It lands in its
  own PR with a green CI before any behavior moves; the crate has no build
  script, no external dependency, and no feature flags.
- **[The `ttsd` protocol change is cross-language]** → `chunk_limit` is
  optional with a 900 default, so an older sidecar keeps working; the mock
  sidecars in `src-tauri/tests/fixtures/mock_ttsd_*.py` cover both shapes.
- **[The annotation pass costs time on 1 MB inputs]** → It is one linear pass
  over `char_map`, run only when a span-producing rule fired; `tests/perf.rs`
  keeps its existing budget and is measured before/after.
- **[Spans and `char_map` drift apart]** → Property tests assert sortedness,
  non-overlap, bounds and "every span resolves to a non-empty output range"
  under every profile.

## Migration Plan

Three PRs on one branch, each independently green:

1. `tts-contract` crate + profile/route/markup types + registry + invariants
   (no behavior change; chunker moves here without being wired to engines yet).
2. `process_for_engine` + span annotations + active-profile plumbing through
   `run_normalization` and `preview_normalize` + golden harness parameterization.
3. Shared chunker wired to Piper and `silero-native`, profile limits and
   pauses, `ttsd` request field, fixture reorganization.

Rollback is a plain revert per PR; no persisted data changes (`EngineKind`
variants are unchanged, history and generation snapshots keep their shape).

## Open Questions

None. Deferred decisions that cannot change these specs live in the probe
issue (#311) and in the language-routing change.
