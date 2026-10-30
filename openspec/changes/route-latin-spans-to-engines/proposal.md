# Change: route Latin spans to the TeraTTS English frontend

## Why

TeraTTSv2 ships ten voices, six of them English, and its `<en>` tag reads
Latin natively. The RU-MVP profile (#308) still narrates every Latin fragment
through the Russian pipeline: `Kubernetes` becomes `кубернетис`, and
alphanumeric tokens expose a normalization gap — `IPv6` loses its digit in the
TeraTTS alphabet gate, while `HTTP/2` is half-narrated (`HTTPслэш два`). The
#311 listening probe found native prose, brands and identifiers acceptable, so
the engine can read them itself instead of the transliteration crutch.

## What Changes

- **TeraTTS profile**: `languages: ["ru", "en"]`, `latin_prose` and
  `identifier` become `Native`; numbers, units, URLs, emails and fenced code
  stay with the Russian pipeline. `segment_ms: 200`.
- **Pipeline**: alphanumeric abbreviations (`IPv6`, `HTTP/2`) are read as one
  Russian token on both routes, so a Native prose profile never emits a
  mixed-script fragment; a `/` inside a Latin token is no longer mistaken for
  a filesystem path.
- **Engine contract**: a defaulted `synthesize_spans` method carries the
  pipeline spans; only TeraTTS overrides it. Other engines keep their current
  Russian-only behavior.
- **`teratts` crate**: a `Language` parameter on the text frontend (`<ru>` +
  RUAccent vs `<en>` without accentuation), `synthesize_lang`, and
  `synthesize_segments` that concatenates language segments with the profile's
  segment pause, offsets timestamps into one timeline, and keeps codepoint
  ranges in whole-text coordinates. One selected voice reads both languages;
  only the language tag and the accentuation change.
- **App adapter**: spans become `Segment`s (Russian gaps and English spans),
  synthesis runs once with the configured `teratts_voice`, and the resulting
  timestamps map back through the existing `char_mapping`.
- **Tests**: pipeline span/normalization tests, adapter segment tests, a
  bundle-gated mixed RU/EN synthesis test, and the #311 probe harness.

## Non-goals

- Native routing for any other engine; the #311 verdicts for Piper and Silero
  stay with their own changes.
- Automatic language detection, translation, or native reading of URLs,
  emails and fenced code.
- A per-fragment abbreviation route; abbreviations are narrated as a whole.
- Changing the RU-MVP bundle or its voices.

## Impact

- Affected specs: `multi-language-synthesis` (new), `teratts-engine`,
  `text-pipeline`, `word-highlight`.
- Affected code: `teratts/` (text/engine API), `src-tauri/src/pipeline`,
  `src-tauri/src/tts/{engine,switcher,profiles,teratts}`, `commands`.
- User-visible: mixed RU/EN articles read by the same selected voice with
  correct English pronunciation; `IPv6` and `HTTP/2` now read correctly.
