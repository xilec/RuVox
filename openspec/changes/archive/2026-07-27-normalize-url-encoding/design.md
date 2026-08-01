# Design: normalize-url-encoding

## Context

`URLPathNormalizer` (`src-tauri/src/pipeline/normalizers/urls.rs`) renders URLs
and emails into speakable Russian. Structural parsing splits a URL into
scheme / authority / path segments / query pairs / fragment, then each
component goes through `transliterate_segment` (IT_TERMS → digraph
transliteration, digit runs). Neither percent-encoding nor `+` is handled:
`%` and `+` pass through into the output verbatim, and hex bytes are read as
numbers and letters.

Probe of current behavior (inputs → outputs):

- `…/hello%20world` → `хелло% двадцать ворлд`
- `…/%D1%84%D0%B0%D0%B9%D0%BB` ("файл") → `%д один % восемьдесят четыре …`
- `?q=hello+world` → `к равно хелло+ворлд`
- `user+tag@example.com` → `усер+таг собака …`

The URL/email regexes in `pipeline/mod.rs` already match addresses containing
`%` and `+`, so no detection changes are needed — only the rendering.

## Goals / Non-Goals

**Goals:**

- Percent-decode URL components so encoded content is read as the content
  itself: `%20` → word separator, UTF-8 runs (Cyrillic file names) → readable
  text, ASCII codes → their characters read by existing rules.
- Context-dependent `+`: space in query components (form-urlencoded), the
  word "плюс" in email local parts, path segments, and fragments.
- Invariant: no literal `%` or `+` in the normalized output.

**Non-Goals:**

- Punycode / IDN, HTML entities, URL validation or re-encoding.
- Changing URL/email detection regexes.

## Decisions

### Decode per component, after structural splits

Decoding happens inside `render_host_and_tail` / `normalize_email` **after**
the structural splits (`/`, `&`, `=`, `@`) and **before** the lexical ones
(`.` splitting, transliteration). A decoded `%2F` therefore never becomes a
path separator and `%3D` never splits a query pair — the URL structure is
taken from the raw form, exactly as a browser parses it.

Applied to: host labels, path segments, query keys and values, fragment,
email local part. (Host encoding is rare but decoding there is free and
removes the last `%` leak path.)

### Hand-rolled decoder, no new dependency

`percent_decode(input, plus_as_space) -> String`, ~40 lines: scans for `%`
followed by two hex digits, collects consecutive `%XX` bytes, UTF-8-decodes
each maximal run. `percent-encoding`/`url` crates are not current
dependencies and pull in more than needed for a pure text function.

### Invalid sequences never leak `%`

Policy, per character scan:

- `%` + two hex digits → candidate byte; consecutive bytes decoded as one
  UTF-8 run.
- A run that is not valid UTF-8 → fall back per byte: `%` read as "процент",
  the two hex chars kept as ordinary text (read by existing rules).
- `%` not followed by two hex digits (truncated, `%ZZ`) → read as "процент",
  following chars read normally.

This keeps the Cyrillic-only invariant on every input without a separate
error channel — normalization never fails, it just reads.

### Decoded punctuation is read, not leaked

Decoding can yield punctuation inside a component (`%2F` → `/`, `%28` → `(`,
Wikipedia-style names). These characters would otherwise pass through
`transliterate_word` verbatim and leak to TTS. A `DECODED_PUNCT` table maps
them to words with URL-context readings ('/' is "слэш", not "делить";
vocabulary otherwise mirrors the symbols normalizer). '.', '-', '_' are not
in the table — the regular chunk reading already handles them.

### `+` by component

- Query keys/values: `+` → space before decoding; decoded/literal spaces act
  as word separators (each word transliterated, joined by spaces).
- Email local part: `+` joins the existing separator set (`.`, `_`, `-`)
  read as words — rendered "плюс".
- Path segments, fragments, host: `+` → "плюс".

### Char mapping is unaffected structurally

URLs and emails are whole-span substitutions at the pipeline level; the
decoder only changes the text produced inside an already-collapsed span, so
no mapping-mechanics changes. Golden fixtures pin this.

## Risks / Trade-offs

- **Over-decoding**: a URL deliberately showing `%20` as text will now be
  read decoded. Acceptable — the encoded form is never the intended reading
  for narration.
- **Double-encoded input** (`%2520`) decodes once, to `%20` → "процент
  двадцать". Correct per single-decode semantics; not worth recursion.
- **Decoded `.` inside a segment** reads as "точка" (it goes through the
  existing dot-splitting). Rare and the reading stays sensible.
