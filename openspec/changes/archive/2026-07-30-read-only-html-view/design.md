# Design: read-only-html-view

## Context

`TextViewer` renders entry content via `dangerouslySetInnerHTML` in three
modes (plain, markdown, HTML). HTML content is sanitized by DOMPurify at
ingestion and again at render (`src/lib/html.ts`), which already strips
scripts, `on*` handler attributes, `form`/`input`, and `javascript:` URIs —
but keeps `<a href>`, `<img>`, media elements, and other interactive markup.
The Tauri webview is a reader, not a browser: it has no native context menu,
link clicks navigate the app's own webview, and interactive elements are
meaningless. Decisions below were agreed with the user on the proposal:

- link click → fully block + tooltip with the URL (no external open);
- interactivity blocking applies in **all** display modes;
- "Копировать изображение" copies the **bitmap** (address as a second item);
- Ctrl+C/Cmd+C on a link copies its URL (as in GH issue #156).

## Goals / Non-Goals

**Goals:**

- All rendered content is inert: no navigation, no working form/media
  controls, in every display mode.
- Right-click context menu with copy actions: link address, selected text,
  image bitmap, image address — available in every display mode.
- Ctrl+C/Cmd+C copies the link URL when the selection/focus is on a link.
- Clipboard writes only via `@tauri-apps/plugin-clipboard-manager`.

**Non-Goals:**

- No "open link in external browser" action (opener plugin stays unused here).
- No editing/annotation features; the viewer stays strictly read-only.
- No changes to the TTS pipeline, storage, or Rust command surface.
- No native-looking menu — a Mantine-styled custom menu is fine.

## Decisions

### 1. Link blocking: delegated click handler on the viewer container

A single `click` listener on the viewer container (capture or bubble phase)
checks `e.target.closest('a')` and calls `preventDefault()` +
`stopPropagation()`. Delegation covers all modes and any nested markup
without per-link wiring, and survives re-renders because it is bound to the
container, not the content. Alternatives considered: rewriting `<a>` into
`<span>` at sanitize time — rejected, because we need the element + `href`
for the tooltip, the context menu, and the hotkey.

The URL tooltip uses the native `title` attribute path with the original
`href` verbatim — resolving relative links against the webview origin would
show a meaningless localhost URL.

### 2. Neutralizing interactive elements: post-render DOM pass

A `makeInert(root)` pass (`src/lib/inertContent.ts`) runs on the mounted
viewer container after React commits `dangerouslySetInnerHTML`, in **all**
display modes (single code path instead of one per render function):

- `button`, `select`, `textarea`, and any surviving `input` → set `disabled`
  and `tabIndex = -1`;
- `video`/`audio` → remove the `controls` attribute (element stays as a
  poster/thumbnail);
- `details`/`summary` → toggle clicks blocked by the delegated click handler;
- `<a>` → kept focusable (the Ctrl+C hotkey needs a focus target) and gets a
  `title` tooltip with the original `href` verbatim.

Sanitization stays the security boundary (DOMPurify); `makeInert` is a UX
layer on top, not a security mechanism. Alternative considered: extending
`FORBID_TAGS` with `button`/`video`/… — rejected, because the issue asks to
keep the elements visible (e.g. video poster), not delete them; `form`/`input`
stay forbidden as today.

### 3. Context menu: custom React component, opened on `contextmenu`

A `ViewerContextMenu` component listens for `contextmenu` on the container,
`preventDefault()`s, inspects the event target (closest `a`, `img`, current
`window.getSelection()`), and opens a Mantine `Menu` at the pointer
coordinates with the applicable items only:

- link under cursor → "Скопировать адрес ссылки" (`writeText(url)`);
- non-empty selection → "Копировать" (`writeText(selection)`);
- image under cursor → "Скопировать изображение" (bitmap, see D4) and
  "Скопировать адрес изображения" (`writeText(src)`).

Link and image addresses are copied verbatim from the source markup (no
resolution against the webview origin — relative links would otherwise turn
into meaningless localhost URLs). Only the image bitmap fetch resolves
relative/protocol-relative URLs (`//habrastorage.org/...`, `/img/...`) with
`new URL(href, document.baseURI)` before requesting.

### 4. Copying the image bitmap: tauri-plugin-http + clipboard `writeImage`

Webview `fetch()` on remote hosts would hit CORS, so the image bytes are
fetched through `tauri-plugin-http` (new frontend dependency + Rust plugin
registration) with a scoped `http:default` / fetch permission in
`capabilities/default.json`, then written via the clipboard plugin's
`writeImage`. Failures (network, non-image response, unsupported format)
surface as a red Mantine notification; the "address" item remains as the
always-available fallback.

New permissions in `src-tauri/capabilities/default.json`:
`clipboard-manager:allow-write-text`, `clipboard-manager:allow-write-image`,
`http:allow-fetch` (scoped per plugin defaults).

### 5. Hotkey: extend the existing keydown handler

`TextViewer` already owns a delegated `keydown` handler (Ctrl/Cmd+A). A
Ctrl/Cmd+C branch is added there: if `document.activeElement`/selection anchor
is inside an `<a>` within the container, copy `href` via `writeText` and
`preventDefault`; otherwise let the default copy proceed. This keeps both
hotkeys in one place instead of two competing listeners.

## Risks / Trade-offs

- [Remote image fetch blocked by host hotlink protection or network offline]
  → copy action fails with a notification; "Скопировать адрес изображения"
  always works.
- [New `tauri-plugin-http` dependency widens the permission surface]
  → permission scoped in `default.json`; fetch used only for image copy, no
  user-text-driven requests.
- [Intercepted clicks could break mermaid click-to-zoom, which also listens on
  the container] → the link branch only acts when `closest('a')` matches;
  mermaid SVGs contain no anchors, ordering verified in tests.
- [Word-highlight spans (`data-orig-*`) wrap link text, so `e.target` is a
  span, not the anchor] → all target detection uses `closest()`, never direct
  tag checks.
- [Markdown/plain modes contain few interactive elements] → the same handlers
  attach in all modes anyway, keeping behavior uniform and the code path
  single.
