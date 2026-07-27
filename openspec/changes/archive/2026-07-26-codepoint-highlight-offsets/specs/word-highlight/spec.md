# Delta: word-highlight

## MODIFIED Requirements

### Requirement: Highlight application and styling

The system SHALL locate the rendered span whose `data-orig-start` /
`data-orig-end` range matches the active timestamp's `original_pos`
(preferring an exact match, falling back to the smallest containing span)
and add the `word-highlight` CSS class to it, removing the class from the
previously highlighted span. The `data-orig-start` / `data-orig-end`
attributes SHALL be expressed in Unicode codepoints, matching the
codepoint-based `char_map` contract of the position-mapping spec — an
astral-plane character (occupying two UTF-16 code units) SHALL advance the
offsets by exactly 1. The highlight background SHALL be
`rgba(255, 213, 0, 0.45)` in light mode and `rgba(255, 213, 0, 0.3)` in dark
mode (via the `--ruvox-highlight-bg` token), with an 80 ms transition.

#### Scenario: Highlight moves to the next word

- GIVEN word N is highlighted
- WHEN playback advances into word N+1
- THEN the `word-highlight` class moves from word N's span to word N+1's
  span

#### Scenario: Offsets after an astral character

- GIVEN the source text "Привет 🌍 мир" rendered for the viewer
- WHEN the span attributes are computed
- THEN the span for "🌍" carries `data-orig-start` / `data-orig-end` = 7 / 8
  and the span for "мир" carries 9 / 12 (codepoints), so a timestamp whose
  `original_pos` points at "мир" matches it exactly
