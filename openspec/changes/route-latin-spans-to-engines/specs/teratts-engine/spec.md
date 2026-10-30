# Delta: teratts-engine

## MODIFIED Requirements

### Requirement: Language tags and text validation

The model requires every fragment to be wrapped in a language tag. The adapter
SHALL wrap each language segment in its own top-level tag: `<ru>…</ru>` for
Russian and `<en>…</en>` for English. Tags SHALL be top-level only: nested
tags, an unknown tag name, an unbalanced tag, or a stray `<`/`>` outside a tag
MUST fail with a typed invalid-text error rather than reach the model. The
pipeline SHALL NOT emit tags; wrapping is the adapter's job, and a segment's
language is chosen from its span annotation, never guessed from the text.

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

- GIVEN the text `привет < мир`
- WHEN the adapter validates it
- THEN it fails with a typed invalid-text error

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
`styles/<voice>/` directories. The Russian voices SHALL be `ru_m5` (default),
`ru_f1`, `ru_f2` and `ru_m1`; the English voices SHALL be `eng_f5` (default
for English segments), `eng_m3`, `eng_f3`, `eng_m4`, `eng_f4_whisper` and
`eng_m2_whisper`. Selecting a different voice, including switching between a
Russian and an English voice, SHALL NOT reload the ONNX graphs. An unknown
voice MUST fail with a typed `voice_unknown` error listing the available
voices.

#### Scenario: The default voice is ru_m5

- GIVEN no `teratts_voice` in the config
- WHEN a synthesis runs
- THEN the `ru_m5` style tensors are used

#### Scenario: The default English voice is eng_f5

- GIVEN no `teratts_en_voice` in the config
- WHEN an English segment is synthesized
- THEN the `eng_f5` style tensors are used

#### Scenario: Unknown voice is rejected

- GIVEN the voice id `ru_m9`
- WHEN synthesis is requested
- THEN it fails with `voice_unknown` and the message lists the available voices

#### Scenario: A voice change keeps the graphs loaded

- GIVEN a loaded engine
- WHEN the next synthesis uses a different voice
- THEN no ONNX session is re-created
