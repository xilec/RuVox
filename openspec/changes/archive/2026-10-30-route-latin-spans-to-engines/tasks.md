# Tasks

## 1. Probe and policy

- [ ] 1.1 Record the #311 probe: pipeline inputs (`teratts_en_probe` test),
  native/narrated WAVs (`teratts/export/en_probe.py`), and the verdict
  (prose/brands/identifiers native; URL/email/code narrated) in issue #311.
- [x] 1.2 Update `TERATTS_PROFILE` to `languages ["ru","en"]`,
  `latin_prose/identifier: Native`, `segment_ms: 200`; update the registry
  test that asserted no engine routes Latin natively.

## 2. Pipeline boundary

- [x] 2.1 Add the alphanumeric-abbreviation phase (`IPv6`, `HTTP/2`) before
  symbols/numbers; read `/` as `слэш` and digits as words.
- [x] 2.2 Make the path phase skip a slash preceded by a word character;
  regression test for `HTTP/2`.
- [x] 2.3 Tests: both routes produce no mixed-script `IPv`/`HTTP`; a
  `Native` profile still emits prose/identifier spans.

## 3. Crate language support

- [x] 3.1 `Language` enum; `text::prepare` tags `<ru>`/`<en>` and accents only
  Russian; unit tests for both.
- [x] 3.2 `Synthesis.samples`; `synthesize_lang`; `synthesize_segments`
  (segment pause, timestamp audio offset, `text_start` char offset).
- [x] 3.3 Bundle-gated mixed test: RU + EN + RU, monotonic timestamps,
  absolute English char range, 44.1 kHz WAV.
- [x] 3.4 Parity test still passes for the RU path (no audio change).
- [x] 3.5 Short Russian connector segments use a reduced duration scale so a
  one-letter word between English fragments is not stretched; bundle-gated
  regression test.
- [x] 3.6 Punctuation-only chunks and leading punctuation are not synthesized:
  the chunker merges a wordless part into the previous chunk and the adapter
  trims a segment's leading punctuation; regression tests.

## 4. App integration

- [x] 4.1 `TtsEngine::synthesize_spans` default; switcher delegates; adapter
  overrides and builds segments from spans.
- [x] 4.2 Pipeline normalization returns spans; synthesis passes them and uses
  the configured `teratts_voice` for every segment.
- [x] 4.3 Adapter unit tests: prose span splits into RU/EN/RU with absolute
  offsets; no spans -> one RU segment; dictionary override stays RU.
- [x] 4.4 One voice for both languages: no English voice config or selector;
  `Language` changes only the tag and accentuation.
- [ ] 4.5 Word highlighting: mixed timestamps map through `char_mapping`;
  verify no gap/overlap regression in the bundle-gated mixed smoke.

## 5. Docs and validation

- [x] 5.1 README/README.en: mixed RU/EN reading with one voice, `IPv6` fix.
- [x] 5.2 CHANGELOG `[Unreleased]` entry.
- [ ] 5.3 `cargo fmt`, clippy (`src-tauri`, `teratts`), `cargo test`
  (src-tauri, teratts with `TERATTS_BUNDLE`), `pnpm typecheck`,
  `pnpm test:unit`, `pnpm lint`, `openspec validate --strict`.
- [x] 5.4 Manual pass: mixed article in the app (prose/brands/identifiers
  read in English by the same voice, URL/email/code Russian, `IPv6`/`HTTP/2`
  correct, highlight continuous); archive the change.
