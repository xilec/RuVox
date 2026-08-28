## Purpose

Lets users add narration sources from outside the clipboard: dropping a local
text file or a web-page link onto the window, or picking them explicitly from
the split-button import menu. Covers file reading with legacy-Cyrillic
encoding handling, page fetching, and unsupported-source error reporting;
ingested content flows through the existing normalization path unchanged.

## ADDED Requirements

### Requirement: Drag & drop ingestion

Dropping exactly one file with an extension of `.txt`, `.md`, `.html`, or
`.htm` onto the main window SHALL start the import flow for that file.
Dropping one `http(s)` link SHALL start the URL import flow for it. While a
drag hovers over the window, the system SHALL show a full-window overlay
(«Отпустите, чтобы добавить»); when the drag leaves without being released,
the overlay SHALL disappear and nothing SHALL be ingested. Dropping anything
else (zero items, several items, an unsupported extension, another scheme)
SHALL be ignored with no entry created.

#### Scenario: Text file dropped creates an entry through the normal flow

- GIVEN the main window is focused
- WHEN the user drops a single `.md` file onto it
- THEN the import flow starts with the file's contents routed like any other
  source (preview gate, then ingest/normalize) and no second entry appears

#### Scenario: Overlay follows the drag

- GIVEN the user drags a file over the window
- WHEN the pointer enters, moves across, and leaves the window without release
- THEN the drop overlay appears on entry, persists while hovering, disappears
  on leave, and no queue entry is created

#### Scenario: Unsupported drop is ignored

- WHEN the user drops three `.png` images onto the window
- THEN the overlay dismisses, no entry is created, and no error notification
  is shown

### Requirement: Split-button import menu

The navbar «Добавить» control SHALL be a split-button: the primary click
keeps the existing clipboard Add behavior unchanged, and the dropdown menu
SHALL offer three actions — «Файл…» (native file picker), «Файл с
кодировкой…» (file picker followed by the encoding dialog), and «По ссылке…»
(URL input). Each action SHALL feed the picked source into the same import
flow as drag & drop.

#### Scenario: Menu actions share one flow

- GIVEN the user opens the split-button dropdown
- WHEN they pick «Файл…» and select a `.txt` file
- THEN the source enters the same import flow (preview gate → ingest) that a
  dropped `.txt` file would take

### Requirement: File reading with encoding detection

The backend SHALL read imported text files through a dedicated command that
detects the character encoding from a BOM first and statistical analysis of
the bytes second, and SHALL always return decoded UTF-8 text together with
the encoding name it used. The detected set SHALL cover at least: UTF-8,
UTF-16 LE/BE, CP1251, CP866, ISO-8859-5, KOI8-R, KOI8-U, MacCyrillic,
Windows-1250, Windows-1252, ISO-8859-1, and ISO-8859-15. A file whose bytes
cannot be decoded into any supported encoding SHALL fail with a coded
«decode failed» error.

#### Scenario: Legacy CP1251 file reads correctly

- GIVEN a `.txt` file saved in CP1251 without a BOM
- WHEN the user imports it
- THEN the returned text contains proper Cyrillic letters (no mojibake), and
  the reported encoding is CP1251

#### Scenario: Undecodable file reports a coded error

- GIVEN a binary blob renamed to `.txt`
- WHEN the user imports it
- THEN a red notification with a localized decode-failure message is shown
  and no entry is created

### Requirement: Manual encoding override

The «Файл с кодировкой…» action SHALL open an encoding dialog before the
normalization preview: it shows a preview of the raw text decoded with the
auto-detected encoding (preselected in a dropdown listing all supported
encodings), and confirming re-decodes the file with the chosen encoding and
continues the import flow. Cancelling SHALL abort the import entirely.

#### Scenario: Wrong auto-detection can be corrected in place

- GIVEN a KOI8-R file that auto-detection misread as CP1251
- WHEN the user opens it via «Файл с кодировкой…», sees mojibake in the
  preview, picks KOI8-R in the dropdown, and confirms
- THEN the preview shows correct Cyrillic after switching and the import
  continues with the corrected decoding

### Requirement: URL fetching

Importing an `http(s)` link SHALL fetch the page through a dedicated backend
command that accepts only `http` and `https` schemes, caps the response at
~10 MiB, and enforces connect and total timeouts. The downloaded markup
SHALL be routed through the HTML extraction path; its format SHALL come from
format auto-detection. A non-2xx response SHALL surface a localized error
including the HTTP status; a network failure SHALL surface a localized fetch
error. The fetched content SHALL enter the same preview-gated flow as file
imports.

#### Scenario: Static article URL imports its visible text

- GIVEN a server-rendered article page
- WHEN the user drops its URL onto the window
- THEN the extracted readable text opens in the normalization preview (or is
  ingested directly when the gate is disabled)

#### Scenario: Server error surfaces the status

- WHEN the fetched URL responds with HTTP 403
- THEN a red notification names the HTTP status and no entry is created

### Requirement: JS-rendered page detection

When a fetched page yields little or no extracted text while its markup is
dominated by script bundles and empty framework mount points, the system
SHALL report a clear «содержимое страницы формируется JavaScript'ом — режим
не поддерживается» error instead of creating a near-empty entry. Pages whose
server markup already carries sufficient readable text SHALL NOT be flagged,
even if they also load additional content client-side.

#### Scenario: SPA shell is rejected with a clear message

- GIVEN a page whose HTML body is an empty mount-point div plus script tags
- WHEN the user imports its URL
- THEN the JavaScript-rendered error notification is shown and no entry is
  created

#### Scenario: Server-rendered page with hydration is not flagged

- GIVEN a page whose initial markup already contains the full article text
- WHEN the user imports its URL
- THEN the entry is created normally even though the page also ships script
  bundles

### Requirement: Import format routing

For imported files the extension SHALL decide the source format: `.md` →
markdown, `.html`/`.htm` → the HTML extraction path, `.txt` → plain text.
For URLs — and for file contents that contradict their extension (an
unreadable `.html`) — the system SHALL fall back to format auto-detection
over the raw text. Routed content SHALL reuse the existing ingest decision
layer and its sanitize/extract machinery.

#### Scenario: Extension decides for files

- WHEN the user imports `notes.md`
- THEN the entry is created with format markdown without consulting content
  sniffing

#### Scenario: URL falls back to detection

- WHEN the user imports a URL whose response is plain text served as
  `text/plain`
- THEN auto-detection classifies it and the entry is created with the
  detected format
