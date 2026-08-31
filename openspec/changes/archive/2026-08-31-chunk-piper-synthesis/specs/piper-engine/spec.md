## Purpose

Specifies the in-process Piper TTS engine's long-text behavior: bounded chunking of the
normalized text before ONNX inference, per-chunk synthesis with audio concatenation, chunked
word-timestamp estimation, and cancellation between chunks. Chunking keeps VITS encoder
activation memory bounded — a single unchunked inference over long text grows quadratically and
can freeze the machine (issue #155).

## ADDED Requirements

### Requirement: Bounded chunking before synthesis

The engine SHALL split the normalized text into chunks of at most a fixed per-chunk limit
(codepoints) before inference. When a window of the limit's length must be split, the split
point SHALL prefer, in order: sentence-ending punctuation (`.`, `!`, `?`) followed by
whitespace, then clause punctuation (`,`, `;`, `:`) followed by whitespace, then any whitespace
run; a hard split at the limit SHALL be used only when the window contains no whitespace at
all. A paragraph break (blank line) SHALL always end the current chunk, even before the limit
is reached. Whitespace between chunks SHALL not be synthesized. Each chunk SHALL be
synthesized by a separate inference call and the audio SHALL be concatenated in chunk order
into a single WAV at the synthesis output path. A chunk whose inference fails SHALL fail the
whole synthesis with that error; chunks already synthesized SHALL be discarded.

#### Scenario: long text is split on sentence boundaries

- GIVEN a normalized text longer than the per-chunk limit containing sentence-ending punctuation
- WHEN the engine synthesizes it
- THEN the text is split into multiple chunks, each within the limit, every non-final chunk
  ends on a sentence boundary, and the concatenated audio is a single valid WAV

#### Scenario: paragraph break always ends the chunk

- GIVEN a normalized text shorter than the per-chunk limit containing a blank line between two
  paragraphs
- WHEN the engine synthesizes it
- THEN the text is split at the paragraph break into separate chunks, one per paragraph

#### Scenario: text without boundaries is hard-split

- GIVEN a normalized text longer than the per-chunk limit containing no sentence or clause
  punctuation in a split window (e.g. one long token)
- WHEN the engine synthesizes it
- THEN the text is still split into chunks within the limit and synthesis completes

#### Scenario: chunk inference failure fails the synthesis

- GIVEN a multi-chunk text where inference fails on some chunk
- WHEN the engine synthesizes the text
- THEN the synthesis fails with that chunk's error and no WAV is written

### Requirement: Paragraph pauses

The engine SHALL insert a fixed silence pause into the audio between two chunks separated by a
paragraph break in the source text (espeak-ng reads blank lines as plain spaces, so the model
alone produces no paragraph pause). The pause SHALL be reflected in the synthesis duration and
in word timestamps, so the audio, the reported duration and the timestamps stay aligned. A
sentence-boundary chunk separation SHALL NOT get an inserted pause.

#### Scenario: paragraph break produces an audible pause

- GIVEN a normalized text of two paragraphs separated by a blank line
- WHEN the engine synthesizes it
- THEN the audio contains a fixed silence between the paragraphs, the reported duration equals
  the silence plus both paragraphs' audio, and the second paragraph's word timestamps start
  after the silence

#### Scenario: sentence boundary gets no inserted pause

- GIVEN a multi-chunk text whose chunks are separated by single spaces
- WHEN the engine synthesizes it
- THEN no silence is inserted at those chunk boundaries

### Requirement: Chunked word timestamps

Word timestamps SHALL be estimated per chunk over that chunk's audio duration and shifted by
the accumulated duration of all preceding chunks, so timestamps are monotonically non-decreasing
across chunk boundaries and contiguous with the concatenated audio. Word positions SHALL be
offsets into the full normalized text (codepoint units), so the pipeline char mapping maps them
to original-text offsets exactly as single-chunk timestamps do.

#### Scenario: timestamps advance monotonically across chunk boundaries

- GIVEN a multi-chunk synthesis
- WHEN word timestamps are estimated
- THEN every word's start is at or after the previous word's end, and the first word after a
  chunk boundary starts at or after the accumulated duration of the preceding chunks

#### Scenario: timestamps map through char mapping

- GIVEN a multi-chunk synthesis with a pipeline char mapping
- WHEN word timestamps are estimated
- THEN each word's `original_pos` is the mapped original-text span for its full-normalized-text
  offsets, same as the single-chunk path

### Requirement: Cancellation between chunks

The engine SHALL support cancellation of in-flight synthesis between chunks: a cancellation
request SHALL cause the chunk loop to stop before the next chunk inference with a typed
cancellation error. A cancelled synthesis SHALL NOT write any WAV at the output path, and the
model lock SHALL be released promptly so the next synthesis can start.

#### Scenario: cancellation stops before the next chunk

- GIVEN a multi-chunk synthesis in progress
- WHEN cancellation is requested while chunk k is being synthesized
- THEN no chunk after k starts inference, the synthesis ends with a cancellation error, and
  the next synthesis attempt can acquire the model immediately after

#### Scenario: cancelled synthesis writes no audio

- GIVEN a multi-chunk synthesis that is cancelled mid-way
- WHEN the cancellation completes
- THEN no WAV file exists at the synthesis output path
