# Proposal: read-only-html-view

## Why

In HTML mode (and markdown mode) the `TextViewer` renders sanitized HTML via
`dangerouslySetInnerHTML`, but the webview is not a browser: clicking a link
navigates the app's own webview (or triggers an external handler), interactive
elements (buttons, inputs, media controls, `<details>`) stay operable in a
read-only reader, and there is no way to copy a link URL, selected text, or an
image — the webview has no native context menu. Found during manual testing of
the HTML view branch (GH issue #156).

## What Changes

- **Read-only viewer (all display modes: plain, markdown, HTML).**
  - Link navigation is blocked: clicks on `<a>` inside the viewer are
    intercepted and `preventDefault`-ed; the link URL is shown in a tooltip.
  - Interactive elements are neutralized in rendered content: form controls
    (buttons, inputs, selects, textareas) disabled, media `controls` stripped,
    `<details>` inert, inline `on*` handlers covered explicitly (on top of
    DOMPurify script stripping).
- **Custom context menu (right-click) with copy actions, in all modes:**
  - on a link: "Скопировать адрес ссылки";
  - on selected text: "Копировать";
  - on an image: "Копировать изображение" (fetches the image and writes the
    bitmap to the clipboard) and "Скопировать адрес изображения".
- **Hotkey Ctrl+C / Cmd+C**: when the selection/focus is on a link, copies the
  link URL instead of the default copy behavior.
- **Clipboard writes** go through `@tauri-apps/plugin-clipboard-manager`
  (already used for reads); image copy adds `writeImage` and HTTP fetch
  permissions to the Tauri capabilities.

## Capabilities

### New Capabilities
- `viewer-copy-actions`: custom context menu and hotkeys for copying link
  addresses, selected text, and images (bitmap + address) from the viewer,
  via the clipboard plugin.

### Modified Capabilities
- `text-display`: the viewer becomes strictly read-only — link navigation
  blocked with a URL tooltip, and interactive elements in rendered content are
  inert, in all display modes.

## Impact

- `src/components/TextViewer.tsx` — click interception, context menu, hotkey
  handling.
- `src/lib/html.ts` (sanitization / post-processing of rendered content) and
  possibly `src/lib/markdown.ts` — neutralizing interactive elements.
- New `src/lib/` or component module for the context menu and copy logic.
- `src-tauri/capabilities/` — new permissions: clipboard `write-image`, HTTP
  fetch for remote images.
- Frontend unit tests (`pnpm test:unit`) for the new logic; no pipeline, no
  Rust behavior changes.
