# Design: route Latin spans to the TeraTTS English frontend

## Context

`text-pipeline` already records `Span { kind, lang, start, end }` over the
normalized text (#307), and `TextProfile` already carries `languages`,
`route.latin_prose` / `route.identifier` and `Markup::RuEnTags`. The RU-MVP
`TERATTS_PROFILE` kept every route `Narrate`, so the spans were always empty
and the adapter only ever called the Russian frontend. The #311 probe
(2026-10-30) rendered native `<en>` samples for prose, brands, identifiers,
abbreviations, a URL, an email and a code fragment: prose, brands and
identifiers were accepted; URL, email and code were rejected (`@`, `{`, `}`
are absent from the model alphabet, and the URL is read as a symbol stream).
Abbreviations are a pipeline concern, not a native-voice concern.

## Decisions

### D1. Native only for prose and identifiers; everything else stays Russian

`TERATTS_PROFILE` sets `latin_prose: Native` and `identifier: Native`, keeps
`digits: Expanded`, `units: Expanded`, `stress: Manual`,
`markup: RuEnTags`, `chunk: Secs(20)` and `segment_ms: 200`. URLs, emails and
fenced code never produce `Native` spans today, so they keep their Russian
narration without a new profile field. Pure Latin runs (`API`) become native;
alphanumeric tokens (`IPv6`, `HTTP/2`) are narrated by the pipeline as a whole
because the model alphabet cannot read the mixed token and the pipeline's
symbol/number phases would otherwise leave half of it in Latin.

Rejected: `identifier: Narrate` with only prose native. The probe accepted
`getUserData`, and identifiers are the most valuable native fragment in
technical articles.

Rejected: a new `Route.abbreviation` field. It would change the shared
contract for one engine and one narrow class; the existing abbreviation
normalizer can read the token as a whole without a new route.

### D2. Spans stay decisions; the adapter renders segments

The pipeline never emits `<ru>`/`<en>` markup. The TeraTTS adapter walks the
sorted spans, turns Russian gaps and English spans into `teratts::Segment`s,
and lets the crate render the tags. `DictionaryOverride` spans (`lang: None`)
stay Russian. Empty/whitespace-only segments are skipped; `text_start` keeps
every timestamp in whole-text codepoint coordinates.

Rejected: teaching the pipeline to emit tags. It would put engine-specific
markup into shared normalization and break the "spans are decisions" rule.

### D3. One voice for every segment

TeraTTSv2 voices are cross-lingual: an English reference voice can speak
Russian and a Russian voice can speak English (the upstream model card
documents cross-language prompts and suggests a lower `duration_scale` for an
English voice on Russian). The engine therefore keeps the configured
`teratts_voice` for every segment; `Language` changes only the tag and whether
RUAccent runs. No English voice setting exists, and the voice never changes at
a language boundary. An isolated one- or two-letter Russian segment (the
conjunction `и`, the preposition `в`) makes the duration predictor return a
nearly fixed ~0.9 s, which sounds stretched between English fragments; such a
segment uses 0.3× the configured duration scale, while longer text keeps the
measured optimum.

Rejected: a separate `teratts_en_voice` with automatic switching. It makes one
utterance change timbre mid-sentence and doubles the voice configuration for
no model need; the upstream probe used `eng_f5` only to make the comparison
unambiguous.

### D4. One engine call for the whole utterance

`TtsEngine::synthesize_spans` defaults to the existing `synthesize`, so the
three Russian-only engines and their tests are untouched. TeraTTS overrides it
and calls `teratts::Engine::synthesize_segments`, which:
- synthesizes each segment with its own language frontend and the shared voice;
- inserts `pauses.segment_ms` between segments (no pause after the last);
- offsets word timestamps by the accumulated audio and adds `text_start` to
  `start_char`/`end_char`;
- returns one WAV and one timestamp list.

Rejected: concatenating per-segment WAVs in the adapter. It would duplicate
WAV parsing/encoding and make timestamp offsets easy to get wrong; the crate
already owns the audio timeline.

### D5. English frontend is the Russian frontend minus accentuation

`text::prepare` takes a `Language`. `Ru` keeps RUAccent dictionary stress and
`<ru>`; `En` skips accentuation and wraps `<en>`. Both run NFKD and the counted
alphabet gate, and both reject caller-provided markup. Chunking, the duration
predictor and the sampler are language-agnostic; the voice selects the style
arrays.

Rejected: a separate English preparation path. The only difference is the
accentizer and the tag; a second path would drift on the gate and NFKD rules.

### D6. Alphanumeric tokens are narrated on every route

A new pipeline phase runs before symbols and numbers and rewrites
`IPv6`/`HTTP/2` (all-caps prefix plus digits, or prefix/slash/digits) through
the abbreviation normalizer: letters by name, digits as words, `/` as
`слэш`. The path normalizer skips a slash preceded by a word character, so
`HTTP/2` is no longer read as a path. The phase runs for `Narrate` and
`Native` profiles, which keeps `IPv6` correct in the RU-MVP path too.

Rejected: protecting native spans from symbol/number phases with a general
"protected ranges" mechanism. It is a larger pipeline change than this
fragment class needs, and the probe did not ask for native abbreviations.

## Risks

| Risk | Mitigation |
|------|------------|
| The selected voice reads a fragment worse than Russian narration | The #311 probe verdict; native limited to prose/identifiers; URL/email/code stay narrated; the user can pick another voice |
| Mixed timestamps drift at segment boundaries | `synthesize_segments` offsets by actual sample counts; bundle-gated mixed test asserts monotonicity and absolute char ranges |
| A Native profile emits mixed-script text | The alphanumeric phase runs on both routes; a pipeline test asserts no `IPv`/`HTTP` remains |
| Other engines accidentally receive spans | `synthesize_spans` defaults to `synthesize`; only TeraTTS overrides it |
| `HTTP/2` mistaken for a path | Path phase skips a slash preceded by a word character; regression test |
