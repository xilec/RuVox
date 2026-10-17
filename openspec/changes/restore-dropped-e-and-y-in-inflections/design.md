# Design: restore-dropped-e-and-y-in-inflections

## Context

`EnglishNormalizer::compose_inflection` walks `INFLECTION_SUFFIXES` (longest
first) and composes `IT_TERMS[stem] + transliterate(suffix)` as soon as the
stripped stem is a curated entry. English inflections often modify the base
itself, so the stripped stem is not what the dictionary stores:

- silent "e" is dropped before "-s"/"-ed"/"-ing" (`use` → `us`+`es`)
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
whose stem is a curated entry ending in a silent "e". All four were reviewed
individually and follow from the restoration rule:

| row | before | after | base |
|-----|--------|-------|------|
| `driver` | дривер | драйвер | `drive` + "-er" |
| `driving` | дривинг | драйвинг | `drive` + "-ing" |
| `packaging` | пакагинг | пакетинг | `package` + "-ing" |
| `practically` | практикалли | практисалли | `practice` + "-ally" |

The last row is the suffix-agnostic edge of the rule: the restored base is the
curated `practice` reading, so "-ally" composes from it rather than from the
transliterated whole word. Restricting restoration to some suffixes would make
the rule arbitrary, so the drift is recorded here instead. No golden pipeline
fixture changes.

