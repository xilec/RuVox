## Why

Text ingestion is clipboard-only today (paste anywhere + the Add button). Users cannot add a local `.txt`/`.md`/`.html` file or a web page URL without manually copying its contents first. Issue #224 asks for file and URL imports that reuse the existing ingest/normalize path.

## What Changes

- **Drag & drop onto the whole window**: dropping one supported file (`.txt`, `.md`, `.html`/`.htm`) or an `http(s)` link opens the same ingestion flow as the clipboard paths; a full-window overlay («Отпустите, чтобы добавить») appears during drag-over.
- **Split-button «Добавить»**: the navbar Add button becomes a split-button whose dropdown menu holds three import actions — «Файл…», «Файл с кодировкой…», «По ссылке…». The primary click keeps today's clipboard behavior unchanged.
- **File reading with encoding handling**: files are read by a new backend command with BOM + statistical encoding detection (`encoding_rs`), normalizing legacy Cyrillic encodings (CP1251, KOI8-R, CP866, ISO-8859-5, MacCyrillic, …) to UTF-8. «Файл с кодировкой…» opens an extra dialog first: raw decoded-text preview plus a manual encoding dropdown (auto-detected value preselected) before continuing to the preview dialog.
- **URL fetching**: a new backend command fetches `http(s)` pages (scheme allowlist, response size cap ~10 MiB, connect/total timeouts) reusing the hardening pattern of `fetch_image_bytes`. The downloaded markup goes through the HTML extraction path; a best-effort heuristic detects JS-rendered (SPA) pages — little extracted text combined with script-dominated markup / empty framework mount points — and surfaces a clear «содержимое формируется JavaScript'ом» error instead of silent truncation; non-2xx responses surface the HTTP status.
- **Format routing**: for files the extension is authoritative (`.md` → markdown, `.html` → HTML path, `.txt` → plain); URLs and mismatched content fall back to format auto-detection (#241 detector once it lands).
- **Preview gate**: single-file and URL imports respect `preview_dialog_enabled` exactly like the Add button — enabled (default) means the preview dialog opens pre-filled; disabled means direct ingestion. Batch (multi-file) drops are out of scope (#242).

## Capabilities

### New Capabilities

- `text-import`: importing text from local files (with encoding detection and manual override) and from `http(s)` URLs into the queue via drag & drop or the split-button import menu, including backend file-reading and page-fetching commands and unsupported-source error reporting.

### Modified Capabilities

- `preview-dialog`: the Add flow gating requirement extends to the new import entry points — dropped/imported single sources follow the same preview-dialog-enabled/disabled decision as the clipboard Add flow.
- `ui`: the navbar Add control becomes a split-button with the import menu; the window gains a drag-over drop overlay.

## Impact

- **Frontend**: `src/components/AppShell.tsx` (split-button, drop overlay, Tauri drag-drop event wiring via `getCurrentWebview().onDragDropEvent`, import actions), new encoding dialog component, new URL-input dialog, ingest glue in `src/lib/` (pure decision functions extended alongside `addFlow.ts`/`ingest.ts`), i18n catalogs (`src/i18n/*`).
- **Backend**: new commands in `src-tauri/src/commands/` (`read_text_file` with encoding detection, `fetch_url_text`), shared hardened HTTP client already introduced for `fetch_image_bytes`; `encoding_rs` dependency.
- **IPC types**: `src/lib/tauri.ts` wrappers + typed errors through the existing `CommandError {type, code, params?, message?}` localization layer (new codes: unsupported extension, decode failure, SPA page, HTTP status, size cap).
- **No changes** to the normalization pipeline, storage schema, or TTS engines.

## Non-goals

- Multi-file batch ingestion (sequential entries, partial-success summary) — deferred to #242.
- JavaScript-rendered page support (headless rendering) — detected and reported as unsupported, not worked around.
- New source formats beyond plain text families (PDF, DOCX, …).
- Changes to the normalization pipeline itself; imports reuse it as-is.
