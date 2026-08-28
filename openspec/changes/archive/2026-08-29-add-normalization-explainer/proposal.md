# Proposal: add-normalization-explainer

## Why

The Add flow opens the normalization preview dialog, but nothing in the UI says
what normalization *is* in RuVox terms. A new user sees two panes — «Оригинал»
and «После нормализации» — with no guidance on what changed, why the right pane
reads differently, or which controls are useful. The concept (English
identifiers, abbreviations, numbers, URLs, operators rewritten so Silero can
narrate them in Russian) exists only in developer docs and the
`text-pipeline` spec. (Issue #239.)

## What Changes

User-facing copy only; no pipeline or backend work:

1. **Explainer line in the preview dialog.** A short one–two sentence hint in
   the dialog (Russian) stating what will be spoken and why it differs from
   the source, shown on every open without hiding any controls.
2. **A "Подробнее" affordance.** A `?`-style icon in the dialog header with a
   tooltip carrying the fuller explanation (what gets rewritten, what the
   source-format selector does) and a link that opens the README's
   normalization section in the system browser.
3. **README section «Нормализация».** A user-facing section in `README.md`
   (with the `README.en.md` mirror regenerated): what normalization rewrites
   and how to steer it today — the source-format choice in the preview dialog
   (Авто / Обычный текст / Markdown / HTML) and the per-document code-block
   directives `<!-- ruvox-code: full|brief -->` (incl. the Mermaid marker).
   The unused `code_block_mode` / `read_operators` config fields are NOT
   documented as user-steerable — they are not wired to the UI or the
   pipeline.

The dialog's existing requirements (gating, panes, footer) are unchanged; the
explainer is additive copy with its own delta requirement in the
`preview-dialog` spec.

## Impact

- **Affected specs:** `openspec/specs/preview-dialog/spec.md` (new requirement
  for the explainer line and the header affordance).
- **Affected code:**
  - `src/dialogs/PreviewDialog.tsx` (+ module CSS) — explainer line, header
    icon, tooltip, external link.
  - `src/i18n/ru.ts` / `src/i18n/en.ts` — new `preview.*` strings.
  - `README.md` + `README.en.md` — new section (regenerate the mirror).
- **CHANGELOG:** user-visible → an additive `[Unreleased]` entry is proposed
  in the task branch (human-owned file, approval-gated diff).
- **Out of scope:** wiring `code_block_mode` / `read_operators` to Settings or
  the pipeline; any pipeline behavior change; in-app help pages beyond the
  README link.
