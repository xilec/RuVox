# Preview Dialog — Source Format Auto-Detection (HTML boundaries, delta)

## MODIFIED Requirements

### Requirement: Source format auto-detection

The system SHALL classify text into a source format (`plain`, `markdown`, or
`html`) from content signals only, without any configuration or user input:

- `html` — when the trimmed text starts with a `<!DOCTYPE html` or `<html`
  prefix (case-insensitive), or when, after trimming whitespace and
  zero-width characters (`U+200B`–`U+200D`, `U+FEFF`) at both ends, it
  **starts with a well-formed tag AND ends with a well-formed tag** (an
  angle-bracket construct that opens with `<` or `</` followed by a letter,
  may carry attributes, and closes with `>`). Markup is delimited by tags:
  placeholder-like fragments buried in prose never satisfy both boundaries.
- `markdown` — when the text carries at least one **strong structural signal**:
  an ATX heading line (`#`–`######` followed by a space), a fenced code block
  delimiter (``` ``` ```` or `~~~`) on its own line, three or more list-item
  lines (starting with `-`, `*`, `+`, or a numbered `1.`-style marker), or two
  or more inline links (`[text](target)`).
- `plain` — otherwise, and always for empty or whitespace-only text.

The classification SHALL be conservative on the `html` side, because reading
markup aloud is the costlier mistake than under-detecting it: technical prose
with angle brackets (`a < b`, `x -> y`, C++ includes), single generic
parameters (`<T>`), or stray tag-looking fragments in an otherwise plain
text SHALL NOT classify as `html` — such texts do not both start and end
with a tag.

#### Scenario: Full HTML document is detected

- WHEN the text starts with `<!DOCTYPE html>` (or `<html`) and contains markup
- THEN the detected format is `html`

#### Scenario: Full HTML document stays html despite heading-like lines

- GIVEN a text starting with `<!DOCTYPE html>` whose body contains a line
  like `# notes` inside markup
- WHEN the format is detected
- THEN the detected format is `html` — the document prefix outranks the
  markdown signals

#### Scenario: Markup fragment with several tags is detected

- GIVEN the text is not a full document but both starts and ends with
  well-formed tags (e.g. `<p>Первый</p><p>Второй</p><b>третий</b>`)
- WHEN the format is detected
- THEN the detected format is `html`

#### Scenario: Bare tag-pair snippet is detected

- GIVEN a text consisting of a single tag pair with content (e.g.
  `<b>жирным</b>`)
- WHEN the format is detected
- THEN the detected format is `html` — both boundaries are tags

#### Scenario: Changelog-style prose with placeholder fragments stays markdown

- GIVEN a changelog-style document: an ATX heading (`# Changelog`), several
  list-item lines, and fragments such as `` `<type>(<module>): <desc>` ``
  and `` `<UnlistenFn>` ``
- WHEN the format is detected
- THEN the detected format is `markdown` — the text neither starts nor ends
  with a tag, so the markdown structure decides

#### Scenario: Text starting with a tag but not ending with one stays non-html

- GIVEN a text that starts with a tag-like construct but ends with ordinary
  text (e.g. `<T> get_user_data()` or an unclosed fragment
  `<p>раз\n<p>два\n<p>три`)
- WHEN the format is detected
- THEN the detected format is NOT `html` — the end boundary is not a tag

#### Scenario: Angle-bracket prose stays plain

- GIVEN the text is technical prose such as `if a < b && c > d` or
  `Vec<T> get_user_data()`
- WHEN the format is detected
- THEN the detected format is `plain`

#### Scenario: Single stray tag-looking fragment stays plain

- GIVEN a plain paragraph that contains exactly one tag-looking fragment
  (e.g. `<cmath>` in `подключите <cmath> для std::sqrt`)
- WHEN the format is detected
- THEN the detected format is `plain`

#### Scenario: Markdown structural signals are detected

- GIVEN a text with an ATX heading, or a fenced code block, or three or more
  list-item lines, or two or more inline links
- WHEN the format is detected
- THEN the detected format is `markdown`

#### Scenario: Sparse markdown-looking decoration stays plain

- GIVEN a plain paragraph where a single line happens to start with `-` and no
  other structural signal exists
- WHEN the format is detected
- THEN the detected format is `plain`

#### Scenario: Empty text classifies as plain

- GIVEN the text is empty or whitespace-only
- WHEN the format is detected
- THEN the detected format is `plain`
