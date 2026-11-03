# Design — numeric span normalization

## Context

The pipeline used to normalize numbers with a stack of regex phases, one per
shape: sizes and durations, dates and times, percentage ranges, percentages,
ranges, versions, leading-dot decimals, bare numbers. The stack was held
together by ordering: the percentage-range phase had to run before the
percentage phase, versions before leading-dot decimals, leading-dot decimals
before bare numbers, URLs before everything numeric. Each phase collected its
own matches over a snapshot, and a shape the previous phase had already
consumed was silently skipped by the tracked-text overlap rule. Adding a shape
(named units, ordinal suffixes, years) meant another regex and another ordering
constraint, and the readings stayed context-free: a year was a cardinal, a day
after "от" was a label form, a count did not agree with its head noun.

Numbers are not marginal in the target texts. A technical article carries
sizes, versions, dates, ranges, prices, phone numbers, reference numbers and
identifiers in almost every paragraph, and the whole point of the
normalization layer is to read them the way a competent speaker would. The
epic (#343) therefore set an evaluation rather than a wish list: a corpus of
256 cases (57 locked — already correct, kept as a regression guard; 199 target
— the natural reading), the existing app-level golden fixtures, and a 1 MB
runtime sample on a fixed machine with a cap of 1.5x the current pipeline's
time.

Baseline on the reference machine: main normalizes the 1 MB corpus sample in
11.83 s. The span layer does it in 8.77 s (1.35x faster). The target corpus is
199/199 and the locked set is green. The rejected alternative — keeping the
old phases and adding a morphology pass over their output — reached 198/199 at
19.99 s, i.e. 1.69x the baseline, over the 1.5x cap.

## Goals / Non-Goals

**Goals:**

- Read every numeric case in the corpus the way the corpus says a Russian
  speaker would: agreement, case, gender, ordinal years, dates, ranges,
  decimals, money, phones and identifiers.
- Remove the cross-phase race: one owner for every digit, no ordering
  constraints between numeric shapes.
- Keep the existing invariants: one batch per phase, no overlapping
  replacements, char mapping intact, profile gating unchanged.
- Stay within the 1.5x runtime cap on the 1 MB sample.

**Non-Goals:**

- An LLM post-edit pass over awkward readings (#342).
- Changing TTS engines, profiles or adapters.
- Adding or changing user-dictionary entries.

## Decisions

**One span layer, not a morphology pass bolted onto the old phases.** The
rejected approach B kept the regex phases and added a morphology stage that
re-read their output. It measured 198/199 on the target corpus at 19.99 s on
the 1 MB sample — one case short and 1.69x the 11.83 s baseline, over the 1.5x
runtime cap — because the old phases still ran, still collected their own
matches, and the morphology stage added a second pass over the text. It also
kept the ordering constraints that caused the wrong readings in the first
place. The chosen layer scans the text once: the scanner consumes a maximal
candidate (digits plus the separators and words that belong to the construct),
the reader classifies it by shape and context, and only the recognized spans
are replaced.

**Maximal candidate first, classification second.** `parse` records the shape
of a candidate — a date, a time, a range, a reference, a phone, a dotted
version, a comma fraction, an ordinal suffix, a unit or a currency attached —
without deciding how it reads. `read` chooses the reading and asks
`morphology` for the required case and gender, using the words before and
after the span (a year word, a preposition, a head noun, a month or season
name, a reference prefix). Rejected: a table of regexes with lookahead — the
regex crate has no lookbehind, and every new context rule would add another
pattern and another ordering constraint instead of one branch in the reader.
Rejected: classify while scanning — the scanner would need the reading rules
for every shape, and a candidate that another owner should keep (a dotted
version inside an address, a digit glued to a letter) would have to be
re-parsed.

**Batch replacement through the existing tracked-text queue, not
placeholders.** The layer takes one snapshot, collects `(start, end, reading)`
for every span the reader accepts, and calls `tracked.replace_byte_ranges`
once. This is the machinery the removed regex phases already used: replacements
refer to the pre-flush snapshot, are applied right-to-left, and overlapping
ranges are dropped by the shared rule. The char mapping therefore comes from
the same path as every other phase, and the layer does not introduce a second
coordinate space. The phase runs inside the placeholder window: native code
blocks stay hidden behind their placeholders while the narrating phases run and
are restored after the mapping is built, so an engine-owned fragment is never
scanned. Rejected: inserting placeholders for numeric spans — nothing
downstream needs protection from an already-replaced digit, and placeholders
would complicate the mapping for no benefit. Rejected: `TrackedText::replace`
per span — it is an unbounded replace-all and would hit the same digits inside
longer tokens, which is exactly the bug the byte-range API exists to avoid.

**Address regexes survive as address tokens.** URLs, e-mails, IPs and file
paths keep their static regex phases, which run before the numeric span phase
and hand the whole address to `URLPathNormalizer`. An address is not a numeric
value: its reading is composed from host words, group digits and path parts,
and the address phase must claim it before the span scanner sees the digits.
This also keeps "192.168.0.1" from being scanned as a dotted version.
Rejected: teaching the span layer about addresses — the address reader is
already a separate, profile-aware owner, and duplicating host/path rules in
the numeric layer would give the same fragment two owners, the exact problem
this change removes.

**Dash normalization after the numeric span phase.** A range must keep the
dash the source used ("5–10", "5—10", "5-10") so the scanner can consume it as
part of one candidate and the reader can emit "от пяти до десяти". The
tracked-text rule drops a replacement that overlaps an already-replaced range,
so normalizing the en dash to "-" first would make the range span overlap the
dash replacement and the reading would be lost or double-processed. Moving the
dash phase after the numeric phase makes the span own the source dash; the
remaining dashes are normalized afterwards, and the post-processing trims the
spaces the reading leaves behind. Rejected: normalizing first and teaching the
scanner only "-" — en dash ranges (the common prose form) would be split.

**Gating is decided when a span is claimed.** With `units: Native` the
scanner does not claim a construct that carries a unit, percentage, currency
or scale word, so the number and its symbol stay together with the engine and
the existing "units and digits are independent" behavior is preserved. With
`digits: Native` the whole numeric phase is skipped. Rejected: read the
number and leave the symbol — that is the stranding bug the old
`unit_owns`/`sub_number_phase` helpers existed to prevent, and it would
contradict the profiles' capability contract.

**Corpus-based evaluation, not unit-only.** The corpus harness is the behavior
specification: each case is a full sentence and the exact normalized text, so
it pins agreement, case, punctuation and surrounding words at once, which
helper-level unit tests cannot. The locked set is the regression guard, the
target set is the yardstick (199/199 required), the app-level goldens pin the
pipeline integration, and the runtime test pins a 1 MB sample against the
baseline. A numeric reading that is not in one of those sets is a reading the
change does not claim.

**Morphology and lexicon are separate modules.** `morphology` is the only place
that knows Russian numeral inflection (cardinals, ordinals, decades,
fractions); `lexicon` holds lexical facts (units and their declensions,
currencies, scale words, months, noun genders, reference words, ordinal
heads). Rejected: formatting readings in the reader — the same numeral would
be built in several places and drift, which is how the old pipeline ended up
with two date normalizers and two number tables.

## Risks / Trade-offs

- **Ambiguous suffixes.** "N-й" and "N-х" depend on the head noun and the
  preposition. The rules above reproduce the corpus notes, but unusual prose
  can still be resolved differently; the corpus case note is authoritative
  when a new example is added.
- **Invalid calendar values.** The old date phase returned its input unchanged
  on an out-of-range date so the number phase could read the digits; there is
  no number phase now. An invalid time is pinned to fall through to the number
  reading ("25:00" → "двадцать пять:ноль"); an out-of-range dotted date is
  not a date and falls to the version reading ("32.13.2024" → "тридцать два
  точка тринадцать точка две тысячи двадцать четыре"). Neither fallback is
  pinned by the corpus, and the old requirement text does not describe them.
- **Hand-written tables.** 4 000 lines of numeric code, most of them word
  tables and morphology. New shapes need a scanner branch, a reader branch and
  a corpus case; the corpus review before merge is the guard.
- **One snapshot per pass.** The layer clones the text once to scan it, like
  the removed regex phases did per phase. The measured result is 1.35x faster
  overall because eight regex passes and their snapshots disappeared.
- **Profile coverage.** The corpus is evaluated under the Silero profile. Other
  profiles inherit the same readings only when they expand digits and units;
  a profile that keeps them Native still gets the symbols untouched, as the
  gating tests pin.
