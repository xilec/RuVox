# Delta: word-highlight

## MODIFIED Requirements

### Requirement: Approximate word timestamps from an engine without native alignment

An engine that exposes no native word alignment SHALL be allowed to produce
approximate word timestamps, provided the producer guarantees the existing
highlight invariants: the list is sorted by `start` ascending, every word has
`start < end`, consecutive words do not overlap, and the last `end` is at most
the audio duration. Sentence boundaries SHALL be exact (each sentence is
synthesized as its own bounded chunk, so its audio range is known); word
boundaries inside a sentence MAY be approximate, and the producing engine's
capability spec SHALL document the approximation and its typical lag.

A mixed-language utterance SHALL produce one timestamp list for the whole
utterance: the engine offsets each language segment's words by the audio and
codepoint position already produced, so the highlight crosses a language
boundary without a reset, gap or overlap. The mapped `original_pos` ranges
SHALL stay inside the original document.

The highlighting behavior SHALL NOT change: the active word is still found by
binary search over `start`, a position in a gap still resolves to the closest
upcoming word, and a position at or past the last word's `end` still produces
no active word.

#### Scenario: Approximate timestamps drive the highlight

- GIVEN an entry whose timestamps come from an engine without native alignment
- WHEN playback positions arrive
- THEN the active word is highlighted by the same binary search as for exact
  timestamps, and sentence boundaries land on the chunk boundaries

#### Scenario: Producer invariants are asserted on load

- GIVEN a timestamps file whose entries are not sorted or overlap
- WHEN the timestamps are loaded in a development build
- THEN the invariant assertion fails instead of the highlight silently
  missing matches

#### Scenario: Documented lag does not break the highlight

- GIVEN word boundaries that lag the spoken audio by 0.2–0.4 s
- WHEN the position falls inside a word's interval or in the pause after it
- THEN the word stays highlighted while it is spoken, and the next word is
  highlighted ahead of its interval in the pause

#### Scenario: Highlight crosses a language boundary

- GIVEN an entry with Russian, English and Russian segments
- WHEN playback moves from the last Russian word into the English word
- THEN the English word is highlighted at its absolute timestamp and the
  following Russian word continues without a gap or overlap
