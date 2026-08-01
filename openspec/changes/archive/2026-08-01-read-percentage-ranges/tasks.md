# Tasks: Read percentage ranges

## Implementation

- [x] `src-tauri/src/pipeline/mod.rs` — new `re_percentage_range()`
  (`\b(\d+)\s*-\s*(\d+)\s*%`) and a Percentage ranges phase immediately
  before Percentages: `normalize_range` on the "N-M" part + " процентов".
- [x] Update the phase-order comment.

## Tests

- [x] Unit test: "Рост на 10-20% за квартал" → "от десяти до двадцати
  процентов", no bare "-"/"%".
- [x] Unit test: plain "10-20" and plain "20%" readings unchanged.
- [x] Golden fixture trio `range_percent.*` (input/expected/char_map).

## Validation

- [x] `nix develop -c cargo test --manifest-path src-tauri/Cargo.toml` green.
- [x] `nix develop -c just lint` green.
- [x] openspec validate read-percentage-ranges --strict green.
