# Design: fix-html-boundary-detection

## Context

`detectFormat` (src/lib/detectFormat.ts) counts well-formed tag fragments
anywhere in the text (`</?[a-zA-Z][^<>]*>`, ≥3 → `html`). Technical and
documentation prose routinely carries tag-*looking* placeholder fragments —
`<type>(<module>): <desc>`, `<UnlistenFn>`, `<cmath>` — so the count
misfires on exactly the texts the preview dialog exists for. An earlier
approach (reordering markdown signals before the fragment count) was
discarded by the maintainer: the html signal itself should be precise, not
merely outranked.

## Goals / Non-goals

- Goal: only text delimited by tags classifies `html`; prose with
  placeholder fragments falls through to `markdown`/`plain`.
- Non-goals: markdown thresholds, tag-name allowlists, backend ingest
  behavior, the #246 check order (prefix → html → heading/fence → lists →
  links → plain stays).

## Decisions

### D1: boundary rule replaces the fragment count

`html` when the trimmed text starts with a well-formed tag AND ends with a
well-formed tag. Rationale: markup documents are delimited by tags; prose
never both starts and ends with one. Consequent flips, all accepted with
the maintainer and all in the module's conservative-`html` direction:

- bare tag-pair snippets (`<b>жирным</b>`): `plain` → `html` (reading tags
  aloud is the costlier mistake);
- unclosed fragments (`<p>раз\n<p>два\n<p>три`): `html` → non-`html`.

### D2: trim covers zero-width characters

"Непечатаемые" at the edges: `String.trim()` whitespace plus
`U+200B`–`U+200D` and `U+FEFF` (clipboard pastes from web sources carry
zero-width spaces). Implemented as a single edge-strip regex before the two
anchored tag tests.

### D3: HTML_MIN_TAG_FRAGMENTS is removed

The exported constant and the count regex usage disappear; no other module
imports them (knip-verified). The tag regex itself stays, anchored to both
ends. Tests referencing the threshold are reworked to boundary cases.

## Risks / Trade-offs

- A markdown document that *both* starts and ends with raw inline HTML tags
  (e.g. `<br>\n# Заголовок\n…\n<b>конец</b>`) classifies `html`. Accepted:
  such pastes are typically real markup, html extraction strips the tags,
  and the selector remains one click away.
- None downstream: `detectFormat` is pure; its consumers — `PreviewDialog`
  (auto label + ingest decision) and the URL `text/plain` fallback in
  `importFlow` — need no changes.

## Migration Plan

None — pure classification change; entries already ingested keep their
stored `format`.

## Open Questions

None.
