# Delta: text-chunking

## MODIFIED Requirements

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
