# Design: preview-dialog-html-add-flow

## Decision

D1. **The preview gate sits after flavor detection, not after the plain
fallback.** `addEntry()` becomes: (1) best-effort read of the `text/html`
flavor via `navigator.clipboard.read()`; (2) read plain text via the plugin;
(3) a pure decision function maps `{ html, plain, previewEnabled,
defaultFormat }` to one of four actions:

| html non-empty | plain non-empty | previewEnabled | Action |
|---|---|---|---|
| yes | — | true  | open dialog, text = raw HTML, selector = `html` |
| no  | yes | true  | open dialog, text = plain, selector = `defaultFormat` |
| no  | no  | true  | neutral «Буфер обмена пуст» hint |
| yes | — | false | direct HTML ingest (today's behavior, plain fallback on empty extraction) |
| no  | yes | false | direct plain `addTextEntry` |
| no  | no  | false | neutral «Буфер обмена пуст» hint |

The decision function lives in `src/lib/addFlow.ts` as a pure exported
function so the matrix is unit-tested without mounting `AppShell`
(the component has no test harness today).

D2. **No new PreviewDialog prop.** The dialog resets `sourceFormat` from
`defaultFormat` on every open (`useEffect` on `[opened, text,
defaultFormat]`), so AppShell passes `defaultFormat = 'html'` for an
HTML-detected opening. The dialog's existing `html` handling (extraction for
the right pane, `resolveIngest` on synthesis) covers the rest.

D3. **Ordering of reads.** Both flavors are probed best-effort up front
(HTML via `navigator.clipboard.read()`, plain via the plugin); the plain
result is only *used* when HTML is absent or — on the direct path — when
its extraction yields nothing. Reading both unconditionally keeps the flow
linear (no conditional second read after the decision); the extra plugin
read is cheap and side-effect free.

## Alternatives considered

- **Skip the HTML fast-path entirely when preview is enabled** (always open
  the dialog with the plugin's plain text). Rejected: loses `html_source` —
  the entry would render as plain text instead of sanitized HTML, a silent
  regression of the html-ingestion spec for preview users.
- **Make the preview dialog always auto-detect HTML** from its `text` prop.
  Rejected: explicit selector state passed from the caller is simpler and
  keeps auto-detection in one place (the Add flow).
