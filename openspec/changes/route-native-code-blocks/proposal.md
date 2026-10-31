# Change: route fenced code blocks natively for TeraTTS and Piper

## Why

The #311 listening probe found that TeraTTSv2 and Piper read fenced code
blocks acceptably themselves, but `TextProfile::Route` has only
`latin_prose` and `identifier`. In the «Читать полностью» mode the pipeline
therefore still spells the block out in Russian for both engines —
transliterated identifiers, operator names and expanded numbers — which is now
the worst-sounding fragment on the engines that can read it.

## What Changes

- **Route gains a code-block reading.** `Route` grows
  `code_block: Reading`; `any_native()` includes it, and
  `TextProfile::validate` requires a non-Russian language for any `Native`
  route (the previous error name assumed Latin prose only).
- **The code-block phase honours the route.** In the read mode, a `Native`
  code block is passed through instead of narrated: whitespace inside the
  block collapses to single spaces, the block carries a `CodeBlock` span with
  the engine's language, and every later narrating phase (English,
  identifiers, operators, symbols, numbers) is kept off it. Brief mode keeps
  its announcement, mermaid keeps its marker, and `Narrate` keeps today's
  narration.
- **Per-engine values.** `TERATTS_PROFILE` and `PIPER_PROFILE` set
  `code_block: Native`; Piper also declares `en` among its native languages,
  because it reads Latin code acceptably while its prose route stays
  `Narrate`. Both Silero profiles (and the `ttsd` sidecar that shares their
  alphabet) keep `Narrate`.
- **Mapping and highlight stay intact.** A native block collapses to its
  source range in `CharMapping` exactly like the existing full-narration path,
  so word timestamps keep pointing into the original fenced block.
- **TeraTTS reuses the segment path.** The `CodeBlock` span becomes an English
  segment and is wrapped in `<en>` with no adapter rewrite; Piper ignores the
  span and reads the text as-is. The TeraTTS adapter gains one guard: tag
  characters inside a native code span are replaced by spaces before wrapping,
  because code with generics or comparisons would otherwise fail validation.
- **Tests and fixtures.** Unit tests for the route gate, span and mapping, and
  one golden fixture per flipped engine (TeraTTS, Piper) with the read mode.
- **Docs.** README code-block bullets, README.en mirror and a CHANGELOG
  `[Unreleased]` note.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `text-pipeline`: the engine-profile invariants, the fragment route (a third
  routed kind), the fenced-code-block requirement (route versus narration
  mode) and the span annotation output (a code-block kind).
- `position-mapping`: the span annotation model gains the code-block kind.
- `multi-language-synthesis`: native routing per engine profile now covers
  fenced code blocks for TeraTTS and Piper.
- `teratts-engine`: language-tag validation for native code spans.

## Impact

- Affected code: `tts-contract/src/profile.rs`, `src-tauri/src/pipeline`
  (`mod.rs`, `normalizers/code_blocks.rs`, `spans.rs`, `tracked_text.rs`),
  `src-tauri/src/tts/profiles.rs`, `src-tauri/src/tts/teratts/engine.rs`,
  the golden harness in `src-tauri/tests/`.
- Fixtures: two code-block cases move from the flat corpus to `silero/`
  (`markdown_code_block`, `user_dict_code_block`) because the test-only
  `NATIVE_EN_PROFILE` also flips its code route; new cases appear under
  `teratts/` and `piper/`.
- User-visible: on TeraTTSv2 and Piper, «Читать полностью» now reads code
  with the engine's own voice instead of Russian narration. «Кратко» is
  unchanged, and the Silero engines are unchanged.

## Non-goals

- Native routes for URLs, emails or abbreviations; inline code (single
  backticks) also stays narrated.
- Changing what `code_block_mode` means: «Кратко» still replaces a block with
  the announcement on every engine; the route only picks the reader of a block
  the user asked to read.
- Precise per-token mapping inside a native block: it collapses to the block
  range, the same granularity the narrated-full path already has.
- Indented or HTML code blocks, new engines, voices or profile fields beyond
  the code-block route.
