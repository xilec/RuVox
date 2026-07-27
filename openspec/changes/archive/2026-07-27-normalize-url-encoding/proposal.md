# Proposal: normalize-url-encoding

## Why

Percent-encoded and `+`-encoded characters in URLs and emails leak through the
pipeline literally: `%20` is read as "% двадцать", a UTF-8-encoded Cyrillic
file name (`%D1%84%D0%B0%D0%B9%D0%BB`) degrades into hex garbage
("%д один % восемьдесят четыре ..."), and `+` survives verbatim in query
strings and email local parts. Literal `%` and `+` in the output break the
pipeline's core invariant (Cyrillic-only text — Silero cannot read special
characters), and hex readings are meaningless noise instead of the content the
link actually carries.

## What Changes

- URLs (schemed and scheme-less) SHALL be percent-decoded per component before
  segment normalization: `%20` becomes a word separator, valid UTF-8 sequences
  (e.g. Cyrillic file names) decode to readable text, ASCII codes decode to
  their characters which are then read by the existing rules.
- `+` SHALL be read context-dependently: as a space (word separator) inside
  URL query components (form-urlencoded), and as the word "плюс" in email
  local parts and URL path segments.
- Invalid percent sequences (truncated `%2`, non-UTF-8 bytes) MUST NOT leak a
  literal `%`: the `%` is read as "процент" and the following characters are
  read normally.
- No literal `%` or `+` may remain in the normalized output.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `text-pipeline`: the "URLs, emails, IP addresses, and file paths"
  requirement gains percent-decoding and `+` handling rules for URL path /
  query / fragment components and email local parts.

## Impact

- `src-tauri/src/pipeline/normalizers/urls.rs` — decoding inside URL/email
  normalization; no new dependencies (decoder is a small pure function).
- Golden fixtures in `src-tauri/tests/fixtures/pipeline/` — new fixture(s)
  pinning percent-encoded URL readings; existing fixtures unchanged (no
  fixture contains `%` or `+` today).
- Char mapping: encoded triples collapse into shorter output spans; the
  mapping follows the existing "many input chars → one output span" pattern.

## Non-goals

- IDN / punycode (`xn--...`) decoding.
- Re-encoding or validating URLs; the pipeline reads text, it does not
  sanitize links.
- HTML entity decoding (`&amp;` etc.) — out of URL scope.
