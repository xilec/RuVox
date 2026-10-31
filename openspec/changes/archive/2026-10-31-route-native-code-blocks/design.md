## Context

`Route` currently carries two kinds — a Latin run in prose and a code
identifier — and every engine narrates fenced code blocks. The #311 probe
(recorded in the issue, harness in #324) listened to a raw `code` fragment on
every engine: Piper and TeraTTSv2 accepted it, the Silero model family did
not. The code-block phase is a shared block that runs first; today it either
announces the block (`CodeBlockMode::Brief`) or spells it out in Russian
(`CodeBlockMode::Full`), and it has no way to hand the block to the engine.

Constraints that shape the implementation:

- The fixed phase order puts fenced code blocks before every content phase;
  later phases (English, identifiers, operators, symbols, numbers, the
  dictionary pre-pass) would rewrite any Latin code left in the text.
- `CharMapping` and word timestamps must keep working: the in-app highlight is
  the fallback for unclear audio.
- TeraTTS already turns span annotations into language segments, so a correct
  span is all the adapter needs; Piper has no span-aware call.
- TeraTTS rejects stray `<`/`>` (they are tag syntax), and real code contains
  them.

## Goals / Non-Goals

**Goals:**

- One explicit per-engine decision for fenced code blocks, honoured by the
  code-block phase.
- Native blocks reach the engine verbatim (whitespace aside), annotated and
  mapped, without any narrating phase touching them.
- TeraTTS reads them through the existing `<en>` segment path; Piper reads
  them as-is.
- The user-facing `code_block_mode` setting keeps its meaning.

**Non-Goals:**

- Native routes for URLs, emails, abbreviations or inline code.
- Per-token mapping inside a block; block-level collapse is deliberate.
- Any change for the two Silero profiles or the `ttsd` sidecar.

## Decisions

### D1. `code_block` is a third `Route` field

`Route { latin_prose, identifier, code_block }`, all `Reading`. `any_native()`
covers all three, and `validate` requires a language other than Russian for
any of them; `ProfileError::NativeLatinWithoutLanguage` becomes
`NativeWithoutLanguage` because the invariant is no longer about Latin prose
alone. A `Native` fragment must carry a language annotation, so an engine that
claims native code reading without a non-Russian language is a data error.

Piper adds `en` to `languages`. That list means "languages the engine accepts
natively", and the probe showed eSpeak-on-Russian handles Latin code; the
prose route stays `Narrate` because transliteration is still the better
reading there. The profile therefore deliberately has `code_block: Native`
with `latin_prose: Narrate`.

Rejected: a separate `CodeBlockRoute` type — it would duplicate the
`Narrate`/`Native` vocabulary and the pipeline would need two routing concepts.
Rejected: keeping Piper `languages: ["ru"]` and exempting the code route from
the invariant — the span would have no language to carry.

### D2. The narration mode stays authoritative; the route picks the reader

With `code_block_mode = "brief"` the block is still replaced by the
announcement on every engine. The route applies only in the read mode: there
the pipeline asks whether it or the engine reads the block. This keeps the
setting's promise ("do not read code aloud") independent of engine choice.

Rejected: letting a `Native` route override brief mode — a user who selected
«Кратко» would suddenly hear whole code blocks when they switch to TeraTTS or
Piper, a silent and noisy regression.

### D3. Native code leaves the tracked text behind a placeholder

The phase must run first, and the later phases must not see the code. In the
read mode with a `Native` route, the whole fenced block is replaced by a
unique private-use placeholder; after the punctuation post-processing and
before `build_mapping`, each placeholder is substituted by the code text in
the final `CharMapping`. Restoring through `TrackedText::replace` is not
possible: a replacement nested inside an existing replacement is rejected by
the tracking invariant, so the substitution splices `transformed` and
`char_map` directly through a dedicated `CharMapping` method.

Rejected: reordering the code-block phase to the end — it violates the fixed
phase order and lets markdown, URL, identifier and dictionary phases consume
code contents first.
Rejected: a "protected ranges" mechanism in every phase — broad machinery for
one fragment class, and every regex substitution would have to honour it.
Rejected: leaving the raw code in the text and trusting later phases — they do
match code; for TeraTTS, `digits: Expanded` would even read `42` as words.

### D4. Native block whitespace collapses to single spaces

The block content is trimmed and its internal whitespace runs become one
space. The text is still the code, but line layout is not readable content,
and TeraTTS drops `\n`/tabs from its alphabet (the gate would glue `fn main`
into `fnmain`). Piper reads the result as one utterance, like any other text.

Rejected: passing raw newlines — the reading would then depend on each
adapter's newline handling, and TeraTTS has none.
Rejected: pre-filtering unsupported punctuation in the pipeline — engine
alphabets are adapter data; the existing codepoint gate stays the only filter.

### D5. Span and mapping: block-level, like the narrated-full path

A native block records a `CodeBlock` span with the profile's native language;
the span's output range is exactly the code text. Every codepoint of the
inserted code maps to the whole original fenced range, because the block was
one replacement. This is the same granularity the current full narration has,
so word timestamps keep pointing into the source block and the highlight does
not regress relative to today.

Rejected: identity mapping per code character — it requires the code to stay
in the tracked text, which D3 rules out.

### D6. TeraTTS neutralises tag characters in native code spans

TeraTTS's text frontend rejects stray `<`/`>` with a typed error, because the
model can only interpret them as tag syntax. Native code legitimately contains
them (`Vec<T>`, `a < b`). The adapter replaces `<`/`>` with spaces only inside
spans of kind `CodeBlock` before building segments; the codepoint count is
unchanged, so spans and timestamps stay valid. No information the engine could
read is lost, and words do not glue (which a drop would cause).

Rejected: relaxing `validate_no_markup` for every English segment or for the
crate — a caller's broken markup should still fail, as today; the exception
must be narrow to pipeline-owned code spans.
Rejected: letting synthesis fail on `<` — common Rust/C++ blocks would break,
and the user could not act on the error.

### D7. Golden harness gains engine profiles

`NATIVE_EN_PROFILE` flips its code route too: it exists to exercise every
gated path and would otherwise stop covering one. As a result the two flat
code-block fixtures (`markdown_code_block`, `user_dict_code_block`) move to
`silero/`, where they pin the narrated behavior; the new native behavior is
pinned by fixtures under `teratts/` and `piper/`. The harness resolves folder
names against all four known profiles, but the flat corpus keeps running under
its two classified reference profiles by default, so nothing else moves.

Rejected: adding teratts and piper to the flat-corpus profile set — their
prose routes differ from silero by design, and every Latin fixture would have
to be reclassified and duplicated.

## Risks / Trade-offs

| Risk | Mitigation |
|------|------------|
| Native code sounds worse than Russian narration on some article | #311 listening verdict; the route only applies in the opt-in read mode; brief stays the default |
| TeraTTS drops punctuation absent from its alphabet and warns | Existing gate behavior; angle brackets are handled explicitly; the probe accepted the resulting reading |
| A placeholder leaks into the output if restoration is skipped | Restoration runs unconditionally before mapping; golden fixtures pin the text and a unit test pins no private-use codepoint in output |
| The block-level mapping makes the highlight cover the whole block | Same granularity as the narrated-full path today; per-token mapping is a documented non-goal |
| Piper's eSpeak voice mispronounces some Latin code | The verdict came from listening; the route is per engine and can be flipped back in one field |
