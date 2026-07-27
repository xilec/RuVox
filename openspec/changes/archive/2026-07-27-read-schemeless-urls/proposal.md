# Proposal: read-schemeless-urls

## Why

Phase 7 of the text pipeline only detects URLs with an explicit scheme
(`http(s)://`, `ftp://`, `ssh://`, `git://`). A scheme-less URL such as
`www.example.com` or `example.com/path` falls through to the English phase,
which transliterates each label separately and leaves the dots as literal
punctuation: `www.example.com` is read as "ввв.экзампл.ком" with the dots
swallowed as pauses and no "точка" separators. Technical texts mention bare
domains constantly, so a noticeable share of links reaches Silero mis-read.

## What Changes

- Detect scheme-less URLs in the pipeline's URL phase:
  - `www.`-prefixed domains (with optional path/query/fragment);
  - bare domains whose last label is a known TLD from the existing `TLD_MAP`
    (com, org, net, ru, io, dev, app, ai, co, me, uk, edu, gov, info, biz),
    with optional path/query/fragment.
- Read them like full URLs minus the scheme: domain labels joined with
  "точка" (known TLDs via `TLD_MAP`), path segments after "слэш", query
  after "вопросительный знак", fragment after "решётка", with the same
  transliteration and digit-run handling as schemed URLs.
- False-positive guards: the TLD must come from `TLD_MAP` (so `file.txt`,
  `test.spec.ts`, `config.yaml` are not matched) and must be alphabetic (so
  versions `1.2.3` and dates are not matched); matches inside emails and
  schemed URLs are excluded by phase order and boundary checks.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `text-pipeline`: the "URLs, emails, IP addresses, and file paths"
  requirement gains scheme-less URL detection and reading rules.

## Impact

- `src-tauri/src/pipeline/mod.rs` — new scheme-less URL regex, applied in
  phase 7 after schemed URLs and emails.
- `src-tauri/src/pipeline/normalizers/urls.rs` — entry point for normalizing
  a URL without a scheme (reuse of the existing domain/path/query/fragment
  logic).
- `src-tauri/tests/fixtures/pipeline/` — new golden fixtures (www-prefixed,
  bare domain with path, and false-positive guards: filenames, versions).
- Unit tests for the new detection and normalization paths.
