# Design: retire-multi-word-phrases

## Context

The English phase (`src-tauri/src/pipeline/mod.rs::process_english_tracked`)
walks `re_english_words()` = `\b([A-Za-z]+)\b` and resolves each match in
priority order (lone letter → exact `IT_TERMS` → composed inflection →
all-uppercase abbreviation → `AS_WORD` → transliteration). A phrase such as
`stack trace` therefore arrives as two independent words, and
`MULTI_WORD_PHRASES` — read only by `EnglishNormalizer::normalize` — is
unreachable from production text (#287).

The tempting fix is to make the table reachable. Measured against `origin/main`
(per-word `normalize` of each word, joined by a space, compared with the curated
row), the table turns out not to carry phrase-level knowledge at all:

| phrase | per-word | curated | diverging word |
|--------|----------|---------|----------------|
| `stack trace` | стак трейс | стэк трейс | `stack` |
| `live reload` | ливе релоуд | лайв релоуд | `live` |
| `dry run` | дри рун | драй ран | `dry`, `run` |
| `edge case` | едге кейс | эдж кейс | `edge` |
| `use case` | юз кейс | юз кейс | — |
| the other ten rows | identical | identical | — |

Eleven of fifteen rows are exactly the concatenation of their words' readings,
and the four that differ do so because a single word reads better than its
transliteration. A phrase entry is also a fragile place for a word fix: the
reading only applies to the separator the matcher knows, so `stack\ntrace`
stays "стак\nтрейс" and the output depends on invisible formatting. The reading
belongs to the word.

## Decisions

- **Curate the words, not the phrases.** Add `IT_TERMS` entries for the five
  words the diverging rows needed: `stack` → "стэк", `live` → "лайв",
  `dry` → "драй", `run` → "ран", `edge` → "эдж". They become ordinary
  dictionary words: reachable from the per-word pass, case-insensitive like
  every other entry, usable as stems by the inflection composition
  (`stacks` → "стэкс"), and no longer silent for the `normalize` / pipeline
  split.
- **Give `living` its own entry.** The composed reading of `live` + `ing` would
  be "лайвинг", but `living` is /ˈlɪvɪŋ/ — the verb form, not the adjective
  "live" (/laɪv/) that the curated stem means here. `living` → "ливинг" is
  therefore curated separately, with a test naming the reason. Other verb forms
  (`lives`, `lived`) are rare enough in IT narration to stay uncurated; the
  adjective/reload sense is what `live` carries.
- **Retire the table and its lookup.** On `origin/main` the mechanism is
  `MULTI_WORD_PHRASES` plus the exact-phrase step at the top of
  `EnglishNormalizer::normalize` (`phrase_reading`); the branch deletes both,
  the whole-phrase test cases of `case_insensitive`, and the `multiword_phrases`
  test that asserted the table. The normalizer's lookup order is again
  IT_TERMS → composition → transliteration, and `normalize` takes one word — the
  only shape its callers pass.
- **Say so in the spec.** The delta removes the `MULTI_WORD_PHRASES` clause from
  the resolution order and the phrase-first scenario, and states instead that
  multi-word input has no reading of its own; new scenarios pin the curated
  words, the separator independence (`stack trace` / `stack  trace` /
  `stack\ttrace` / `stack\ntrace` → "стэк трейс", "стэк трейс", "стэк трейс",
  "стэк\nтрейс") and the user-dictionary precedence. The phase order itself is
  unchanged, so the requirement is not part of the delta.
- **Baseline fixture stays word-level.** `baseline.csv` feeds one row per
  `normalize` call, so its fifteen multi-word rows contradicted the new
  signature-level contract and are deleted; the four word rows the new entries
  change (`dry`, `edge`, `living`, `run`) are refreshed to the correct readings.
- **New golden fixture.** `english_phrases_words` pins the new contract in prose
  — six former phrases, `stack`/`trace` split across a line break, and the
  doubled-space form — so a regression that reintroduces phrase lookup fails on
  both the unit and the fixture level.

## Rejected alternatives

- **The phrase pre-pass (the branch's first attempt, shipped then withdrawn).**
  Rejected: it encoded single-word fixes as phrases, needed word-level splitting
  of each reading to survive `TrackedText`'s whitespace-collapse overlap, left
  the newline case broken, and kept a mechanism that no row actually justifies.
- **Keep the table for the four diverging rows only.** Rejected: the same
  objection in miniature — those rows fix `stack`, `live`, `dry`, `run`, `edge`,
  and a word fix belongs in the word dictionary where every separator, case, and
  inflection path reaches it.
- **Merge compounds (`front end` → "фронтенд").** Rejected here as a different
  feature: the transformation deletes the space, so the replacement must span two
  source words, which collides with the recorded whitespace collapse
  (`TrackedText::flush_pending` drops overlapping ranges) and forces a decision
  about the char map of the merged token. It also needs a curated list, not a
  rule: `unit test` stays two words while `data set` merges. Out of scope; the
  candidates measured now read as `фронт енд`, `бак енд`, `дата сет`,
  `наме спасе`, `тайм лайн`, `брик пойнт`, `чек сум`, `шорт кут`, `лог ин`,
  `ворк флоу` — every one of them is also a word-level reading defect.
- **Heteronym disambiguation by context (`live` adjective "лайв" vs verb
  "лив").** Rejected: reliably separating the senses needs context modelling the
  pipeline does not have, and one curated reading covers the IT cases
  (`live reload`, `live stream`, `live demo`); the escape hatch for the rest is
  a user-dictionary entry, which today cannot span a space — that gap is a
  follow-up issue rather than a reason to keep a built-in phrase table.

## Intended reading drift

Pipeline readings for the fifteen former phrases (before → after):

| phrase | before | after |
|--------|--------|-------|
| `stack trace` | стак трейс | **стэк трейс** |
| `live reload` | ливе релоуд | **лайв релоуд** |
| `dry run` | дри рун | **драй ран** |
| `edge case` | едге кейс | **эдж кейс** |
| `use case` | юз кейс | юз кейс |
| the other ten rows | unchanged | unchanged |

Non-drift, pinned by tests: `stack traces` → "стэк трейсес" (`traces` is
composed as before, `stack` is now curated), `restack trace` → "рестак трейс"
(no word-boundary phrase hit ever applied), `stack_trace` / `StackTrace` →
"стакк траке" (consumed by the code-identifier phase long before English),
`hot-fix` → "хот фикс" (already correct word by word).
