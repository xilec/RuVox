# Design: read-schemeless-urls

## Context

Phase 7 (`src-tauri/src/pipeline/mod.rs`) runs four `tracked.sub` passes in
order: schemed URLs (`re_url`: `https?|ftp|ssh|git://...`), emails, IPv4,
file paths. A span replaced by one pass is invisible to all later passes and
phases (`TrackedText` skips matches overlapping replaced regions). The whole
reading logic already lives in `URLPathNormalizer::normalize_url`
(`src-tauri/src/pipeline/normalizers/urls.rs`): scheme → protocol table,
domain labels → "точка" with `TLD_MAP`, port, path segments → "слэш", query
→ "вопросительный знак", fragment → "решётка", with transliteration and
digit-run splitting throughout.

Scheme-less URLs currently slip past all four passes and land in the English
phase, which transliterates label-by-label and leaves literal dots:
`www.example.com` → "ввв.экзампл.ком".

## Goals / Non-Goals

**Goals:**

- Detect `www.`-prefixed domains and bare domains ending in a `TLD_MAP` TLD,
  including optional path/query/fragment.
- Read them through the same code path as schemed URLs (minus the scheme
  part), so transliteration, digit runs, and punctuation wording stay
  identical.
- No false positives on filenames (`file.txt`, `test.spec.ts`,
  `config.yaml`), versions (`1.2.3`, `v2.3.1`), dates, or decimals.

**Non-Goals:**

- Detecting arbitrary domains with unknown/TLD-less suffixes (`localhost`,
  `intranet.local`, `example.technology`) — unknown suffixes stay as today.
- Percent-decoding, IDN/punycode handling.
- Emails/IPs/paths behavior — unchanged.

## Decisions

### D1: Reuse `normalize_url` by giving the match a synthetic scheme

`normalize_url` already splits scheme/authority/path/query/fragment and
handles every sub-part. Instead of duplicating that logic for scheme-less
input, add a thin entry `normalize_schemeless(url: &str)` that renders the
domain/path/query/fragment exactly like `normalize_url` but skips the
protocol and "двоеточие слэш слэш" prefix. Implementation: extract the
post-scheme rendering into a shared private helper taking `(rest: &str)`
(i.e. everything after `scheme://`); `normalize_url` calls it after pushing
the scheme parts, `normalize_schemeless` calls it directly. One home for the
reading rules; the two call sites differ only in the prefix.

Alternative considered: a separate hand-rolled renderer — rejected, it
duplicates the domain/path/query logic and would drift (one-home rule for
normalization tables).

### D2: Generic candidate regex, TLD/www validation in the closure

New `re_schemeless_url()` in `mod.rs` matches any dotted-labels candidate
(`\b(?:[a-zA-Z0-9-]+\.)+[a-zA-Z0-9-]+(?:/[^\s<>"'\)]*)?`) and is applied in
phase 7 **after** schemed URLs and emails (both consume their spans first;
`TrackedText` overlap protection does the rest). The accept/reject decision
lives in the closure, not the regex:

- accept if the first host label is `www` (any suffix), or the last host
  label is in `TLD_MAP` (checked via `is_known_tld`);
- reject otherwise: filenames (`file.txt`, `test.spec.ts` — suffix not in
  the map), versions and dates (`1.2.3` — numeric last label), unknown-TLD
  domains (`example.com.evil` — the generic regex matches the whole dotted
  chain, so a TLD in the middle does not trigger a partial match);
- reject candidates immediately preceded by `/` — a dotted segment inside a
  file path (`/home/site.dev/main.py`) is a directory, not a domain; the
  path pass (which runs later) then consumes the whole path.

Notes:

- Putting the TLD alternation in the regex (the initially considered
  option) was rejected: it cannot express "last label of the whole dotted
  chain", so `example.com.evil` would half-match, and the map would be
  encoded in two places (one-home rule). The closure check keeps `TLD_MAP`
  the single source.
- `\b` on both ends plus the earlier email pass keep the regex out of email
  addresses; the schemed-URL pass consumes full URLs first.
- Trailing sentence punctuation is stripped by the same
  `split_trailing_punct` helper used for schemed URLs.

### D3: Reading of "www"

The `www` label is transliterated like any other alpha label (→ "ввв").
No special-casing: common Russian practice reads it as one word, and the
letter-name question for single letters is owned by issue #120.

### D4: Detection set is fixed to `TLD_MAP`

The whitelist is exactly the existing `TLD_MAP` keys — no second TLD table
to maintain (one-home rule). Extending TLD coverage later means extending
`TLD_MAP` in one place.

## Risks / Trade-offs

- [False positives on dotted names that end in a TLD word: `main.app`,
  `story.dev`, `loader.co`] → Accepted: these are genuinely ambiguous, the
  reading ("мейн точка апп") stays close to how a user would say them, and
  the TLD whitelist keeps the class small. Golden fixtures pin the guards
  for the clear-cut cases (filenames, versions, dates).
- [`www.` match without a dot after the host (`www.example`)] → The regex
  requires at least one more dotted label for the www-alternative
  (`www\.label(\.label)*` matches `www.example` too) — accepted: reading
  "ввв точка экзампл" is reasonable for such input.
- [Regex catches a bare domain inside a longer URL path (`example.com` in
  `https://x.com/redir?to=example.com`)] → Schemed pass runs first and
  consumes the whole URL; overlap protection blocks the inner match.

## Open Questions

(none)
