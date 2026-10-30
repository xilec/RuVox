# text-chunking Specification

## Purpose
TBD - created by archiving change normalize-per-engine-blocks. Update Purpose after archive.
## Requirements
### Requirement: Profile-declared synthesis limits

Every engine SHALL obtain its per-call limit and its pause policy from its text
profile; no engine SHALL carry a chunk limit of its own. The limit SHALL be
expressed either in codepoints of normalized text (`Chars(n)`) or in seconds of
produced audio (`Secs(n)`). The pause policy SHALL declare, in milliseconds,
the silence inserted at a paragraph break and the silence inserted at a
language-segment boundary, where `0` means "no inserted silence".

A `Chars(n)` limit SHALL be passed to the shared splitter directly. A `Secs(n)`
limit SHALL be honored by the engine that declares it, because converting
seconds to a codepoint window needs that engine's duration estimate: the engine
SHALL derive a conservative codepoint budget from a measured chars-per-second
constant, split with the same shared splitter, validate every chunk with its
duration estimator, and re-split a chunk predicted above the engine's hard call
limit at an inner sentence boundary (then a clause boundary, then whitespace,
then a hard split). The shared splitter itself SHALL stay codepoint-based; the
seconds conversion is engine-side behavior, not a second splitter.

#### Scenario: Engines with different limits split by their own numbers

- GIVEN two engines whose profiles declare `Chars(900)` and `Chars(500)`
- WHEN each splits the same long text
- THEN every chunk of the first stays within 900 codepoints and every chunk of
  the second within 500, with no constant shared between them

#### Scenario: No engine-local constant remains

- GIVEN the engine implementations
- WHEN a chunk limit or a pause is read
- THEN it originates from a profile field, and no engine module declares a
  chunk-size or pause constant

#### Scenario: A second-based limit is data, not behavior yet

- GIVEN the shared codepoint splitter
- WHEN it receives a profile limit
- THEN a `Secs(n)` limit is still data the splitter does not interpret; the
  engine that declares it converts it into a codepoint budget

#### Scenario: A second-based limit is honored with a measured budget

- GIVEN a profile that declares `Secs(20)`
- WHEN its engine splits a long text
- THEN the engine derives a codepoint budget from its measured
  chars-per-second constant, splits with the shared splitter, and validates
  every chunk with its duration estimator before synthesis

#### Scenario: An over-budget chunk is re-split

- GIVEN a chunk whose predicted duration exceeds the engine's hard call limit
- WHEN the engine validates the split
- THEN the chunk is split at an inner sentence boundary and each part is
  re-validated before it reaches synthesis

### Requirement: Break preference

When a window of the limit's length must be split, the split point SHALL
prefer, in order: sentence-ending punctuation (`.`, `!`, `?`) followed by
whitespace; clause punctuation (`,`, `;`, `:`) followed by whitespace; any
whitespace run; and only when the window contains no whitespace at all, a hard
split at the limit. A paragraph break (a blank line, `\r` counting as a
newline) SHALL always end the current chunk even before the limit is reached.
Whitespace between chunks SHALL NOT be synthesized.

#### Scenario: Sentence boundary preferred over a clause boundary

- GIVEN a window containing both a clause boundary and a sentence boundary
- WHEN the text is split
- THEN the chunk ends at the sentence boundary

#### Scenario: Paragraph break ends the chunk early

- GIVEN a text shorter than the limit containing a blank line between two
  paragraphs
- WHEN the text is split
- THEN it yields one chunk per paragraph

#### Scenario: Text without boundaries is hard-split

- GIVEN a text longer than the limit whose windows contain no whitespace
- WHEN the text is split
- THEN the text is still split into chunks within the limit

### Requirement: Chunk offsets

Splitting SHALL return, for every chunk, its codepoint offset in the input
text, so that the timestamp estimator can attribute synthesized audio to a
text range. Offsets SHALL be monotonically increasing and non-overlapping, and
the chunks SHALL preserve the order and the content of the input apart from the
whitespace dropped between them.

#### Scenario: Offsets are monotonic and within bounds

- GIVEN any input text
- WHEN it is split
- THEN every offset is strictly greater than the previous one, every chunk
  starts at its offset in the input, and the last chunk ends at the input
  length or earlier

#### Scenario: Whitespace-only input yields no chunks

- GIVEN an input that is empty or contains only whitespace
- WHEN it is split
- THEN no chunks are produced

### Requirement: Single splitter for in-process engines

In-process engines SHALL share one splitter implementation. The Python sidecar
SHALL keep its own split so that its per-chunk timestamp estimation stays
inside the sidecar, but the limit it applies SHALL arrive from the caller
rather than from a constant declared in the sidecar.

#### Scenario: Both in-process engines call the same splitter

- GIVEN the Piper and Silero-native engines
- WHEN either synthesizes a long text
- THEN the split comes from the shared implementation parameterized by that
  engine's profile

#### Scenario: Sidecar applies the caller's limit

- GIVEN a `synthesize` request that carries a chunk limit of 400 codepoints
- WHEN the sidecar synthesizes a longer text
- THEN no chunk exceeds 400 codepoints

