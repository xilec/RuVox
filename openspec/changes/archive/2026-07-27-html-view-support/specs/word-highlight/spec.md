# Delta: word-highlight

## MODIFIED Requirements

### Requirement: Highlight lifecycle

The system SHALL clear the highlight and reset the cached timestamps when
playback stops or finishes, when a different entry is selected, or when the
display mode is switched. Pausing SHALL keep the current highlight visible.
After an entry or mode switch the viewer SHALL re-subscribe to playback
events so highlighting resumes for the new context. Highlighting SHALL work
in `plain`, `markdown`, and `html` modes: in `html` mode the rendered word
spans carry offsets in the extracted text (`original_text`), which is the
same coordinate space as the timestamps' `original_pos`.

#### Scenario: Pause keeps the highlight

- GIVEN a word is highlighted during playback
- WHEN the user pauses
- THEN the highlight remains on the current word

#### Scenario: Stop clears the highlight

- GIVEN a word is highlighted
- WHEN playback stops or finishes
- THEN all `word-highlight` classes are removed from the viewer

#### Scenario: HTML mode highlights the spoken word

- GIVEN an HTML-ingested entry is playing and the viewer is in HTML mode
- WHEN a `playback_position` event arrives
- THEN the word whose span matches the active timestamp's `original_pos` is highlighted
