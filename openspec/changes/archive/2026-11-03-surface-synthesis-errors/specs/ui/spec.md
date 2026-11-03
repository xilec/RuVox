# Delta: ui

## ADDED Requirements

### Requirement: Synthesis failure on the queue card

A queue card SHALL render the entry's persisted `error_message`; it MUST NOT
depend on the transient `tts_error` toast. When the entry status is `error`,
the card SHALL show a red-tinted row with a 3 px left border and, below the
title, a localized prefix («Ошибка синтеза» in RU) followed by the stored
engine message verbatim — the message MUST NOT be parsed, classified or
translated. The message SHALL be clamped to two lines, SHALL expose the full
text through the `title` attribute, and SHALL be selectable and copyable. When
the clamped text overflows, a localized expand control («Показать полностью»)
SHALL appear and toggle the message between the clamped and the full state
(«Свернуть» when expanded).

Both failure variants SHALL offer a «Повторить» action that invokes
`commands.regenerateEntry(id, false)` directly — no regeneration preview
dialog, because the text has not changed and this is failure recovery. The
action SHALL be disabled while its command is pending and SHALL stay busy
until an `entry_updated` event for that entry reports that the retryable error
state has cleared; a rejected command SHALL clear the busy state and surface
the localized red error notification.

When the entry is playable (`ready`) and `error_message` is set — a failed
regeneration that kept the previous recording — the card SHALL show an amber
inline note «Последняя перегенерация не удалась: <message>» with the hint that
the previous recording still plays and the same «Повторить» action. The audio
SHALL stay playable, the red error treatment MUST NOT be applied, and the note
SHALL disappear when the next regeneration starts (its `entry_updated` clears
`error_message`).

The `tts_error` toast SHALL keep its existing behavior. Cards whose
`error_message` is `null` SHALL render exactly as before.

#### Scenario: failed entry shows its stored reason

- GIVEN an entry with status `error` and `error_message`
  "модель не загружена (ttsd)"
- WHEN the queue renders
- THEN the card shows "Ошибка синтеза: модель не загружена (ttsd)" with the red
  error treatment, without waiting for a toast

#### Scenario: long failure message expands and collapses

- GIVEN an error card whose stored message overflows the two-line clamp
- WHEN the user activates «Показать полностью»
- THEN the message is shown in full and the control reads «Свернуть»;
  activating it clamps the message again

#### Scenario: retry starts a regeneration without a preview

- GIVEN an error card
- WHEN the user activates «Повторить»
- THEN `commands.regenerateEntry(entryId, false)` is called once, no preview
  dialog opens, and the action is disabled until the entry's `entry_updated`
  clears the error state

#### Scenario: failed regeneration shows the amber note

- GIVEN a `ready` entry with stored audio and `error_message` "таймаут ttsd"
- WHEN the queue renders
- THEN the card shows the amber «Последняя перегенерация не удалась: таймаут
  ttsd» note with «Повторить», the Play action stays enabled, and the red
  error treatment is absent

#### Scenario: a new regeneration hides the note

- GIVEN a `ready` entry showing the amber failed-regeneration note
- WHEN the retry starts and `entry_updated` arrives with `error_message: null`
- THEN the note disappears and the card renders like a healthy entry

#### Scenario: healthy entries are unchanged

- GIVEN entries with no `error_message`
- WHEN the queue renders
- THEN no error block, retry action or amber note appears, and the card layout
  matches the pre-change layout
