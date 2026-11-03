## Why

The pipeline reads each numeric shape with its own regex phase — sizes, dates
and times, percentage ranges, percentages, ranges, versions, leading-dot
decimals, bare numbers — and the phase order is load-bearing: percentage ranges
must run before percentages, versions before numbers, leading-dot decimals
after versions. Every phase races the others for the same digits, and the
readings that fall out of that race are wrong in ordinary Russian text:

- "в 2024 году" keeps the cardinal year ("в две тысячи двадцать четыре году");
- "Отчёт от 15.01.2024" keeps the label form of the day instead of the
  genitive ("от пятнадцатое января");
- "5-й этаж" leaves the ordinal suffix untouched ("пять-й этаж");
- "42 книги" does not agree with the head noun ("сорок два книги");
- "10 000 рублей" tears the digit group apart ("десять ноль рублей");
- "1,5 кг" tears the decimal apart ("один,пять кг");
- "100 ₽" leaves the currency symbol raw.

These are not edge cases: the target texts are technical articles full of
sizes, versions, dates, ranges, prices, phone numbers and identifiers, and the
normalization layer exists precisely to read them the way a competent speaker
would. Each new numeric shape added another regex and another ordering
constraint, so the per-shape design had reached its limit.

Measured on the corpus harness: the corpus holds 256 cases, 57 locked (already
read correctly, the regression guard) and 199 target (the natural reading the
epic wants). The corpus is the behavior specification for this change. On the
1 MB sample assembled from the corpus, normalization takes 8.77 s against
11.83 s on main (1.35x faster), and all 256 cases are green after the change.
The alternative implementation — keeping the old phases and adding a
morphology pass on top — reached 198/199 at 19.99 s (1.69x the baseline, over
the 1.5x runtime cap) and was rejected; see `design.md`.

## What Changes

- One numeric span layer replaces the per-shape phases
  (`tts-normalize/src/pipeline/numeric/`): a single scan collects maximal
  candidate spans including the separators that belong to a number (space and
  NBSP digit groups, decimal comma/dot, signs, range dashes, slash dates, time
  colons, version dots and suffixes, ordinal suffixes) and the words that
  belong to the construct (units, currencies, scale words, month names,
  reference prefixes). The reader classifies each span by its shape and its
  neighbouring context and asks a morphology module for the spoken form in the
  case and gender the sentence requires.
- A morphology module owns Russian numeral inflection (cardinals, ordinals,
  decades, fractions), and a lexicon module owns the word tables (units,
  currencies, scale words, months, noun genders, reference words, ordinal
  heads). The tables are built in; the user dictionary is untouched.
- The old number, date, time, version, range, percentage, percentage-range,
  size and leading-dot-decimal phases are removed, together with the tilde
  approximation regex and the helpers that kept those phases from stealing
  each other's digits. No separate phase can race the span layer for a digit
  any more.
- Address regexes (URLs, emails, IPs, file paths) stay where they are, as
  static regex phases before the numeric span phase: an address is one token
  whose reading belongs to `URLPathNormalizer` (host words, group digits, path
  parts), and claiming a dotted address before the span scan keeps
  "192.168.0.1" from being read as a version number.
- Dash normalization moves after the numeric span phase: a range keeps the
  dash the source used ("5–10"), because replacing the en dash earlier made
  the numeric span overlap an already-replaced range and the tracked-text
  overlap rule dropped one of the two.
- The layer reuses the tracked-text batch replacement queue
  (`replace_byte_ranges`): all readings are computed against one snapshot and
  applied as one batch, so the char mapping comes from the same machinery as
  every other phase. It runs while native code blocks are still behind their
  placeholders, so an engine-owned fragment is never scanned.
- The corpus harness becomes the gate: 57 locked cases must stay green, the
  199 target cases must read 199/199, the app-level numeric goldens are
  refreshed, and the 1 MB sample must stay within 1.5x of the current
  pipeline's time.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `text-pipeline`: adds one numeric span phase that owns sizes, dates and
  times, ranges and percentages, versions, numbers, years, ordinals, money,
  phones and identifiers; removes the per-shape numeric phases from the fixed
  order and moves dash normalization after them; rewrites the six numeric
  requirements and adds "Years, ordinals, and decades" and "Money, units, and
  identifiers".

## Impact

- `tts-normalize/src/pipeline/mod.rs` — numeric span phase wiring, removed
  regex phases and their helpers, dash normalization moved after the span
  phase, address regexes kept with a comment that explains the ownership.
- New `tts-normalize/src/pipeline/numeric/{mod,parse,read,morphology,lexicon}.rs`
  — the span layer.
- `tts-normalize/src/pipeline/tests.rs` — the digit-group expectation
  refreshed ("42 142" is one grouped number) and the invalid-time comment
  updated to the parser's behavior.
- `tts-normalize/src/pipeline/normalizers/numbers.rs` — the legacy numeric
  reading methods dropped; `normalize_number` stays for the URL/IP and
  Markdown-list callers.
- `src-tauri/tests/fixtures/pipeline/silero/` — numeric goldens refreshed
  (`leading_dot_decimal`, `range_years`, `tilde_approx`,
  `tilde_approx_space`).
- `tts-normalize/tests/corpus/` — the year-1900 case corrected; the corpus
  itself is the yardstick and is not otherwise changed.
- `CHANGELOG.md` — an `[Unreleased]` entry for the user-visible reading change.
- Performance: the 1 MB sample normalizes in 8.77 s against 11.83 s on main;
  no new dependency and no new runtime cost on the pipeline path.

## Non-goals

- An LLM post-edit pass that rewrites awkward readings after normalization —
  tracked separately in #342.
- TTS engine, profile, or adapter changes: the engines receive the same
  Cyrillic text as before; only the readings differ.
- User-dictionary changes: the layer reads built-in tables only, and no new
  dictionary entries are added by this change.
