# Delta: queue-lifecycle

## ADDED Requirements

### Requirement: Persisted failure reason

The system SHALL persist the latest synthesis failure reason in the entry's
`error_message` field exactly as the engine or pipeline reported it — no
parsing, translation, truncation or classification — and SHALL keep the field
`null` for entries whose last synthesis succeeded. Because the field lives in
the entry history, it SHALL survive an application restart; no transient
event is required to recover it. A regeneration attempt SHALL clear
`error_message` when it starts, so the UI can distinguish an in-flight retry
from the failure it replaces. A failed regeneration SHALL store the failure
reason while keeping the previous recording playable (see the Entry status
lifecycle requirement).

#### Scenario: failure reason survives a restart

- GIVEN an entry whose synthesis failed with the engine message
  "модель не загружена"
- WHEN the application restarts and loads the queue
- THEN the entry still carries status `error` and `error_message`
  "модель не загружена"

#### Scenario: starting a regeneration clears the stored reason

- GIVEN an entry with status `error` or `ready` and `error_message` set
- WHEN `regenerate_entry` is invoked
- THEN the entry moves to `pending` with `error_message: null` and emits
  `entry_updated`

#### Scenario: a successful regeneration leaves no failure reason

- GIVEN an entry whose failed regeneration is retried
- WHEN the fresh synthesis reaches `ready`
- THEN `error_message` is `null`
