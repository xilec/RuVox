# Tasks: normalize-url-encoding

## 1. Percent decoder

- [x] 1.1 Add `percent_decode(input: &str, plus_as_space: bool) -> String` in
  `src-tauri/src/pipeline/normalizers/urls.rs`: scan for `%` + two hex digits,
  decode maximal `%XX` runs as UTF-8; invalid runs / truncated `%` fall back
  to reading `%` as "процент" with following chars kept as text
- [x] 1.2 Unit tests for the decoder: valid ASCII (`%20`, `%2B`, `%25`),
  UTF-8 Cyrillic run, truncated `%2`, non-hex `%ZZ`, non-UTF-8 bytes,
  `plus_as_space` on/off

## 2. Wire decoding into URL/email rendering

- [x] 2.1 Decode path segments, query keys/values, fragment, and host labels
  in `render_host_and_tail` (query with `plus_as_space: true`); decoded
  spaces split query values into words read separately
- [x] 2.2 Read `+` as "плюс" in path segments and fragments
- [x] 2.3 Decode email local part in `normalize_email` and add `+` to its
  separator set, read as "плюс"

## 3. Tests

- [x] 3.1 Unit tests in `urls.rs` for every delta-spec scenario:
  `%20` in path, percent-encoded Cyrillic name, `+` in query, `+` in email,
  `%25` + truncated `%2`, `+` in path segment
- [x] 3.2 Add a golden fixture with a percent-encoded URL (Cyrillic file
  name + query with `+`) under `src-tauri/tests/fixtures/pipeline/`
- [x] 3.3 Assert no literal `%` or `+` survives in the new tests' outputs

## 4. Gates

- [x] 4.1 `cargo fmt`, `cargo clippy --no-deps -- -D warnings`,
  `cargo test --manifest-path src-tauri/Cargo.toml` — all green
- [x] 4.2 `pnpm dlx @fission-ai/openspec validate normalize-url-encoding --strict`
