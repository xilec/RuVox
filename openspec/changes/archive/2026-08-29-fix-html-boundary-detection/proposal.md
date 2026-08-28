# Proposal: fix-html-boundary-detection

## Why

Pasting the repository's own `CHANGELOG.md` into the preview dialog classifies
as `html`. The file carries 4 well-formed tag-like fragments
(`<UnlistenFn>`, `<type>`, `<module>`, `<desc>` — the last three from the
documented commit-format line), so the ≥3 tag-fragment heuristic in
`detectFormat` fires. But the text is unambiguously a Markdown document
(ATX heading, 38 list-item lines, inline links). Found during the #239
manual pass; the fragment-count heuristic itself came in with #246.

Agreed with the maintainer: instead of reordering checks (an earlier,
discarded approach) or tuning thresholds, the html signal is made precise:
**markup is delimited by tags** — a pasted text is `html` only when its
trimmed form both starts and ends with a tag. Placeholder fragments inside
prose never satisfy both boundaries at once.

## What Changes

Replace the ≥3 tag-fragment count in `detectFormat` with a boundary rule:
after trimming whitespace and zero-width characters at both ends, the text
SHALL classify as `html` when it starts with a well-formed tag and ends with
a well-formed tag. The `<!doctype html` / `<html` document prefix keeps its
existing top priority; the evaluation order of the remaining checks is
unchanged. Consequent behavior changes, all in the module's stated
conservative-`html` direction:

- changelog/docs-style prose with placeholder fragments classifies
  `markdown` (its structure now decides), fixing the reported bug;
- a bare tag-pair snippet (`<b>жирным</b>`) now classifies `html`
  (previously `plain` below the fragment threshold) — reading tags aloud is
  the costlier mistake;
- an unclosed fragment (`<p>раз\n<p>два\n<p>три`) now classifies as
  `plain`/`markdown` instead of `html` (it does not end with a tag).

Spec delta updates the synced "Source format auto-detection" requirement
(preview-dialog spec): the fragment-count bullet is replaced by the boundary
rule, with regression scenarios for each of the above.

## Impact

- **Affected specs:** `openspec/specs/preview-dialog/spec.md`
  ("Source format auto-detection").
- **Affected code:** `src/lib/detectFormat.ts` (boundary rule replaces the
  fragment count; `HTML_MIN_TAG_FRAGMENTS` constant removed);
  `src/lib/detectFormat.test.ts` (regression cases reworked).
- **Out of scope:** markdown thresholds, backend ingestion behavior, the
  check order decided in #246.
