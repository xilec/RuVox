# Delta: position-mapping

## MODIFIED Requirements

### Requirement: Span annotation model

Alongside `CharMapping`, the pipeline output SHALL carry span annotations. A
span SHALL be a triple of kind, optional language code and range, where the
range is a half-open `[start, end)` in **output** (transformed) coordinates, in
Unicode codepoints — never bytes and never original-text offsets. The kinds
SHALL be code identifier, prose Latin run, fenced code block and dictionary
override; the language SHALL be present for a native span and absent
otherwise.

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

#### Scenario: Native code block reports its source range

- GIVEN a profile that routes fenced code blocks `Native`, the read mode and
  the input "Текст\n\n```rust\nlet x = 42;\n```"
- WHEN the pipeline returns
- THEN one span of kind code-block with language English covers exactly the
  code text in the output, and every codepoint of that range maps through
  `char_map` into the original fenced block

#### Scenario: Expanded fragment reports a collapsed source

- GIVEN the reference profile and the input "42"
- WHEN the pipeline returns
- THEN the output is the Russian reading and its `char_map` maps every output
  codepoint back to the two original codepoints

#### Scenario: Spans stay sorted and non-overlapping under every profile

- GIVEN every golden fixture, run under every profile in the harness
- WHEN the spans are collected
- THEN the invariants above hold for each result
