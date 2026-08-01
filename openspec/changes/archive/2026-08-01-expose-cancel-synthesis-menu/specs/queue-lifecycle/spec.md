# Delta: queue-lifecycle

## MODIFIED Requirements

### Requirement: Per-entry actions

Each queue item SHALL offer a Play action (enabled only for `ready` entries)
that invokes `play_entry`, and a right-click context menu with the items
"Воспроизвести", "Перегенерировать аудио" (disabled while `processing`),
"Отменить синтез" (enabled only while `processing`, invoking
`cancel_synthesis` without a confirmation dialog — cancellation is
non-destructive: the entry returns to `pending` and can be regenerated),
and
"Удалить". The currently playing entry SHALL be visually highlighted while
playback is active or paused; the highlight clears on stop or finish. When
the playing entry is scrolled out of view, the system SHALL show a
"К читаемому" button that selects the playing entry and scrolls it into view.

#### Scenario: Play from the queue

- GIVEN an entry with status `ready`
- WHEN the user clicks its Play button
- THEN `play_entry` is invoked and the entry is highlighted as playing

#### Scenario: Cancel a running synthesis from the menu

- GIVEN an entry with status `processing`
- WHEN the user opens its context menu and clicks "Отменить синтез"
- THEN `cancel_synthesis` is invoked for that entry

#### Scenario: Cancel item is unavailable outside processing

- GIVEN an entry with any status other than `processing`
- WHEN the user opens its context menu
- THEN the "Отменить синтез" item is disabled

#### Scenario: Jump to playing entry

- GIVEN an entry is playing and the user scrolled it out of the viewport
- WHEN the user clicks "К читаемому"
- THEN the playing entry becomes selected and is scrolled into the center of
  the list
