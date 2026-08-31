## MODIFIED Requirements

### Requirement: Fenced code blocks

The system SHALL process fenced code blocks (` ```lang ... ``` `) before all
other content phases. A block tagged `mermaid` SHALL be replaced with the
exact marker string "Тут мермэйд диаграмма" and MUST NOT be read aloud. All
other fenced blocks SHALL be narrated according to the pipeline's code block
narration mode, which the application sets from the `code_block_mode` config
value (`"brief"` default, `"read"` optional; the wiring lives in the
`ipc-commands` capability):

- **Brief** (`"brief"`): the block is replaced with "далее следует пример
  кода на <язык>" (language looked up in `LANGUAGE_NAMES`) or "далее следует
  блок кода" when no language tag is present.
- **Full** (`"read"`): block contents are tokenized and read aloud with
  identifiers, operators, brackets, and integer literals normalized to spoken
  Russian.

The pipeline's implicit default mode SHALL be Brief, matching the config
default, so a freshly constructed pipeline never contradicts the product
default. The former inline mode-switch directives (`<!-- ruvox-code: … -->`)
SHALL no longer be interpreted: such comments are processed as ordinary text
by the symbol phases exactly like any other punctuation, and the narration
mode is owned solely by the setting.

#### Scenario: Mermaid block replaced with marker

- GIVEN the input "```mermaid\ngraph TD\nA-->B\n```"
- WHEN the pipeline processes it in any mode
- THEN the entire block is replaced with "Тут мермэйд диаграмма"

#### Scenario: Brief mode is the default

- GIVEN a pipeline constructed without an explicit mode and the input
  "```python\nprint('hi')\n```"
- WHEN the pipeline processes it
- THEN the block is replaced with "далее следует пример кода на пайтон"

#### Scenario: Brief mode without a language tag

- GIVEN the pipeline in Brief mode and the input "```\nx = 1\n```"
- WHEN the pipeline processes it
- THEN the block is replaced with "далее следует блок кода"

#### Scenario: Code block read in full mode

- GIVEN the pipeline configured in Full mode and the input
  "```python\nprint('hi')\n```"
- WHEN the pipeline processes it
- THEN the block content is read as "принт открывающая скобка хи
  закрывающая скобка"

#### Scenario: Mode-switch directive

- GIVEN the pipeline in Full mode and the text
  "<!-- ruvox-code: brief -->\n```python\nprint('hi')\n```"
- WHEN the pipeline processes it
- THEN the block is still read in full (the directive comment does not change
  the mode) and the comment is normalized as ordinary text; the exact output
  is pinned by a golden fixture

### Requirement: Golden regression fixtures

The system's pipeline behavior SHALL be pinned by golden fixtures in
`src-tauri/tests/fixtures/pipeline/`: each case consists of
`<case>.input.txt`, `<case>.expected.txt`, and `<case>.char_map.json`, and
the suite SHALL be executed by
`cargo test --manifest-path src-tauri/Cargo.toml --test golden`. The fixtures
SHALL cover numbers, sizes, durations, versions, dates, times, ranges,
percentages, English words, abbreviations, code identifier styles,
URLs/emails/IPs/paths, Markdown constructs, code blocks in both narration
modes (brief by default and full via an explicitly set mode), mermaid,
symbols, operators, whitespace handling, and mixed paragraphs.

#### Scenario: Golden suite passes

- GIVEN the current pipeline implementation
- WHEN `cargo test --manifest-path src-tauri/Cargo.toml --test golden` runs
- THEN every fixture's pipeline output matches its `.expected.txt` and its
  `CharMapping` matches `.char_map.json`
