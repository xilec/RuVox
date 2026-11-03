# Design — reading width and reading scale

## Context

The viewer renders every display mode inside one `.content` container
(`src/components/TextViewer.tsx` + `TextViewer.module.css`) whose base size is
the fixed `--ruvox-reading-font-size: 17px`. At 1440 px the markdown/HTML
content spans the full pane, roughly 110–120 characters.

#354 asked for a narrow column plus font-size presets. #357 showed that a
font-size control only reaches text: HTML headings keep their `em` ratios and
tables/images keep their natural size, so the document's internal proportions
change instead of the whole page growing as in browser zoom. Its decision
(point 6) is the constraint implemented here: one scale factor replaces the
font-size presets, and only the width control remains from #354.

## Decisions

### 1. One scale factor, applied as CSS `zoom` on `.content`

`zoom: var(--ruvox-reading-scale)` (a unitless multiplier set inline as
`reading_scale / 100`) is applied to the same content container in plain,
Markdown and HTML modes. Text, headings, code, tables, borders and images all
scale by the same factor, and the layout keeps the pane width: percentages
inside the zoomed subtree resolve against the zoomed coordinate space, so a
`100%`-wide column still fits the pane and the text reflows.

`transform: scale()` is rejected for the reasons in #357: it keeps the original
layout box, so scrollbars, hit-testing and `scrollIntoView`/
`getBoundingClientRect` (used by word highlighting) drift, and the content
overflows the pane instead of reflowing.

Verified on the actual engine before implementing: a WebKitWebDriver session
against MiniBrowser on WebKitGTK **2.54.1** (the nixpkgs version the repo builds
with) reported

- `zoom: 2` on a 100×50 box → `getBoundingClientRect()` 200×100, computed
  `zoom: "2"`;
- `zoom: var(--z)` with `--z: 1.5` → 150×75 (the custom-property indirection
  used here works);
- `zoom: 2; width: 50%` → 633 px in a 1266 px viewport, and its `100%`-wide
  child also 633 px — i.e. zoom does not overflow the pane, the layout box is
  scaled and percentage widths keep resolving against the parent.

Residual risk: `zoom` is only now being standardized (css-viewport-1); the
WebView2/WKWebView engines were not exercised here, and a webview update could
change the computed behavior. The manual checklist covers 80/150/200 % in the
Tauri build, including word-highlight scrolling at 200 %. If a future webview
ignores `zoom`, `--ruvox-reading-scale` is the seam for the text-only
font-size fallback discussed in #357 (open question 3) — degraded, but the
config and controls stay unchanged.

### 2. Narrow width as a token-driven `max-width`, wide as today's layout

Narrow mode adds `max-width: min(var(--ruvox-reading-measure), 100%);`
`margin-inline: auto` to `.content`; wide mode removes both. `70ch` lives in
`src/globals.css` as `--ruvox-reading-measure` so the measure is a design token
(the styling rule forbids literal measurements in CSS Modules where a token
exists) and can be tuned without touching the component.

The container uses Mantine's global `border-box`, so the 1 rem padding is
inside the measure (about 68.6 characters of text in a 70ch column). Adding the
padding to the token (`calc(70ch + 2rem)`) was rejected: with the `100%`
clamp it can overflow a narrow pane by 2 rem.

Width and scale are independent, as #357's open question 4 recommends: width is
about line length, scale is about size. `zoom` keeps the character measure at
70ch of the zoomed font, so at 200 % the narrow column still holds ~70
characters until the pane clamp takes over and the text reflows — exactly the
browser-zoom behavior.

### 3. Wide tables scroll in a wrapper, not on the table itself

`pre` scrolls on its own with `overflow-x: auto`. For tables that property does
not create a scroll container: a WebKitGTK probe with a 300 px parent showed the
table stay at its 1308 px max-content width and overflow the parent —
`overflow-x: auto` on the `<table>` never produced a scroller. The markdown
(`table_open`/`table_close` renderer rules) and HTML (`renderHtml`, which
already walks the DOM for highlighting) renderers therefore wrap every table in
`<div data-scroll-x>`; the CSS module gives that box `overflow-x: auto;
max-width: 100%`, while the table keeps its existing `width: 100%` styling.
Rejected: `display: block` on the table — it does scroll, but loses the
full-width layout for narrow tables and changes the table's box model.

### 4. Config: string canonicalization + clamped percent

`reading_width` mirrors `code_block_mode`: a `String` field with a serde
default and a `UIConfig::canonical_reading_width` helper that maps `"wide"` to
itself and everything else to `"narrow"`, applied in `load_config` and in
`apply_config_patch`. `reading_scale` is a `u32` with a default of 100 and
`UIConfig::clamp_reading_scale` clamping to 80–200 on load and patch. Storage
does not snap values to the UI ladder: a hand-edited in-range value (e.g. 93)
stays valid, and the stepper walks to the nearest ladder steps around it.

Rejected: an enum for the width. Only two values exist, the pipeline never
consumes it, and a `String` keeps the config schema and the TS IPC type
symmetric with the existing mode fields. Rejected: storing the scale as a
float. Percent steps are integers, the issue specifies percent, and `u32` plus
clamping makes out-of-range values trivially safe.

### 5. Persistence paths: immediate in the viewer, batched in Settings

The toolbar controls are the fast path: every click applies optimistically and
calls `update_config` with just that field, mirroring `code_block_mode`'s
persist pattern and the display-mode switch (rollback + localized error toast
on a rejected promise). Settings keeps the dialog's existing batched
save-on-submit model — the new fields are form fields included in the same
`UIConfigPatch` as theme/language/engine, so a user who never touches the
toolbar sees no behavior change.

`TextViewer` receives the two values as props from `AppShell` (which already
loads `get_config()` and refreshes it through Settings' `onSaved`). This keeps
the viewer free of its own config fetch and lets tests drive the persisted
state directly; local state seeds from the props so an optimistic change
survives unrelated re-renders, and the effect re-syncs when a Settings save
lands.

Rejected: a config event/subscription — no other UI setting needs one today,
and the Settings `onSaved` refresh plus the viewer's local optimistic state
already keep both surfaces consistent.

### 6. Accessibility and labels

`Узко | Широко` is a Mantine `SegmentedControl` (native radio inputs: tab into
the group, arrows switch), the `−`/`+` are native buttons, and the percentage
is a button that resets to 100 %. Each has a localized accessible name from the
RU/EN catalogs; the group label is «Ширина чтения» / «Reading width». Bounds
are expressed with `disabled`, not hidden, so keyboard/screen-reader users get
the same state.
