# Design: restore-dropped-e-and-y-in-inflections

## Context

`EnglishNormalizer::compose_inflection` walks `INFLECTION_SUFFIXES` (longest
first) and composes `IT_TERMS[stem] + transliterate(suffix)` as soon as the
stripped stem is a curated entry. English inflections often modify the base
itself, so the stripped stem is not what the dictionary stores:

- silent "e" is dropped before a vowel- or elision-taking suffix
  (`use` → `us`+`es`, `drive` → `driv`+`er`)
- "-y" becomes "-ies" (`repository` → `repositor`+`ies`)

The helper is shared by `normalize` and the pipeline's all-uppercase path, so a
single fix covers both.

## Decisions

- **Restore, don't stem.** Only two bounded restorations are added: append a
  silent "e" to the stripped stem, and read "-ies" through `stem + "y"`. A
  general stemmer would guess at unknown bases and could compose readings for
  words that are not curated — outside the change's blast radius.
- **Restore after the direct hit.** The existing exact-stem composition is tried
  first, so already-working forms are untouched by construction. The
  restoration only runs for stems that are not curated entries.
- **Sibilant gate for the syllabic "-es".** English spells a syllabic "es" only
  after a sibilant base (`caches`, `uses`, `pages`, `services`), while a
  non-sibilant base takes a plain "s" (`features`, `files`, `types`). Without
  the gate, restoring `featur` → `feature` would produce "фичаес" instead of
  "фичас". The gate inspects the restored stem minus its final "e" for a
  sibilant ending (`ch`, `sh`, `s`, `z`, `x`, `c`, `g`) and, when it fails,
  falls through to the shorter suffix / raw fallback.
- **"-ies" requires `stem + "y"`.** Matching `IT_TERMS[stem + "y"]` is what
  makes `repositories` compose from `repository` and stops `species` from
  composing from `spec` (since `specy` is not an entry). The base reading drops
  a trailing "й" because the Russian plural adds only "с" (`репозиторий` →
  `репозитори` + `с`), not "ис".
- **Silent-e restoration skips "-ally".** English elides a base-final silent
  "e" before `-s`/`-es`/`-ed`/`-ing`/`-er(s)`/`-able`/`-ation`/`-ment(s)`
  (`use` → `uses`, `achieve` → `achievement`), but "-ally" attaches to an "-al"
  base (`practical` → `practically`). Restoring there would read `practice` +
  "ally" = "практисалли" for a word whose base is `practical`, so the suffix is
  excluded from restoration and `practically` keeps its previous reading.
- **Keep the minimum stem length for direct hits.** The direct path keeps its
  `>= 3` guard unchanged; the restoration path only needs `>= 2` so that
  `us` → `use` composes. This keeps two-letter entries (`cd`, `js`, `ts`) out of
  direct composition exactly as before.

## Rejected alternatives

- **Try the restored "e" before the direct hit:** rejected — `species`-style
  inputs and existing forms like `codes`/`files` would be re-read through a
  restored base (`code`/`file`) instead of the direct stem, changing readings
  that currently work.
- **Split `-es` into a separate earlier pass:** rejected — it would duplicate the
  suffix table and bypass the direct-hit precedence that keeps `features`
  → "фичас".
- **Delete the `-ies` branch entirely:** rejected — `repositories` would lose its
  curated reading even though the fix should keep it.

## Intended baseline drift

`baseline.csv` snapshots canonical readings, so the fix necessarily moves rows
whose stem is a curated entry ending in a silent "e". All three were reviewed
individually and follow from the restoration rule:

| row | before | after | base |
|-----|--------|-------|------|
| `driver` | дривер | драйвер | `drive` + "-er" |
| `driving` | дривинг | драйвинг | `drive` + "-ing" |
| `packaging` | пакагинг | пакетинг | `package` + "-ing" |

Readings outside `baseline.csv` also move in the intended direction — a
sibilant `-es` now composes the syllabic "ес" (`cases` "кейсс" → "кейсес",
`packages` "пакетс" → "пакетес", `services` "сервисс" → "сервисес") — but none
of them appears in the baseline word list, so they are not rows here. The same
rule moves `compilation` from "компилашн" to "компайлашн" (restored base
`compile`), which follows the elision rule.

The end-to-end behavior is pinned by a new `english_inflected_silent_e` golden
fixture (using/uses/used/caches/cached/repositories/FEATURES/species).

