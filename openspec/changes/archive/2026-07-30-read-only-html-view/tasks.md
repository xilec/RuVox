# Tasks: read-only-html-view

## 1. Read-only rendering

- [x] 1.1 Add a `makeInert(root)` post-render pass (`src/lib/inertContent.ts`, applied on the mounted viewer container): disable `button`/`select`/`textarea`/surviving `input`, remove `controls` from `video`/`audio`, neutralize `details` toggling; keep `<a>` focusable
- [x] 1.2 Apply the neutralization in all display modes via a single container-level pass in `TextViewer` (covers markdown and HTML output alike)
- [x] 1.3 Delegated `click` handler on the `TextViewer` container: `closest('a')` → `preventDefault` + `stopPropagation`; verify mermaid click-to-zoom is unaffected
- [x] 1.4 Show the link's original `href` verbatim on hover (`title` set during the inert pass); click-listener effect deps on `entry?.id` — regression fix: with empty deps it never attached when the first mount had no entry
- [x] 1.5 Unit tests: disabled controls, media without controls, link tooltip URL, links stay focusable, URL resolution (`src/lib/inertContent.test.ts`)

## 2. Context menu and copy actions

- [x] 2.1 Add `tauri-plugin-http` (Rust plugin registration in `src-tauri`, JS dependency in `package.json`) and extend `src-tauri/capabilities/default.json` with `clipboard-manager:allow-write-text`, `clipboard-manager:allow-write-image`, scoped `http:allow-fetch`; update `flake.nix` pnpmDeps hash
- [x] 2.2 Implement copy helpers (`src/lib/viewerCopy.ts`): `copyLinkAddress`, `copySelection`, `copyImageBitmap` (fetch via http plugin → `Image.fromBytes` → clipboard `writeImage`), `copyImageAddress`; URL resolution via `new URL(href, document.baseURI)` (`src/lib/urls.ts`); failures → red Mantine notification
- [x] 2.3 Implement `ViewerContextMenu` component: `contextmenu` on the container → `preventDefault`, detect target (`closest('a')` / `closest('img')` / current selection), open Mantine `Menu` at pointer with applicable items only (labels in Russian)
- [x] 2.4 Wire the context menu into `TextViewer` for all three display modes
- [x] 2.5 Unit tests: clipboard write calls (mocked plugin), URL resolution, error notifications (`src/lib/viewerCopy.test.ts`); menu visibility per target type — manual pass (no component-test infra in repo)

## 3. Hotkey

- [x] 3.1 Extend the existing Ctrl/Cmd+A keydown handler in `TextViewer.tsx` with a Ctrl/Cmd+C branch: focus/selection on a link → copy resolved URL + `preventDefault`, otherwise default behavior
- [x] 3.2 Manual verification of the hotkey (confirmed in the manual pass, task 4.3)

## 4. Verification

- [x] 4.1 `nix develop -c pnpm test:unit` and `nix develop -c pnpm typecheck` green
- [x] 4.2 `nix develop -c just lint` green (eslint, knip — new deps used)
- [x] 4.3 Manual pass: run the app, load an HTML entry (e.g. with remote images), verify link blocking + tooltip, context menu items, image bitmap copy, Ctrl+C on a link — confirmed by the user on tmp/testHtml1.txt
