# Design: Cyrillic dictionary keys

## Context

- `validate_entry` requires `^[A-Za-z0-9]+$` with at least one ASCII letter;
  the editor mirrors it in `SOURCE_TOKEN_RE`. The pipeline pre-pass uses
  `\b[A-Za-z0-9]*[A-Za-z][A-Za-z0-9]*\b` over the whole text and looks each hit
  up in a `BTreeMap` keyed by the lowercased `from`.
- A dictionary replacement is inserted verbatim into the region it replaces and
  is skipped by later phases, which is what makes a `+` mark safe (#312).
- The native accentor (`silero-native/src/frontend/accentor.rs`) handles a user
  `+` on *any* word: the exception branch and the model branch both split the
  raw word on the stress token before predicting and keep the user's positions,
  so a marked dictionary word reaches the model as a stress hint.

## Decisions

### D1. One token, one script, Russian letters only

`from` is a single token of letters and digits with at least one letter, all
letters from one script: ASCII (`A-Za-z`) or Russian Cyrillic
(`а-я А-Я ё Ё`). Digits may appear on either side. Rationale:

- *Mixed script rejected* (`IPзамок`): no real word looks like that, and the
  two candidate patterns can then be disjoint, so a text token is matched by at
  most one pass.
- *Russian letters only*: the rest of the pipeline speaks Russian (`ё` is
  handled, `і`/`ї`/`є` are not), so accepting them would silently produce
  entries that never fire or fire on text the pipeline mangles.
- *Length in characters, not bytes*: the old 64 was bytes, which is the same
  number for ASCII and now means what it says for Cyrillic.

### D2. Matching stays exact and case-insensitive

`замок` matches `Замок`, `ЗАМОК`, and `замок`, and nothing else: `замка` needs
its own entry. Inflection tolerance is deliberately out of scope — Russian
endings are ambiguous enough that a stem rule would rewrite unrelated words, and
a wrong rewrite is worse than the wrong stress being fixed. `е` and `ё` are
different keys for the same reason (`все` vs `всё`).

### D3. Two candidate patterns, each gated by the dictionary's contents

The pre-pass keeps the Latin pattern and adds a Cyrillic one
(`\b[0-9]*<cyrillic-letter>[0-9<cyrillic-letter]*\b`), and runs a pattern only
when the dictionary has a key of that script. This keeps three cases cheap:

- empty dictionary: the whole pre-pass returns before scanning (today's path);
- Latin-only dictionary: one Latin scan, exactly as today;
- dictionary with Cyrillic keys: one extra scan, which is the price of the
  feature.

A single `\p{L}` pattern would scan every word of every document for every
user, including the many users whose dictionary has no Cyrillic entry.

### D4. The mark means the same thing for both scripts

The rendering decision is per profile, not per script: `Stress::Auto`/`Manual`
keeps the `+`, `Stress::None` strips it. The native engine can therefore fix the
stress of a Russian word today; the sidecar and Piper still drop the mark
(#311), which is honest — they would read `+` as "plus" or ignore it.

## Risks / Trade-offs

- **[A user enters `замок` and expects `замка` to change too]** → the field
  description and the spec say "exact word form"; the morphology issue covers
  the general fix.
- **[One more full-text scan for Cyrillic dictionaries]** → the scan is gated by
  `has_cyrillic_keys` and pinned by the performance lock; a 1 MB document with
  one Cyrillic entry must stay inside the existing budget.
- **[The mark may not survive a future engine's frontend]** → the profile is the
  single place that decides; a wrong declaration is a one-line data fix.
