# Tasks

## 1. Routing

- [x] 1.1 Run the construct claim on both prose routes: `needs_context` and the
      span claim stop gating on `NumberRoute::Narrate`.
- [x] 1.2 Render a claimed construct per route: `Narrate` keeps
      `english::transliterate`, `Native` pushes the Latin words, records the
      `claimed` range and the `native_ranges` entry.

## 2. Tests

- [x] 2.1 Pipeline tests: a native-route construct reads as Latin English
      words with one prose-Latin span and a mapping back to the construct; a
      Russian sentence keeps the Russian reading on both routes; the narrated
      output is unchanged.
- [x] 2.2 TeraTTS engine test: an English paragraph with a percentage, a size
      unit, a version, a range and a date yields one `en` segment and no
      Russian words.
- [x] 2.3 Perf: the 1 MB budgets keep holding.

## 3. Fixtures and docs

- [x] 3.1 TeraTTS golden fixture with the same English paragraph as the
      narrated #371 fixture and its Russian counterpart.
- [x] 3.2 `CHANGELOG.md` `[Unreleased]` note.

## 4. Validation

- [x] 4.1 `cargo test -p tts-normalize` (lib and corpus suites).
- [x] 4.2 `cargo clippy -p tts-normalize --all-targets -- -D warnings` and
      `cargo fmt --all`.
- [x] 4.3 Golden fixtures and perf budgets.
- [x] 4.4 Archive the change and sync the delta specs.
