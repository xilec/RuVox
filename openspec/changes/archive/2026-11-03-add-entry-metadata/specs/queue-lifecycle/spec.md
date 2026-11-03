## MODIFIED Requirements

### Requirement: Queue list rendering

The system SHALL display all entries in the left navbar, sorted by
`created_at` descending (newest first). For each entry the list SHALL show the
entry title — the stored `title`, or the title derived from `original_text`
when the stored value is `null` — as a single line with an ellipsis when it
does not fit, and the full title in the native tooltip; the 60-character
preview of `original_text` SHALL NOT be used as the card identity anymore. The
card SHALL also show a color-coded status badge with a Russian label
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

#### Scenario: Card shows the title, not the text slice

- GIVEN an entry with `original_text` longer than 60 characters and a stored `title`
- WHEN its card is rendered
- THEN the card shows the title on one line with an ellipsis when truncated,
  the full title is exposed as the tooltip, and the 60-character preview of
  `original_text` is not shown

#### Scenario: Legacy card shows the derived title

- GIVEN an entry with `title: null` whose first line is `## Установка`
- WHEN its card is rendered
- THEN the card shows `Установка`

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

### Requirement: Queue search

The system SHALL provide a search field above the queue that filters entries
by a case-insensitive substring match on any of the entry's `title`, `note` or
`original_text`. A match in any of the three fields SHALL keep the entry
visible; an entry that matches none SHALL be hidden. The `Ctrl+F` / `Cmd+F`
hotkey SHALL focus the search field, and `Escape` in the field SHALL clear the
query. When no entry matches, the system SHALL show "Ничего не найдено".

#### Scenario: Filtering entries

- GIVEN a non-empty queue and a search query
- WHEN the user types into the search field
- THEN only entries whose `original_text` contains the query
  (case-insensitive) are shown

#### Scenario: Filtering entries by title

- GIVEN an entry whose title is `Введение в Rust` and whose `original_text` does not contain the query
- WHEN the user types `rust`
- THEN the entry is shown

#### Scenario: Filtering entries by note

- GIVEN an entry whose note is `про бенчмарки Wasm` and whose text does not contain the query
- WHEN the user types `бенчмарки`
- THEN the entry is shown

#### Scenario: Case-insensitive match across fields

- GIVEN entries whose title or note differs from the query only in letter case
- WHEN the user types the lower-case query
- THEN those entries are shown
