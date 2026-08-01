# Tasks: Read leading-dot decimals

## Implementation

- [x] `src-tauri/src/pipeline/mod.rs` — new `re_leading_dot_decimal()`
  (`(^|[^\p{L}\p{N}_./\\])\.(\d+)`) and a Leading-dot decimals phase right
  after Versions: replace with boundary + `normalize_float("0." + digits)`.
- [x] Update the phase-order comment and the `process_numbers_tracked`
  guard comment to mention the new phase.

## Tests

- [x] Unit test: "Вес .5 кг" → "Вес ноль точка пять кг".
- [x] Unit test: ".75 вероятность" at text start → "ноль точка семь пять …".
- [x] Unit test: letter-preceded dot ("example.5") stays unread.
- [x] Unit test: "1.5" keeps its version-path reading (no double-consume).

## Validation

- [x] `nix develop -c cargo test --manifest-path src-tauri/Cargo.toml` green.
- [x] `nix develop -c just lint` green.
- [x] openspec validate read-leading-dot-decimals --strict green.
