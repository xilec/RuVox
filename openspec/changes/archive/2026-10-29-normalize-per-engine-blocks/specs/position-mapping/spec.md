# Delta: position-mapping

## ADDED Requirements

### Requirement: Span annotation model

Alongside `CharMapping`, the pipeline output SHALL carry span annotations. A
span SHALL be a triple of kind, optional language code and range, where the
range is a half-open `[start, end)` in **output** (transformed) coordinates, in
Unicode codepoints — never bytes and never original-text offsets. The kinds
SHALL be code identifier, prose Latin run and dictionary override; the language
SHALL be present for a native span and absent otherwise.

Span invariants, asserted for every profile:

- `start <= end` and `end <= transformed.chars().count()`;
- spans are sorted by `start`;
- spans do not overlap;
- every span with `start < end` intersects the transformed text, and a span
  whose fragment disappeared is not reported at all;
- a span with `start == end` SHALL NOT be reported.

Spans SHALL NOT alter the `CharMapping` data model: `char_map.len()` still
equals the codepoint count of `transformed`, whatever the profile.

#### Scenario: Unchanged text yields no spans

- GIVEN the input "привет мир" processed with any profile
- WHEN the pipeline returns
- THEN no span is produced and `char_map` is the identity mapping

#### Scenario: Native run maps to its own output range

- GIVEN a profile that routes prose Latin runs `Native` and the input
  "Kubernetes нужен"
- WHEN the pipeline returns
- THEN one span of kind prose-run with language English covers exactly the
  codepoints of "Kubernetes" in the output

#### Scenario: Expanded fragment reports a collapsed source

- GIVEN the reference profile and the input "42"
- WHEN the pipeline returns
- THEN the output is the Russian reading and its `char_map` maps every output
  codepoint back to the two original codepoints

#### Scenario: Spans stay sorted and non-overlapping under every profile

- GIVEN every golden fixture, run under every profile in the harness
- WHEN the spans are collected
- THEN the invariants above hold for each result

### Requirement: Span resolution from original coordinates

A phase that detects a fragment SHALL record it as an original-text range,
converted at detection time through the current mapping (the mapping is
transformed → original, so the conversion is a direct lookup). The final
resolution SHALL be a single pass over the output: for each output codepoint
its original range is known, and that range extending a span widens the span's
output range. No reverse index, no string search and no second range
bookkeeping SHALL be introduced.

A span whose original range no longer maps to any output codepoint — because
later phases deleted the fragment — SHALL be dropped.

#### Scenario: Fragment deleted by a later phase is not reported

- GIVEN a fragment detected as a span and then replaced by a narrating phase
  that emits nothing for it
- WHEN the spans are resolved
- THEN the span is absent from the output and no error is raised

#### Scenario: Resolution never touches the text

- GIVEN any input and profile
- WHEN spans are resolved
- THEN the transformed text is unchanged by the resolution step

#### Scenario: One pass over the output

- GIVEN a 1 MB input with spans
- WHEN the pipeline resolves them
- THEN the resolution cost is linear in the output length and does not
  materialize a per-codepoint reverse index
