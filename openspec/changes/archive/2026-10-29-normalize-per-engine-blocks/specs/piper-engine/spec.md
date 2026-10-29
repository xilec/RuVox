# Delta: piper-engine

## MODIFIED Requirements

### Requirement: Bounded chunking before synthesis

The engine SHALL split the normalized text before inference using the limit
declared by the Piper text profile — currently 500 codepoints, chosen from
peak-memory measurements because Piper's VITS encoder grows activation memory
quadratically with input length. The split SHALL be performed by the shared
splitter (see `text-chunking`), not by an engine-local copy, and the engine
SHALL NOT declare a chunk limit of its own.

The split SHALL prefer, in order: sentence-ending punctuation (`.`, `!`, `?`)
followed by whitespace, then clause punctuation (`,`, `;`, `:`) followed by
whitespace, then any whitespace run; a hard split at the limit SHALL be used
only when the window contains no whitespace at all. A paragraph break (blank
line) SHALL always end the current chunk, even before the limit is reached.
Whitespace between chunks SHALL not be synthesized. Each chunk SHALL be
synthesized by a separate inference call and the audio SHALL be concatenated in
chunk order into a single WAV at the synthesis output path. A chunk whose
inference fails SHALL fail the whole synthesis with that error; chunks already
synthesized SHALL be discarded.

#### Scenario: long text is split on sentence boundaries

- GIVEN a normalized text longer than the per-chunk limit containing
  sentence-ending punctuation
- WHEN the engine synthesizes it
- THEN the text is split into multiple chunks, each within the limit, every
  non-final chunk ends on a sentence boundary, and the concatenated audio is a
  single valid WAV

#### Scenario: paragraph break always ends the chunk

- GIVEN a normalized text shorter than the per-chunk limit containing a blank
  line between two paragraphs
- WHEN the engine synthesizes it
- THEN the text is split at the paragraph break into separate chunks, one per
  paragraph

#### Scenario: text without boundaries is hard-split

- GIVEN a normalized text longer than the per-chunk limit containing no
  sentence or clause punctuation in a split window (e.g. one long token)
- WHEN the engine synthesizes it
- THEN the text is still split into chunks within the limit and synthesis
  completes

#### Scenario: chunk inference failure fails the synthesis

- GIVEN a multi-chunk text where inference fails on some chunk
- WHEN the engine synthesizes the text
- THEN the synthesis fails with that chunk's error and no WAV is written

#### Scenario: limit comes from the profile, not the engine

- GIVEN the Piper profile declares a different codepoint limit
- WHEN the engine synthesizes a long text
- THEN the chunks honour the profile's limit without editing engine code

### Requirement: Paragraph pauses

The engine SHALL insert the silence declared by the Piper text profile
(currently 450 ms) into the audio between two chunks separated by a paragraph
break in the source text (espeak-ng reads blank lines as plain spaces, so the
model alone produces no paragraph pause). The pause SHALL be reflected in the
synthesis duration and in word timestamps, so the audio, the reported duration
and the timestamps stay aligned. A sentence-boundary chunk separation SHALL NOT
get an inserted pause. A language-segment boundary pause SHALL NOT be inserted
while the profile routes every fragment `Narrate`.

#### Scenario: paragraph break produces an audible pause

- GIVEN a normalized text of two paragraphs separated by a blank line
- WHEN the engine synthesizes it
- THEN the audio contains a fixed silence between the paragraphs, the reported
  duration equals the silence plus both paragraphs' audio, and the second
  paragraph's word timestamps start after the silence

#### Scenario: sentence boundary gets no inserted pause

- GIVEN a multi-chunk text whose chunks are separated by single spaces
- WHEN the engine synthesizes it
- THEN no silence is inserted at those chunk boundaries

#### Scenario: pause value comes from the profile

- GIVEN the Piper profile declares a different `paragraph_ms`
- WHEN the engine synthesizes two paragraphs
- THEN the inserted silence equals the profile's value
