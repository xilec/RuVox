# Proposal: normalize-per-engine-blocks

## Why

Text preparation is a single engine-agnostic chain (`src-tauri/src/pipeline/`)
that hard-codes the capabilities of Russian-only engines: Latin is always
transliterated, digits and units always expanded, code identifiers always
narrated in Russian, and the "chunker" exists in three copies
(`silero-native/src/chunking.rs`, `tts/chunking.rs`, `ttsd/ttsd/chunking.py`).
Adding an engine that reads English or digits natively therefore forces either
duplicating normalization inside its adapter or double-normalizing: numbers are
expanded before the engine's own `num2words` runs, Latin is transliterated
before an engine that reads English can ever see it. There is no place to
declare what an engine can read, so the pipeline cannot vary per engine and
every new engine tempts a fork.

## What Changes

- **Engine text profile** — a data-only declaration of what an engine reads
  natively (languages, prose-Latin routing, digits, units, stress, allowed
  charset, chunking limit and pauses, markup syntax), keyed by `EngineKind` in
  a static registry and surfaced through `TtsEngine::text_profile()`.
- **Fragment route** — the profile decides per fragment kind whether the
  pipeline narrates it in Russian or leaves it for the engine. This change
  implements the two kinds with a decided answer (prose Latin runs and code
  identifiers); abbreviations, URLs/emails and fenced blocks wait for a probe
  (see Non-goals).
- **Annotated pipeline output** — normalized text plus `CharMapping` plus span
  annotations (`kind`, `language`, range in output coordinates).
  `process_for_engine` becomes the primary entry point;
  `process_with_char_mapping` remains a compatibility wrapper over the
  reference (Silero) profile.
- **One shared chunker** in a new `tts-contract` crate consumed by both
  `src-tauri` and `silero-native`, parameterized by profile limit and pauses;
  `ttsd` receives its limit in the request instead of hard-coding it.
- **Test infrastructure** — profile-parameterized golden harness, a test-only
  Latin-native profile that exercises every gated path, and
  "no double normalization" assertions.

## Non-goals

- **Language-span routing** (mixed RU/EN audio, per-language voices, the join
  with a pause and continuous highlighting) — a separate change, delivered
  with #308, the first engine that can validate it.
- The engines themselves (#308 TeraTTSv2, #309 Supertonic 3).
- **Gated built-in dictionaries** and engine-scoped dictionaries (#310);
  built-in tables stay unconditional here, the user dictionary keeps its
  precedence.
- **Stress marks** (the dictionary `+` syntax, stored stress position, adapter
  rendering and automatic stress) (#312); the profile declares `stress` as data,
  every shipping engine declares `None`, and nothing consumes it yet.
- Per-voice language catalogues: languages are declared per engine and all its
  voices inherit them; Silero exposes Russian voices only.
- Any audible change for Silero, `ttsd` and Piper: their output stays
  byte-identical on the golden corpus.

## Capabilities

### New Capabilities

- `text-chunking`: how normalized text becomes bounded synthesis calls — the
  limit unit, value and pause policy declared by the profile, the break
  preference, chunk offsets for timestamps, and the single-copy rule.

### Modified Capabilities

- `text-pipeline`: profile-driven gating, fragment route, `process_for_engine`,
  annotation output, adapter boundaries.
- `position-mapping`: span annotations beside `CharMapping` and their
  invariants.
- `piper-engine`: chunk limit and paragraph pause come from the profile.
- `silero-native-engine`: chunking moves to the shared chunker.
- `ttsd-protocol`: the `synthesize` request carries the chunk limit.

## Impact

- New crate `tts-contract` at the repository root, plus wiring in `flake.nix`,
  `justfile`, `lefthook.yml` and the CI workflows; `silero-native` gains its
  first external path dependency.
- `src-tauri/src/pipeline/*`, `src-tauri/src/tts/{engine,switcher,chunking,piper,silero_native}`,
  `silero-native/src/chunking.rs`, `ttsd/ttsd/{protocol,main,chunking}.py`.
- Golden fixtures are reorganized into profile-scoped folders; the existing 76
  cases stay byte-identical under the Silero profile.
- No `CHANGELOG.md` entry: internal refactor with no user-visible behavior
  change (see `ai/rules/conventions.md`, CHANGELOG bar).
