## Context

Ingestion is clipboard-only: `resolveAddAction` (`src/lib/addFlow.ts`) maps
the clipboard probe to a step, `resolveIngest` (`src/lib/ingest.ts`) owns the
sanitize + extract decision, both pure and unit-tested. The Add button and
paste share these; the tray uses plain text only. The preview gate
(`preview_dialog_enabled`) wraps the Add button but not paste
(preview-dialog spec). Backend precedent for hardened network I/O exists:
`fetch_image_bytes` (scheme allowlist via `validate_image_url`, shared
`OnceLock<reqwest::Client>` with timeouts, 20 MiB cap, coded
`CommandError`s localized since add-ui-localization). Drag & drop is unused;
in Tauri 2 the webview-level API is `getCurrentWebview().onDragDropEvent()`
(delivery differs per platform — see Risks).

## Goals / Non-Goals

- Goals: one import flow feeding the existing ingest path; legacy-Cyrillic
  files read correctly; clear failure messages for undecodable files, bad
  HTTP responses, and JS-rendered pages.
- Non-goals: batch drops (#242), JS rendering, new formats, pipeline changes
  (see proposal).

## Decisions

### D1: Drop events via Tauri webview API, not HTML5 DnD

Use `getCurrentWebview().onDragDropEvent()` (enter/over/leave/drop with file
paths or plain-text payload). HTML5 drag events are unreliable in Tauri v2:
with `dragDropEnabled` (default) the native handler intercepts them on
Windows/Linux. The overlay state derives from enter/leave/drop transitions.
Link detection: prefer the event's payload when the platform provides it,
otherwise treat a dropped `.url`-style text / single string starting with
`http(s)://` as a link.

- Alternative considered: HTML5 `drop` handlers — rejected (suppressed by the
  native handler; WebKitGTK quirks).

### D2: Backend reads files, frontend never touches paths

New command `read_text_file(path, encoding?: string)` in
`src-tauri/src/commands/`: validates the extension allowlist, enforces a
size cap (10 MiB, same order as fetch), detects encoding (BOM first, then
statistical detection over the supported encodings), returns `{ text,
encoding }`. Rationale: encoding detection needs byte-level access; doing it
backend-side keeps one hardened reader for both quick open and manual
override (frontend passes back the chosen encoding name to re-decode
deterministically). Use **`encoding_rs` + `chardetng`** — the Firefox pair:
`encoding_rs` carries the WHATWG table (44 encodings incl. every required
Cyrillic one), decodes by label for the manual override and sniffs BOMs via
`Encoding::for_bom`; `chardetng` covers statistical detection of BOM-less
legacy bytes. Glue (~20 lines: BOM → UTF-8 validity → guess → non-text
guard) lives in one testable helper (`src-tauri/src/import.rs`).
*Amended during implementation:* the proposal's original candidate
`transcoding_rs` turned out to be a stale 2021 micro-wrapper over this same
pair with a reader-oriented API, while decode-by-label needs `encoding_rs`
as a direct dependency anyway — so the wrapper saved nothing.

- Alternative considered: `transcoding_rs` wrapper — rejected after review
  (0.1.1, ~4k downloads, unmaintained since 2021; streaming/`Read` API that
  still forces explicit `encoding_rs`).
- Alternative considered: `tauri-plugin-fs` + JS `TextDecoder` — rejected
  (TextDecoder ships no Cyrillic statistical detection; two readers to
  harden).
- Alternative considered: `aconv` / `charset_normalizer_rs` — CLI batch-tree
  tool / detection-only metadata API without decode-by-name; both sit on the
  same WHATWG set.

### D3: URL fetch mirrors fetch_image_bytes hardening

New command `fetch_url_text(url)` reusing the shared HTTP client:
scheme allowlist (http/https), total size cap 10 MiB, connect 5 s / total
30 s timeouts; returns `{ text, encoding, content_type }` — the body is
decoded by the same `import.rs` helper as file reads (one home for the
bytes→text knowledge, and no multi-megabyte byte arrays over IPC).
*Amended during implementation:* D3 originally returned raw body bytes for
frontend-side decoding via a hardcoded UTF-8→CP1251 `TextDecoder` fallback;
decoding server-side keeps a single detection pipeline instead of mirroring
it in TS. Format classification (HTML vs plain vs SPA) stays frontend-side,
shared with file routing. SPA heuristic lives in the frontend next to the
extractor: extracted text below a threshold (~500 chars) AND markup
dominated by scripts/mount points (`chardetng`-free, cheap DOM checks) →
coded error `import.spa_unsupported`. Thresholds become named constants
with tests.

### D4: Format routing as a pure function

Extend the pure decision layer: `resolveImport(source)` where source is
`{ kind: 'file'; format: EntryFormat; text: string } |
{ kind: 'url'; body: string; contentType: string | null }` → same
`AddAction` shape the Add flow already produces, so AppShell has one
executor for all entry points. Extension→format mapping is trivial;
auto-detection delegates to #241's `detectFormat` once that lands (until
then URLs default to the HTML path with plain fallback). Unit-test the full
matrix without mounting components (pattern of `addFlow.test.ts`).

### D5: Encoding dialog before, not inside, the normalization preview

«Файл с кодировкой…» opens a small modal: raw decoded preview (monospace,
unnormalized) + encoding Select preselected with the detected value +
confirm/cancel. Confirm re-invokes `read_text_file(path, chosen)` and only
then opens the normal preview dialog. Rationale: the two previews answer
different questions (byte decoding vs spoken-text shaping); merging them
couples unrelated concerns and breaks the preview-dialog spec's contract.

### D6: Errors ride the CommandError localization layer

New codes under an `import.*` namespace (e.g. `import.unsupported_extension`,
`import.decode_failed`, `import.too_large`, `import.fetch_failed` with
`{status}` param, `import.spa_unsupported`, `import.empty_page`) added to
`src/i18n/{ru,en}.ts`; notifications via the existing bridge. Pre-text-exists
failures surface as errors; post-gate failures keep the dialog's own error
path.

## Risks / Trade-offs

- [Drag-event payload varies per platform (paths vs text, missing over
  events on some Linux WMs)] → derive overlay from enter/leave/drop with
  defensive fallbacks; manual tests cover Windows + Linux (KDE Wayland).
- [Statistical detection can misguess short ambiguous files] → BOM first,
  UTF-8 validity check second (most modern files are UTF-8), heuristics
  last; «Файл с кодировкой…» is the escape hatch.
- [SPA heuristic false negatives (partial SSR)] → accepted; documented in
  the spec scenario set, thresholds tunable constants.
- [10 MiB cap may reject huge generated HTML] → matches the fetch cap; a
  real article page is far below it.

## Migration Plan

Purely additive: no stored data, wire format, or existing flows change.
The navbar Add control swaps to a split-button — visual change only, same
primary action.

## Open Questions

- None blocking; exact SPA-threshold values settled during implementation
  against real pages (constants + tests).
