# Delta: storage

## ADDED Requirements

### Requirement: Atomic Conditional Update

The storage service SHALL provide a compare-and-set update operation over a
single entry: given an entry id, a predicate, and a mutation, it SHALL acquire
the entry map's write lock, evaluate the predicate against the current entry,
and — only when the predicate returns `true` — apply the mutation and persist,
all under that single lock hold. The predicate check and the mutation SHALL NOT
be separated by a release of the write lock.

The operation SHALL return `true` when the entry existed and the predicate
matched (the mutation was applied). It SHALL return `false` and change nothing
when the entry is absent or the predicate rejected it.

This operation SHALL be the mechanism used by status transitions that decide on
the basis of the entry's current status (entry cancellation, and the
stale-completion guards for synthesis ready/error), so a concurrent
read-decide-write cannot persist a stale entry clone over a transition that
already applied.

#### Scenario: predicate accepts applies the mutation
- GIVEN an entry whose status the predicate accepts
- WHEN the conditional update is invoked with a mutation
- THEN the mutation is applied, the history is persisted, and the operation returns `true`

#### Scenario: predicate rejects changes nothing
- GIVEN an entry whose status the predicate rejects
- WHEN the conditional update is invoked with a mutation
- THEN the entry is unchanged, the history is not modified for this update, and the operation returns `false`

#### Scenario: absent id is a no-op
- GIVEN no entry with the given id
- WHEN the conditional update is invoked
- THEN nothing is written and the operation returns `false`

#### Scenario: concurrent status transition cannot regress a completed entry
- GIVEN an entry mid-transition (e.g. `processing`) and two callers racing: one applies a completion that flips it to `ready`, another cancels it back to `pending`
- WHEN both run the conditional update under the predicate `status in {processing, pending}`
- THEN only one transition is applied, the entry ends in exactly one status, and no stale clone overwrites the applied transition
