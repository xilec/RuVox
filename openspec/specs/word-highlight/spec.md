# Word Highlight Specification

## Purpose

Covers word-level highlighting of the displayed text synchronized with audio
playback: loading word timestamps, mapping playback positions to the active
word, applying and clearing the highlight, auto-scrolling, and the lifecycle
of the highlight across pause, stop, entry switches, and display-mode
switches.

## Requirements

### Requirement: Timestamp loading

When playback of an entry starts, the system SHALL load that entry's
`WordTimestamp[]` via the `get_timestamps` command. The system SHALL also
prefetch timestamps as soon as the selected entry has a `timestamps_path`,
so highlighting works even when the `playback_started` event races the
frontend event subscription. Timestamps SHALL be cached per entry and
refetched when the playing entry changes.

#### Scenario: Highlight starts with autoplay

- GIVEN an entry whose timestamps exist on disk and playback starts
  automatically (read-now add)
- WHEN the first `playback_position` events arrive
- THEN the active word is highlighted without requiring a stop/play cycle

### Requirement: Active word detection

On every `playback_position` event the system SHALL binary-search the
timestamp active at `position_sec` (a timestamp `t` is active when
`t.start <= position_sec < t.end`). Positions falling in a gap between words
SHALL produce no active word. Highlighting SHALL update only when the active
word index changes, and MUST be ignored when the event's `entry_id` does not
match the displayed entry.

#### Scenario: Word under playback position

- GIVEN timestamps for the playing entry and a `playback_position` event
- WHEN `position_sec` falls inside a word's `[start, end)` interval
- THEN exactly that word's span receives the highlight

#### Scenario: Event for another entry

- GIVEN the viewer displays entry A while entry B is playing
- WHEN a `playback_position` event for entry B arrives
- THEN no highlight changes in the viewer

### Requirement: Highlight application and styling

The system SHALL locate the rendered span whose `data-orig-start` /
`data-orig-end` range matches the active timestamp's `original_pos`
(preferring an exact match, falling back to the smallest containing span)
and add the `word-highlight` CSS class to it, removing the class from the
previously highlighted span. The highlight background SHALL be
`rgba(255, 213, 0, 0.45)` in light mode and `rgba(255, 213, 0, 0.3)` in dark
mode (via the `--ruvox-highlight-bg` token), with an 80 ms transition.

#### Scenario: Highlight moves to the next word

- GIVEN word N is highlighted
- WHEN playback advances into word N+1
- THEN the `word-highlight` class moves from word N's span to word N+1's
  span

### Requirement: Auto-scroll

When the newly highlighted span lies outside the visible viewport, the
system SHALL scroll it into view (`scrollIntoView`, nearest block, smooth
behavior). Spans already visible MUST NOT trigger scrolling.

#### Scenario: Active word off-screen

- GIVEN a long entry and playback reaching a word below the fold
- WHEN that word becomes active
- THEN the viewer scrolls so the highlighted word is visible

### Requirement: Highlight lifecycle

The system SHALL clear the highlight and reset the cached timestamps when
playback stops or finishes, when a different entry is selected, or when the
display mode is switched. Pausing SHALL keep the current highlight visible.
After an entry or mode switch the viewer SHALL re-subscribe to playback
events so highlighting resumes for the new context. Highlighting SHALL work
in `plain` and `markdown` modes; in `html` mode position events MUST be
ignored because no original-position mapping exists.

#### Scenario: Pause keeps the highlight

- GIVEN a word is highlighted during playback
- WHEN the user pauses
- THEN the highlight remains on the current word

#### Scenario: Stop clears the highlight

- GIVEN a word is highlighted
- WHEN playback stops or finishes
- THEN all `word-highlight` classes are removed from the viewer

#### Scenario: HTML mode without highlighting

- GIVEN the viewer is in HTML mode and the displayed entry is playing
- WHEN `playback_position` events arrive
- THEN no span is highlighted
