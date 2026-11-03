# Design — restrict the viewer display switch to the entry's source format

## Context

`TextEntry.format` was introduced as the preview dialog's effective source
format, but the viewer's old `SegmentedControl` also wrote it through
`set_entry_format` on every switch. One field therefore carried both the
ingestion format and the display choice, and losing the original value made
the "right" view underivable.

The viewer renders `original_text` (for plain/markdown) or `html_source` (for
HTML). Only one of the three old modes is meaningful per entry; the other two
push the text through the wrong pipeline.

## Decisions

### 1. Separate the immutable source format from the display choice

`format` stays `Option<TextFormat>` and becomes write-once: only the ingestion
seam sets it, and the viewer never writes it. A new
`display_mode: Option<DisplayMode>` (`rendered` | `plain`, `#[serde(default)]`)
records the viewer choice. `set_entry_format` is removed and replaced by
`set_entry_display_mode(id, display_mode)`.

Why a persisted field instead of frontend-only state: the issue requires the
choice to survive restarts and entry switches, and history is the only
per-entry persistence available. `Option` keeps "never chosen" distinct from
an explicit `rendered`, and matches the schema's existing optional-field
pattern (no migration, older files parse).

Rejected: storing the old three-value mode and mapping it into the new views
at read time. It preserves the ambiguity — `html` on a markdown entry cannot
tell whether the user wanted the rendered or verbatim view — and would still
need clamping. Rejected: a config-level default instead of per-entry state;
the acceptance criteria require the choice to survive entry switching.

### 2. Derive the source format at render time; clamp stale values

```
deriveSourceFormat(entry):
  html_source != null            -> html
  format == "markdown" | "plain" -> format
  otherwise (null, stale "html") -> detectFormat(original_text)
```

Order matters. Stored markup is proof of an HTML source, so it wins over a
stale field: an old toggle from `html` to `markdown`/`plain` left `format`
contradicting `html_source`, and rendering markdown there would garble the
text. An explicit `markdown`/`plain` is the user's ingestion choice and is
trusted — the old toggle could also have produced those values, but there is
no evidence to distinguish them, and overriding a legitimate explicit choice
(e.g. the dialog's "Plain" pick on markdown-looking text) is the more visible
breakage. `html` without stored markup is impossible as a genuine source
(HTML ingestion always stores `html_source`), so it is always the stale
toggle and falls back to detection.

This matches the issue's rule: `format: null` / legacy derive via
`html_source.is_some()` or `detectFormat`; a contradictory stale `format` is
clamped at render.

Rejected: migrating history by rewriting stale `format` values. It needs one
ambiguous heuristic at load time, touches user data, and the render-time
clamp gives the same visible result with no migration; a later explicit
source-format feature can still normalize the data deliberately.

### 3. Detection stays in TypeScript; the tray path stays unclassified

`detectFormat` is the single source of truth for content detection
(preview-dialog spec), already used by the dialog and imports. The dialog-less
frontend paths pass `detectFormat(text)` into `add_text_entry`, so the entry
records what it was ingested as. `add_clipboard_entry` (system tray, Rust)
has no access to the TS detector; porting it would create two drifting
implementations, so it keeps `format: null`. The viewer's render-time
derivation covers that path with the same classification.

The frontend detection sits at the single `doAddEntry` seam (used by the Add
button, paste, plain fallback and import paths), so a new dialog-less caller
cannot silently fall back to `null`.

Rejected: adding a Rust port of `detectFormat` and detecting inside
`ingest_text`. Duplicated heuristics with no shared tests is the worse
failure mode, and the viewer must handle `null` anyway for existing files.

### 4. The viewer's view model

- `deriveSourceFormat(entry)` — immutable source as above.
- Allowed views: `plain` source → verbatim only; `markdown` / `html` →
  rendered + verbatim.
- Effective mode: plain source clamps to verbatim (a stored `rendered` is
  ignored); otherwise the stored `display_mode`, defaulting to `rendered`.
- Rendering: `plain` → `plainToWordHtml(original_text)`; rendered + `html` →
  `renderHtml(html_source)`, with `original_text` as the fallback for a legacy
  entry whose HTML source was inferred from content and has no stored markup;
  rendered + `markdown` → `renderMarkdown(original_text)`.
- The control is not rendered at all for a plain source; the labels come from
  the catalogs (`Markdown` / `HTML`, «Исходный» / "Raw").

The derivation lives in `src/lib/viewerDisplay.ts` as pure functions so the
whole table — including legacy and stale rows — is unit-testable without
mounting the viewer.

### 5. Command and schema surface

`set_entry_display_mode` writes only `display_mode` and emits
`entry_updated`; it deliberately does not touch `format`, `normalized_text`,
audio or timestamps, and needs no `Processing` guard because synthesis status
transitions use the compare-and-set storage update. The persisted JSON gains
one optional key; existing `history.json` files load unchanged, and re-saving
adds `display_mode: null`.

## Risks

- A legacy entry whose old toggle wrote `plain` or `markdown` over the true
  source is indistinguishable from an explicit ingestion choice, so it may
  render in that format. The wrong-view damage is limited to a formatting
  mismatch, and the user can switch to the verbatim view; no data is lost.
- `format: null` entries still re-run detection on every entry selection
  (memoized per entry object), acceptable for the text sizes involved.
