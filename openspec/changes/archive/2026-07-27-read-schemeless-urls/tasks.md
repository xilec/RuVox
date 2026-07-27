# Tasks: read-schemeless-urls

## 1. Normalizer entry point

- [x] 1.1 In `src-tauri/src/pipeline/normalizers/urls.rs`, extract the
  post-scheme rendering of `normalize_url` (authority/path/query/fragment)
  into a shared private helper and add `normalize_schemeless(&self, url:
  &str) -> String` that renders the same parts without the protocol and
  "двоеточие слэш слэш" prefix (design D1)
- [x] 1.2 Unit tests for `normalize_schemeless`: `www.example.com`,
  `example.com/path/to/page`, `docs.python.org/3/tutorial`, query and
  fragment, digit runs in segments, trailing `www` label transliteration

## 2. Detection and pipeline wiring

- [x] 2.1 Add `re_schemeless_url()` in `src-tauri/src/pipeline/mod.rs`:
  generic dotted-labels candidate regex (built once via `OnceLock`) with the
  accept/reject decision in the phase-7 closure — first label `www` or last
  label in `TLD_MAP` via `is_known_tld`; skip path-internal segments
  (preceded by `/` or `\`) — final design, see design D2
- [x] 2.2 Apply it in phase 7 after the schemed-URL and email passes,
  reusing the trailing-punctuation trim-and-reappend step (share the helper
  with the schemed-URL closure if trivial, otherwise duplicate the two
  lines)

## 3. Tests and fixtures

- [x] 3.1 Pipeline-level tests (mod.rs tests or integration): www-prefixed,
  bare domain with path, bare domain followed by sentence punctuation,
  domain inside an email not re-matched, domain inside a schemed URL not
  re-matched
- [x] 3.2 Golden fixture(s): `url_schemeless` (www + bare domain with path
  in prose) and false-positive guards (`file.txt`/`config.yaml` filenames
  and `1.2.3` version keep their current reading)

## 4. Gates

- [x] 4.1 `nix develop -c cargo fmt --manifest-path src-tauri/Cargo.toml`
- [x] 4.2 `nix develop -c cargo clippy --manifest-path src-tauri/Cargo.toml --no-deps -- -D warnings`
- [x] 4.3 `nix develop -c cargo test --manifest-path src-tauri/Cargo.toml`
  (unit + golden)
