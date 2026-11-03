# Delta: ui

## ADDED Requirements

### Requirement: Synthesis progress on the queue card

A queue card whose status is `processing` SHALL render, below the title, an
indeterminate progress line without a percentage, ETA or chunk stage, the
stage text «Синтез… M:SS» carrying the elapsed time, and an «Отменить»
button on the same row. A card whose status is `pending` SHALL render «В
очереди» and the same button without a timer. The button MUST be visible
without hovering and MUST NOT live inside the hover-only action group; it
SHALL be disabled while its `cancel_synthesis` call is in flight, and its
accessible label SHALL name the entry (for example «Отменить синтез:
<preview>»). A rejected command SHALL surface the existing localized red
notification, and the RU and EN catalogs SHALL carry the new strings.

A small per-row component SHALL own the local start timestamp and the
one-second tick, so that a tick re-renders only that line and neither the
queue item nor the queue list. The interval SHALL be cleared when the entry
reaches any terminal status (`ready`, `error`, `cancelled`) and when the
component unmounts. Rows in a terminal status SHALL render exactly as before,
including the persisted failure note and retry action from #351.

#### Scenario: Processing row presentation

- GIVEN an entry with status `processing`
- WHEN the queue card is rendered
- THEN it shows an indeterminate progress line, «Синтез… 0:00» and an
  «Отменить» button visible without hover
- AND it shows no percentage, ETA or chunk stage

#### Scenario: Pending row presentation

- GIVEN an entry with status `pending`
- WHEN the queue card is rendered
- THEN it shows «В очереди» and the «Отменить» button and no timer

#### Scenario: Timer stops on a terminal status

- GIVEN a processing card whose timer has advanced to «Синтез… 0:05»
- WHEN the entry becomes `ready`, `error` or `cancelled`
- THEN the progress line disappears and no further timer tick renders

#### Scenario: A tick re-renders only the progress line

- GIVEN a queue with several cards, one of them `processing`
- WHEN one second passes and the elapsed text advances
- THEN only the ticking progress line re-renders; the queue items and the
  list do not

#### Scenario: Cancel is invoked once and disables the button

- GIVEN a processing card with the inline «Отменить» action
- WHEN the user clicks it while the command is in flight
- THEN `cancel_synthesis` is called once and the button is disabled until
  the command settles

#### Scenario: Cancel failure shows the existing toast

- GIVEN a processing card with the inline «Отменить» action
- WHEN `cancel_synthesis` rejects
- THEN the existing localized red notification is shown and the button is
  enabled again

#### Scenario: Accessible label names the entry

- GIVEN a processing card whose text starts with «Тёмная сторона инлайнинга»
- WHEN the inline cancel button is inspected
- THEN its accessible label is «Отменить синтез: Тёмная сторона инлайнинга…»

#### Scenario: English labels after a locale switch

- GIVEN the queue renders a `pending` and a `processing` card in RU
- WHEN the UI language switches to EN
- THEN the cards read "Queued" and "Synthesizing… 0:05" and the button reads
  "Cancel"
