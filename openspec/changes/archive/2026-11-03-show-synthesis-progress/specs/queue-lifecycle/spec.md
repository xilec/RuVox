# Delta: queue-lifecycle

## MODIFIED Requirements

### Requirement: Queue list rendering

The system SHALL display all entries in the left navbar, sorted by
`created_at` descending (newest first). For each entry the list SHALL show:
a preview of the first 60 characters of `original_text` (with an ellipsis
when truncated), a color-coded status badge with a Russian label
(`Ожидание` / `Обработка` / `Готово` / `Играет` / `Ошибка` / `Отменено`),
and the duration formatted as `M:SS` once `duration_sec` is available. The
`processing` badge SHALL include a spinner. The list SHALL update live from
`entry_updated` and `entry_removed` events without a full reload. When the
queue is empty the system SHALL show the hint "Скопируйте текст и нажмите
Add".

A `processing` entry SHALL additionally show an indeterminate progress line
and the elapsed-time text «Синтез… M:SS» (for example «Синтез… 0:42»), and a
`pending` entry SHALL show «В очереди»; neither shows a percentage, ETA or
chunk stage. The elapsed time SHALL be counted from the moment the frontend
observed the entry become `processing`, and the progress line SHALL disappear
when the entry reaches any other status. Because the persisted entry carries
no start timestamp, the elapsed time SHALL start over after an application
restart.

#### Scenario: Entries sorted newest first

- GIVEN entries with different `created_at` timestamps
- WHEN the queue is rendered
- THEN entries appear sorted by `created_at` descending

#### Scenario: Live status update

- GIVEN an entry displayed with status `Обработка`
- WHEN an `entry_updated` event arrives with status `ready`
- THEN the entry's badge switches to `Готово` and the duration appears
  without reloading the list

#### Scenario: Processing row shows elapsed time

- GIVEN an entry with status `processing`
- WHEN its card has been rendered for 42 seconds
- THEN the card shows «Синтез… 0:42» and the indeterminate progress line

#### Scenario: Pending row shows no timer

- GIVEN an entry with status `pending`
- WHEN its card is rendered
- THEN the card shows «В очереди» and no elapsed-time text

#### Scenario: Terminal status removes the progress line

- GIVEN a card showing the elapsed time for a `processing` entry
- WHEN `entry_updated` reports `ready`, `error` or `cancelled` for that entry
- THEN the progress line and the elapsed-time text disappear and the
  one-second interval stops

#### Scenario: Restart restarts the elapsed time

- GIVEN an entry that was `processing` when the application restarted
- WHEN the queue loads and renders its card
- THEN the elapsed time counts from 0 again, because the persisted entry
  carries no start timestamp

## ADDED Requirements

### Requirement: Inline synthesis cancel

A `pending` or `processing` queue card SHALL show an inline «Отменить»
action that is visible without hovering and invokes `cancel_synthesis` for
that entry exactly once per activation. The action SHALL be disabled while
its command is in flight. A rejected command SHALL surface the existing
localized red error notification. This action is in addition to the
"Отменить синтез" context-menu item, whose behavior is unchanged.

#### Scenario: Cancel a running synthesis from the card

- GIVEN an entry with status `processing`
- WHEN the user clicks its inline «Отменить» action
- THEN `cancel_synthesis` is invoked exactly once for that entry

#### Scenario: Cancel a queued synthesis from the card

- GIVEN an entry with status `pending`
- WHEN the user clicks its inline «Отменить» action
- THEN `cancel_synthesis` is invoked exactly once for that entry

#### Scenario: Cancel action is disabled while the command is in flight

- GIVEN a card showing the inline «Отменить» action
- WHEN its `cancel_synthesis` command has not settled yet
- THEN the action is disabled and a second click does not invoke the command
  again

#### Scenario: Cancel failure surfaces the localized toast

- GIVEN a card with a visible inline «Отменить» action
- WHEN `cancel_synthesis` rejects
- THEN the existing localized red error notification is shown and the action
  becomes enabled again
