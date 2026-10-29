# Design: dictionary stress marks

## Context

- `DictionaryEntry` is `{ from, to }`, stored verbatim in a TOML table and
  inserted into the text by the user-dictionary pre-pass (phase 15.5), which
  marks the region as replaced so later phases skip it.
- `TextProfile::stress` exists as data (`None | Auto | Manual`) with no
  consumer; every profile says `None` after the `normalize-per-engine-blocks`
  change, which descoped the mark and left the carrier question open.
- `silero-native`'s accentor (`src/frontend/accentor.rs`) already treats a `+`
  in the raw word as a user mark: it moves the mark out, applies its own
  accentuation, and restores the user's mark at its original position. That is
  exactly the `Auto` semantics the enum describes.
- The next engine (#308 TeraTTSv2) documents that it preserves explicit `+`
  marks from the pipeline, so the mark has to come from somewhere before that
  engine lands.

## Decisions

### D1. The mark lives verbatim in the file; the spoken form strips it

`to` keeps the mark as typed (`Москва = "М+осква"`). Parsing is a cheap
function over the string, not a stored field: the file stays the source of
truth, export/import is byte-identical by construction, and a hand-edited file
behaves exactly like an entry created in the UI.

- *Alternative — store `to` without the mark plus a position field:* rejected.
  It creates two representations to keep in sync, makes the TOML writer
  responsible for re-inserting the mark, and gains nothing: the mark is one
  character in a string that is already user-facing.
- *Alternative — a separate TOML key (`stress = 2`):* rejected. The file is
  meant to be hand-editable, and `М+осква` is what the user thinks.

### D2. The pipeline renders the mark, gated by the profile

The substitution site is the only place that knows where the mark belongs,
which is why the archived change's "the adapter renders it" could not be
implemented: a span carries kind, language and range, not an offset inside a
replacement. So the pipeline inserts the entry's `to` **with** the mark when
the active profile declares stress support, and **without** it otherwise.

The mark is inserted as part of the replacement region, so the symbol,
operator, number and English phases never see it; `CharMapping` maps it to the
original word like every other inserted character. The adapter does not strip
`+`, which is what the "engine supports stress" declaration means.

- *Alternative — keep the mark in the text for every engine and let adapters
  strip it:* rejected. Engines without stress support would read `+` as
  "плюс", and each adapter would need to learn about the mark.
- *Alternative — extend the span model with a stress offset:* rejected. The
  pipeline knows the answer at substitution time; adding metadata to a span
  only to undo the substitution later is more machinery for the same result.

### D3. `Auto` for the native Silero engine, `None` until probed

`silero-native` declares `Stress::Auto`: the accentor predicts stress and an
explicit user mark vetoes it — the enum's definition. `ttsd` (Silero v5 via
Python) and Piper stay `None`: the Python frontend's handling of `+` is not
verified, Piper's espeak reads it as "plus", and #311 is the change that
listens and flips them. The test profile (`native_en`) declares `Manual`, so
the explicit-only path is exercised by the golden harness.

### D4. The editor explains the syntax and validates it, the backend decides

The form validates the mark client-side (one `+`, immediately before a vowel)
for instant feedback, exactly like the existing `from` rules; the Rust
validator stays the source of truth, so a hand-edited file and an imported
file are rejected the same way. The help icon sits next to the spoken-form
label: a short tooltip says what the mark is and that it applies only to
engines with stress support — no engine names, which would rot.

## Risks / Trade-offs

- **[A mark that no shipping engine uses]** → `silero-native` uses it from this
  change on; if the probe shows it is wrong there, flipping the profile value
  back to `None` restores today's behavior without touching the dictionary.
- **[Vowel list drift between TS and Rust]** → both lists are covered by tests
  with the same cases (see the tasks), and the Rust side rejects what the
  editor lets through only in the "hand-edited file" direction, where an error
  in the log is the expected behavior.
- **[`+` inside a word the engine's charset lacks]** → `Charset::ModelAlphabet`
  engines keep `+` in their alphabet (Silero's frontend symbol table contains
  it), so the mark does not get filtered out before the model sees it.
