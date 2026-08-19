# Proposal: preview-dialog-html-add-flow

Closes #195. Part of #185 (Windows support), blocks the v0.3.0 release.

## Problem

The Add-button flow has two clipboard paths that disagree about the preview
gate:

- `addEntry()` in `src/components/AppShell.tsx` first makes a best-effort
  `navigator.clipboard.read()` for a `text/html` flavor. When found, the HTML
  ingestion path creates the queue entry **immediately**, before the
  `preview_dialog_enabled` check.
- Only the plain-text fallback (`tauri-plugin-clipboard-manager::readText()`)
  honors the preview gate.

On Linux/WebKitGTK `navigator.clipboard.read()` is unavailable, so the HTML
fast-path never fires and the preview dialog always shows. On
Windows/WebView2 (Chromium) the read succeeds after a one-time clipboard
permission grant — so on Windows the preview dialog is silently skipped for
anything copied from a browser, and synthesis starts immediately. This was
observed on the v0.3.0 Win10 VM pass: «Предпросмотр нормализации» never
appears.

## Change

When `preview_dialog_enabled` is `true`, the preview gate applies to **both**
clipboard flavors:

- HTML flavor present → open `PreviewDialog` pre-filled with the raw HTML
  markup, with the source-format selector initialized to `html` (instead of
  the configured `text_format` default). The dialog already extracts and
  normalizes HTML for its right pane and on synthesis, so no new dialog
  behavior is needed beyond the initial selector value.
- Only plain text → unchanged: dialog opens with the plain text.
- Neither → unchanged: neutral «Буфер обмена пуст» hint (#194).

When `preview_dialog_enabled` is `false`, the current direct-ingestion
behavior is preserved exactly (HTML auto-ingest, plain fallback, no dialog).

## Scope

- Frontend only: `src/components/AppShell.tsx` (flow decision) and a pure,
  unit-testable decision helper in `src/lib/`. `PreviewDialog` needs no new
  props — `defaultFormat` already resets per open.
- No backend changes. The paste (`Ctrl+V`) flow is untouched.

## Risks

- Users on Windows get one extra dialog for HTML content they previously
  skipped — that is the intended, spec'd behavior (preview-dialog spec makes
  the dialog the gate for the Add flow).
- The decision helper must keep the "HTML yields no readable text → plain
  fallback" semantics of the direct path; covered by unit tests.
