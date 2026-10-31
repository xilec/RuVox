# teratts-engine Specification

## Purpose
TBD - created by archiving change add-teratts-engine. Update Purpose after archive.
## Requirements
### Requirement: Model bundle format and verification

The engine SHALL load from a bundle directory containing `manifest.json`,
`unicode_indexer.json`, the four ONNX graphs
(`models/text_encoder.onnx`, `models/duration_predictor.onnx`,
`models/sampler_distilled_cfg3_8step.onnx`, `models/vocoder.onnx`), the voice
styles (`styles/<voice>/style_ttl.npy` and `style_dp.npy`), the RUAccent
dictionary files (`ruaccent/dictionary/accents.json.gz`,
`omographs.json.gz`, `yo_words.json.gz`, `yo_homographs.json.gz`), and the
license files (`LICENSE`, `LICENSE-OPENRAIL`, `RUACCENT_NOTICE.txt`).

`manifest.json` SHALL contain: `format_version`, `bundle_version`, `model_id`,
`model_version`, `upstream_repo`, `upstream_revision`, `opset`, `sample_rate`,
`created_at_utc`, `files[]` with `path`, `asset`, `size` and `sha256` per
file, and `voices[]` with `id` and `language` per voice. `asset` SHALL be the
flat GitHub release asset name, because release assets cannot contain `/`;
`path` SHALL be the bundle-relative location.

Before opening any ONNX session the loader SHALL verify every manifest entry:
the file exists, its size equals `size`, and its sha256 equals `sha256`. A
mismatch MUST fail loading with a typed error naming the file; a corrupt or
partial bundle MUST never reach ONNX Runtime. A `format_version` the loader
does not support MUST fail with a distinct typed error. The loader MUST NOT
download or repair anything.

#### Scenario: Valid bundle loads

- GIVEN a bundle whose files match the manifest sizes and checksums
- WHEN the engine loads it
- THEN the four sessions initialize and the engine reports the manifest
  `model_id` and `bundle_version`

#### Scenario: Corrupted file is rejected

- GIVEN a bundle where `models/vocoder.onnx` does not match its manifest
  sha256
- WHEN the engine loads it
- THEN loading fails with an error naming that file and no session is opened

#### Scenario: Truncated file is rejected without hashing

- GIVEN a bundle where a file's size differs from the manifest
- WHEN the engine loads it
- THEN loading fails before the file is hashed

#### Scenario: Unsupported format version

- GIVEN a manifest with `format_version: 2`
- WHEN the engine loads it
- THEN loading fails with an incompatible-bundle error

#### Scenario: Missing bundle

- GIVEN a directory with no `manifest.json`
- WHEN the engine loads it
- THEN loading fails with a bundle-not-installed error and the availability
  probe reports the engine unavailable

### Requirement: Inference and synthesis parameters

The engine SHALL run the four graphs on the CPU execution provider with the
repository's `ort`/`ort-sys` pins. It SHALL encode text character-by-character
through `unicode_indexer.json` after NFKD, predict the utterance duration with
`duration_predictor`, draw the initial latent `[1, 144, T]` from a seeded
PRNG, run the distilled sampler (8 Euler steps, CFG 3 baked in, `guidance`
passed as 3.0), decode the latent with the causal vocoder in 16-frame chunks
with 20 frames of left context, and trim the waveform to
`round(duration_seconds * 44100)` samples.

`duration_seconds` SHALL be `predictor_output * duration_scale / 1.05`, with
`duration_scale` defaulting to 0.9 (the measured optimum). Intra-op threads
SHALL default to physical-core scale (`available_parallelism / 2`, at least
1) and SHALL be overridable for tests. The seed SHALL be fixed per call, so
the same bundle, text, voice and parameters produce the same WAV on the same
ONNX Runtime build.

#### Scenario: Latent length follows the predicted duration

- GIVEN a text whose predicted duration is `d` seconds
- WHEN the latent is built
- THEN its frame count is `ceil(d * 44100 / 3072)` and the output is trimmed
  to `round(d * 44100)` samples

#### Scenario: Same input and seed reproduce the audio

- GIVEN the same bundle, text, voice and seed
- WHEN the text is synthesized twice
- THEN the two WAV files are identical on the same ONNX Runtime build

#### Scenario: Vocoder chunks match a full decode

- GIVEN a latent of at least 40 frames
- WHEN it is decoded in 16-frame chunks and in one call
- THEN the two waveforms match within a small tolerance and the chunked
  output has no duplicated or missing samples

#### Scenario: Non-positive duration is rejected

- GIVEN a duration predictor output of zero or a negative value
- WHEN synthesis runs
- THEN it fails with a typed synthesis error and writes no WAV

### Requirement: Language tags and text validation

The model requires every fragment to be wrapped in a language tag. The adapter
SHALL wrap each language segment in its own top-level tag: `<ru>…</ru>` for
Russian and `<en>…</en>` for English. Tags SHALL be top-level only: nested
tags, an unknown tag name, an unbalanced tag, or a stray `<`/`>` outside a tag
MUST fail with a typed invalid-text error rather than reach the model. The
pipeline SHALL NOT emit tags; wrapping is the adapter's job, and a segment's
language is chosen from its span annotation, never guessed from the text.

A native code-block span is the one place where segment text may legitimately
contain the tag delimiters. The adapter SHALL replace `<` and `>` inside a
code-block span with spaces before wrapping, because code may contain generics
or comparisons (`Vec<T>`, `a < b`) and the model interprets those characters
only as tag syntax. The codepoint count SHALL stay unchanged so spans, word
timestamps and the `CharMapping` remain valid, and the substitution SHALL be
limited to code-block spans: every other stray `<`/`>` still fails with the
typed error.

#### Scenario: Russian text is wrapped

- GIVEN the normalized text "привет мир"
- WHEN the adapter prepares it for synthesis
- THEN the model input is `<ru>привет мир</ru>`

#### Scenario: English segment is wrapped

- GIVEN a segment with language `en` and text "Kubernetes"
- WHEN the adapter prepares it for synthesis
- THEN the model input is `<en>Kubernetes</en>`

#### Scenario: Nested tags are rejected

- GIVEN the text `<ru>привет <en>world</en></ru>`
- WHEN the adapter validates it
- THEN it fails with a typed invalid-text error

#### Scenario: Stray angle bracket is rejected

- GIVEN a Russian text `привет < мир`
- WHEN the adapter validates it
- THEN it fails with a typed invalid-text error

#### Scenario: Angle brackets in native code are neutralised

- GIVEN a normalized text with a code-block span whose text is `Vec<T>`
- WHEN the adapter prepares the segments for synthesis
- THEN the English segment reads `Vec T` inside the `<en>` tag, its codepoint
  count is unchanged, and synthesis does not fail

### Requirement: Russian accentuation through RUAccent dictionary mode

The adapter SHALL accentuate Russian segments with the bundled RUAccent
dictionary mode: load the four gzip dictionaries, apply `yo_words` and `accents`
word-by-word to `<ru>` spans, preserve the source casing, and leave a word
unchanged when no entry exists. It SHALL NOT load or call the RUAccent neural
models. A word that already carries an explicit `+` SHALL be left untouched —
the mark is authoritative and MUST NOT be duplicated. Homographs that only the
full neural mode resolves (`замок`) SHALL remain unresolved; this limitation
SHALL be documented. English segments SHALL bypass accentuation entirely: the
English frontend SHALL NOT add or remove stress marks.

#### Scenario: A dictionary word receives stress

- GIVEN the word "молоко" present in `accents.json.gz`
- WHEN the adapter accentuates it
- THEN the output carries the dictionary's `+` position

#### Scenario: Explicit stress wins

- GIVEN the word "молок+о" with an explicit mark
- WHEN the adapter accentuates the span
- THEN the word is unchanged and carries exactly one `+`

#### Scenario: Unknown word stays untouched

- GIVEN a word absent from every dictionary
- WHEN the adapter accentuates the span
- THEN the word is returned unchanged

#### Scenario: `ё` words are applied

- GIVEN a word present in `yo_words.json.gz`
- WHEN the adapter accentuates the span
- THEN the word is returned in its `ё` spelling with the dictionary stress

#### Scenario: English segment is not accentuated

- GIVEN an English segment "Kubernetes cluster"
- WHEN the adapter prepares it
- THEN the text is unchanged and contains no `+` marks

### Requirement: Unsupported-codepoint gate

The adapter SHALL normalize text to NFKD and drop every codepoint absent from
`unicode_indexer.json`, emitting one counted `tracing::warn` that lists the
dropped codepoints and their count. The gate SHALL NOT fail synthesis for
ordinary punctuation. A digit that survives the pipeline SHALL be treated as
a normalization gap: it is dropped by the same gate and the warning SHALL name
it explicitly, because the model cannot read digits and the pipeline owns
number expansion.

#### Scenario: Unsupported punctuation is dropped with a warning

- GIVEN a text containing an em dash absent from the indexer
- WHEN the adapter prepares the text
- THEN the dash is removed, the rest is unchanged, and one warning names it

#### Scenario: Supported text passes unchanged

- GIVEN Russian letters, digits-free punctuation and spaces present in the
  indexer
- WHEN the adapter prepares the text
- THEN no codepoint is dropped and no warning is emitted

#### Scenario: A surviving digit is reported as a normalization gap

- GIVEN a text that still contains the digit `5` after normalization
- WHEN the adapter prepares the text
- THEN the digit is dropped and the warning names it as a normalization gap

### Requirement: No double normalization

The adapter SHALL NOT expand numbers, transliterate Latin text, substitute
symbols or units, or run `num2words`. Those decisions belong to the pipeline
and are declared by the text profile (`digits: Expanded`, `units: Expanded`,
and the per-kind fragment routes, which may be `Native`). The only text
transformations the adapter performs are tag wrapping, neutralising tag
delimiters inside a native code-block span, RUAccent dictionary accentuation,
NFKD and the codepoint gate.

#### Scenario: Expanded numbers are not expanded again

- GIVEN the pipeline output "двадцать пять процентов"
- WHEN the adapter prepares it
- THEN the text still reads "двадцать пять процентов" and contains no digits

#### Scenario: Transliterated Latin is not transliterated again

- GIVEN the pipeline output "эй пи ай"
- WHEN the adapter prepares it
- THEN the text is unchanged apart from tags and stress marks

#### Scenario: Native Latin is not transliterated

- GIVEN the pipeline output "Kubernetes" in an English segment
- WHEN the adapter prepares it
- THEN the text is unchanged apart from tags and stress marks

### Requirement: Chunking under the seconds limit

The engine SHALL honor its profile's `Limit::Secs(20)` by deriving a
conservative codepoint budget from a measured chars-per-second constant,
splitting with the shared splitter, and validating every chunk with the
duration predictor. A chunk predicted above the model's hard 25-second call
limit SHALL be re-split at the last sentence boundary inside it (then a clause
boundary, then whitespace, then a hard split) and re-validated. The engine
SHALL insert the profile's paragraph pause (250 ms) after a chunk that ended at
a paragraph break and the segment pause (200 ms) between chunks, and SHALL
check cancellation before each chunk.

#### Scenario: A long article is split into bounded chunks

- GIVEN a multi-paragraph text whose single-call duration would exceed 25 s
- WHEN it is synthesized
- THEN every chunk is predicted at or below 25 s and the chunks cover the text
  in order without dropping content

#### Scenario: An over-budget chunk is re-split

- GIVEN one sentence whose predicted duration exceeds 25 s
- WHEN the chunker validates it
- THEN it is split at an inner boundary and each part is re-validated

#### Scenario: Pauses are inserted between chunks

- GIVEN two consecutive chunks without a paragraph break
- WHEN their audio is joined
- THEN 200 ms of silence separates them; after a paragraph break the pause is
  250 ms

#### Scenario: Cancellation stops between chunks

- GIVEN a synthesis in progress with several chunks
- WHEN cancellation is requested after the first chunk
- THEN synthesis stops with a cancellation error, later chunks are not
  synthesized, and no output WAV is written

### Requirement: Synthesis output contract and model identity

`TtsEngine::synthesize` SHALL write a mono 44 100 Hz PCM16 WAV to `out_wav`
and return `duration_sec` equal to the actual audio duration. The shared
`sample_rate` config value SHALL be ignored by the engine, as Piper already
ignores it; the Opus encoder resamples 44.1 → 48 kHz on its existing off-list
path. `model_info` SHALL return the manifest `model_id` with the
`bundle_version` and the sha256 of `manifest.json` (a small file, safe to hash
at load time).

#### Scenario: WAV properties match the model

- GIVEN a successful synthesis
- WHEN the output file is inspected
- THEN it is mono, PCM16, 44 100 Hz, and its duration equals the returned
  `duration_sec` within one sample

#### Scenario: The configured sample rate does not change the file

- GIVEN `sample_rate: 24000` in the config
- WHEN TeraTTS synthesizes
- THEN the WAV is still 44 100 Hz

#### Scenario: Model identity is reported

- GIVEN a loaded bundle with `model_id: "terattsv2-distilled"` and
  `bundle_version: 1`
- WHEN `model_info` is queried
- THEN the name contains both values and the sha256 is the manifest's

### Requirement: Approximate word timestamps

The engine SHALL return word timestamps with exact sentence boundaries and
approximate word boundaries: each chunk is sentence-bounded, so its audio
range is exact; inside a chunk, the duration predictor is run on cumulative
word prefixes, the predicted boundaries are scaled to the actual chunk
duration, and the timeline is shifted past leading silence. The predictor's
cumulative values SHALL be converted to non-negative per-word durations, and
every word SHALL receive a minimum share of the chunk so no interval collapses
to zero length. Timestamps SHALL be sorted, monotonic, non-overlapping, with
`start < end` for every word, and their last `end` SHALL be at most the audio
duration. Word ranges SHALL be in normalized-text coordinates so the
application can map them to the original document through `char_mapping`. For a
mixed-language utterance, segment offsets SHALL be applied so the list remains
one utterance-wide timeline. The approximation SHALL be documented: pauses are
attributed to the preceding word, so word boundaries may lag an ASR reference
by 0.2–0.4 s.

#### Scenario: Sentence boundaries are exact

- GIVEN a two-sentence text synthesized as two chunks
- WHEN the timestamps are inspected
- THEN the first sentence's words all end before the second sentence's first
  word starts, at the chunk boundary

#### Scenario: Timestamps are monotonic and inside the audio

- GIVEN any synthesized text
- WHEN the timestamps are inspected
- THEN `start < end` for every word, each `start` is at least the previous
  `end`, and the last `end` does not exceed the duration

#### Scenario: Prefix prediction keeps punctuation

- GIVEN a sentence with commas and a colon
- WHEN its word prefixes are measured
- THEN every prefix preserves the punctuation of the sentence prefix

#### Scenario: Mixed-language segments share one timeline

- GIVEN Russian, English and Russian segments synthesized in order
- WHEN the timestamps are inspected
- THEN each segment's words are offset by the audio and codepoint position
  already produced, and the whole list stays monotonic with `start < end`

### Requirement: Voices and defaults

The voice catalogue SHALL come from the manifest `voices[]` and the
`styles/<voice>/` directories. The voices SHALL be `ru_m5` (default), `ru_f1`,
`ru_f2`, `ru_m1`, `eng_f5`, `eng_m3`, `eng_f3`, `eng_m4`, `eng_f4_whisper`
and `eng_m2_whisper`. Every voice SHALL be usable for both languages: a
segment's language selects the tag and whether RUAccent runs, never the voice.
Selecting a different voice SHALL NOT reload the ONNX graphs. An unknown voice
MUST fail with a typed `voice_unknown` error listing the available voices.

#### Scenario: The default voice is ru_m5

- GIVEN no `teratts_voice` in the config
- WHEN a synthesis runs
- THEN the `ru_m5` style tensors are used

#### Scenario: Unknown voice is rejected

- GIVEN the voice id `ru_m9`
- WHEN synthesis is requested
- THEN it fails with `voice_unknown` and the message lists the available voices

#### Scenario: The selected voice reads an English segment

- GIVEN the configured voice `ru_m5`
- WHEN an English segment is synthesized
- THEN the `ru_m5` style tensors are used, not an English voice

#### Scenario: A voice change keeps the graphs loaded

- GIVEN a loaded engine
- WHEN the next synthesis uses a different voice
- THEN no ONNX session is re-created

### Requirement: Typed errors

The engine SHALL report failures as typed errors with stable machine-readable
codes and Russian user-facing messages: `bundle_not_installed`,
`bundle_corrupt`, `bundle_incompatible`, `voice_unknown`, `text_invalid_tags`,
`synthesis_failed` and `teratts_cancelled`. ONNX Runtime failures SHALL be
mapped to `synthesis_failed` with the underlying message; no panic SHALL cross
the engine boundary.

#### Scenario: Each failure carries its code

- GIVEN each of the failure conditions above
- WHEN the engine reports it
- THEN the error carries the matching code and a Russian message

### Requirement: Licensing and attribution

The bundle SHALL ship `LICENSE`, `LICENSE-OPENRAIL` and
`RUACCENT_NOTICE.txt`. The repository SHALL ship `teratts/NOTICE` identifying
the upstream model, the fine-tuning provenance, the modifications made for the
bundle, and the use-based restrictions. `README.md` and `README.en.md` SHALL
carry the commercial attribution "Text-to-speech powered by TeraTTSv2 by
TeraSpace" with a link, and an accurate license section: the default engine
stays non-commercial, TeraTTSv2 is the commercially usable option under
OpenRAIL-M, and RUAccent stays MIT with its notice.

#### Scenario: License files are present in the bundle

- GIVEN a downloaded bundle
- WHEN its root is listed
- THEN `LICENSE`, `LICENSE-OPENRAIL` and `RUACCENT_NOTICE.txt` exist

#### Scenario: Attribution is present in both READMEs

- GIVEN `README.md` and `README.en.md`
- WHEN the attribution string is searched
- THEN both contain it with a link to the TeraSpace model
