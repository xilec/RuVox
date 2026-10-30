# Spec Delta

## Purpose

Define how one utterance that mixes Russian and English is synthesized when the
active engine reads both languages: which fragments the pipeline routes to the
engine, how the engine is called for each language segment, how the segments
join into one audio file, and how the resulting word timestamps stay in one
timeline and in original-document coordinates.

## ADDED Requirements

### Requirement: Language segment contract

A mixed-language utterance SHALL be represented as an ordered list of
`Segment`s. Each segment SHALL carry its text, its language, its voice id, and
its codepoint offset in the normalized utterance. Segments SHALL be derived
from the pipeline's span annotations: an English span becomes an English
segment, and every gap between, before or after spans becomes a Russian
segment. A span without a language (a dictionary override) SHALL stay Russian.
Empty and whitespace-only segments SHALL be skipped without shifting the
offsets of later segments.

Segments SHALL be non-overlapping and SHALL cover the normalized text in
order, so concatenating their text (with the skipped whitespace) reconstructs
the utterance.

#### Scenario: Prose span splits the utterance into three segments

- GIVEN the normalized text `Привет Kubernetes мир` with one English prose span over `Kubernetes`
- WHEN the adapter builds segments
- THEN it produces Russian `Привет ` (offset 0), English `Kubernetes` (offset 7) and Russian ` мир` (offset 17)

#### Scenario: No spans means one Russian segment

- GIVEN a normalized text with no span annotations
- WHEN the adapter builds segments
- THEN it produces exactly one Russian segment covering the whole text

#### Scenario: Dictionary override stays Russian

- GIVEN a dictionary-override span with no language
- WHEN the adapter builds segments
- THEN that range is synthesized by the Russian voice

### Requirement: Native routing is per engine profile

The pipeline SHALL route a fragment to the engine only when the active
engine's text profile marks that fragment kind `Native`. The TeraTTS profile
SHALL route Latin prose and code identifiers `Native`; every other engine
profile SHALL keep narrating both. Numbers and units SHALL stay expanded,
and URLs, emails and fenced code blocks SHALL stay narrated, for every
profile including TeraTTS.

A `Native` fragment SHALL NOT be rewritten by a narrating phase, and a
narrated fragment SHALL NOT be handed to the English voice.

#### Scenario: TeraTTS reads prose and identifiers natively

- GIVEN the TeraTTS profile
- WHEN the pipeline processes `Kubernetes и getUserData()`
- THEN both Latin fragments stay verbatim and carry English spans

#### Scenario: Russian-only engines keep narrating

- GIVEN the Silero, Silero Native or Piper profile
- WHEN the pipeline processes the same text
- THEN the Latin fragments are transliterated and no English span is produced

#### Scenario: Structural fragments stay narrated

- GIVEN the TeraTTS profile
- WHEN the pipeline processes a URL, an email and a fenced code block
- THEN all three are narrated in Russian and carry no English span

### Requirement: Segment synthesis and joining

The engine SHALL synthesize each language segment with its own language
frontend and its own voice, and SHALL concatenate the segment audio into one
44.1 kHz mono WAV. Segments SHALL be separated by the profile's segment pause
(200 ms for TeraTTS) and SHALL NOT have a pause appended after the last
segment. Chunking SHALL apply inside a segment, never across segments.

Cancellation SHALL be checked before each segment and before the WAV is
produced; a cancelled call SHALL return a typed cancellation error and SHALL
NOT write a partial file.

#### Scenario: Mixed utterance produces one file

- GIVEN the segments `Привет. ` (Russian), `Kubernetes cluster` (English) and ` готов.` (Russian)
- WHEN the engine synthesizes them
- THEN one WAV contains all three, with 200 ms of silence between them and none after the last

#### Scenario: Each segment uses its own voice

- GIVEN `ru_m5` for Russian and `eng_f5` for English
- WHEN a mixed utterance is synthesized
- THEN the Russian segments use the Russian style tensors and the English segment uses the English ones

#### Scenario: Cancellation leaves no file

- GIVEN a synthesis cancelled between segments
- WHEN the engine returns
- THEN the error is the typed cancellation and no WAV was written

### Requirement: Unified timestamp timeline

Word timestamps from all segments SHALL be returned as one list in utterance
order. Each timestamp's audio `start`/`end` SHALL be offset by the audio
already produced before its segment; its `start_char`/`end_char` SHALL be
offset by the segment's codepoint start. The list SHALL be sorted,
non-overlapping, and every word SHALL have `start < end`. Timestamps SHALL be
in normalized-text coordinates, so the application maps them through the
existing `char_mapping` to original-document offsets.

#### Scenario: English word keeps its absolute position

- GIVEN a mixed utterance with `Kubernetes` starting at codepoint 14
- WHEN the timestamps are inspected
- THEN the `Kubernetes` timestamp starts after the preceding Russian audio and its character range starts at 14

#### Scenario: Highlight crosses the language boundary

- GIVEN a mixed entry with timestamps from both voices
- WHEN playback moves from a Russian word to the English word
- THEN the active word advances without a gap, overlap or reset

### Requirement: English voice configuration

The configuration SHALL carry a separate English TeraTTS voice id,
`teratts_en_voice`, defaulting to `eng_f5`. The Settings dialog SHALL offer the
six English voices from the bundle manifest (`eng_f5`, `eng_m3`, `eng_f3`,
`eng_m4`, `eng_f4_whisper`, `eng_m2_whisper`) and SHALL persist the choice
through `updateConfig`. Changing the English voice SHALL NOT reload the ONNX
graphs. An unknown English voice SHALL fail with the typed `voice_unknown`
error.

#### Scenario: Default English voice

- GIVEN no `teratts_en_voice` in the config
- WHEN a mixed utterance is synthesized
- THEN the English segments use `eng_f5`

#### Scenario: English voice round-trips

- GIVEN a configuration with `teratts_en_voice: "eng_m3"`
- WHEN it is saved and loaded again
- THEN the loaded value is `eng_m3`

#### Scenario: Unknown English voice is rejected

- GIVEN `teratts_en_voice: "eng_m9"`
- WHEN a mixed utterance is synthesized
- THEN it fails with `voice_unknown` and the message lists the available voices
