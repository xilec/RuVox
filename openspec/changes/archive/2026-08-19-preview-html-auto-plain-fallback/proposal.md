# Proposal: preview-html-auto-plain-fallback

Post-review fix of #198 (part of #185, blocks v0.3.0). Found by
ruvox-reviewer on the merged diff.

## Problem

The preview-gated HTML path dropped the plain-text fallback that the
ungated path keeps:

- Clipboard carries both flavors; the `text/html` markup is nav/button
  chrome yielding no readable text; `text/plain` has the content.
- `preview_dialog_enabled = true` → the dialog opens with the raw markup
  and `html` pre-selected (auto-detected).
- «Синтезировать» → extraction fails → red «Не удалось извлечь текст из
  HTML» and no entry — while the same clipboard with the preview gate off
  ingests the plain text silently.

## Change

Carry the plain flavor into the preview opening (`AddAction::preview` gains
`plainFallback`). When synthesis from the dialog rejects an `html`
extraction **and** the dialog was opened from an auto-detected HTML flavor
(plain fallback carried), ingest the plain text instead of erroring —
identical to the ungated direct path.

An explicit `html` selector choice keeps the current red-error behavior:
the fallback is `null` when the dialog was opened with plain text, so
picking `html` by hand and failing extraction still shows «Не удалось
извлечь текст из HTML».

## Scope

- `src/lib/addFlow.ts` (+ tests: the two missing trim-matrix cells come
  along), `src/components/AppShell.tsx` (carry/reset the fallback, use it
  in the reject arm).
- Delta spec: `preview-dialog` ("Source format selection" requirement).

## Risks

- A user who edits the markup into something unextractable gets the
  original clipboard plain text instead of an error. Accepted: rarer than
  the chrome-only-HTML case this fixes, and the entry content remains
  truthful to what was copied.
