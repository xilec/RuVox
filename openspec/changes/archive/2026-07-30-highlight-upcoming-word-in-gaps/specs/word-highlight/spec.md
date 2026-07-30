# Delta: word-highlight

## MODIFIED Requirements

### Requirement: Active word detection

On every `playback_position` event the system SHALL binary-search the
timestamp active at `position_sec` (a timestamp `t` is active when
`t.start <= position_sec < t.end`). A position falling in a gap between
words SHALL resolve to the closest upcoming word, so the next word is
highlighted ahead of its interval instead of the highlight blinking out
during a pause; only a position at or past the last word's `end` SHALL
produce no active word. The timestamp list MUST be sorted by `start`
ascending — the binary search silently misses matches otherwise; all
producers (ttsd, piper, silero-native) emit sorted lists, and the invariant
SHALL be asserted in development builds when timestamps are loaded.
Highlighting SHALL update only when the active word index changes, and MUST
be ignored when the event's `entry_id` does not match the displayed entry.

#### Scenario: Word under playback position

- GIVEN timestamps for the playing entry and a `playback_position` event
- WHEN `position_sec` falls inside a word's `[start, end)` interval
- THEN exactly that word's span receives the highlight

#### Scenario: Gap between words highlights the upcoming word

- GIVEN timestamps with a pause between word N and word N+1
- WHEN `position_sec` falls inside that gap
- THEN word N+1's span receives the highlight ahead of its interval

#### Scenario: Position past the last word

- GIVEN timestamps for the playing entry
- WHEN `position_sec` is at or past the last word's `end`
- THEN no span is highlighted

#### Scenario: Unsorted timestamps flagged in development

- GIVEN a development build and a `WordTimestamp[]` that is not sorted by
  `start`
- WHEN the timestamps are loaded for highlighting
- THEN a console assertion flags the violated invariant

#### Scenario: Event for another entry

- GIVEN the viewer displays entry A while entry B is playing
- WHEN a `playback_position` event for entry B arrives
- THEN no highlight changes in the viewer
