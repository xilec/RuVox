# Tasks

## 1. Channel and inventory

- [x] 1.1 Add `model-channels/` entries for Silero Native and TeraTTSv2.
- [x] 1.2 Implement channel parsing, throttled fetch, and offline caching.
- [x] 1.3 Implement installed-asset inventory (versions, sizes, active state).

## 2. Install, update, delete

- [x] 2.1 Refactor the shared downloader to verify an expected manifest hash.
- [x] 2.2 Download into staging and swap atomically with rollback.
- [x] 2.3 Unify initial install and update behind one command.
- [x] 2.4 Add guarded deletion for bundles and Piper voices.

## 3. UI and snapshot

- [x] 3.1 Add the unified Models section to Settings (versions, sizes,
  update/delete/skip, total usage, offline note).
- [x] 3.2 Record bundle version/model id/upstream revision in the
  generation-params snapshot.

## 4. Validation and docs

- [x] 4.1 Unit tests for channel parsing, swap/rollback, delete guards, and
  inventory.
- [x] 4.2 Maintainer release procedure in `docs/`.
- [x] 4.3 `cargo fmt`, `cargo clippy`, `cargo test`, `just test`, `just lint`,
  `openspec validate --all --strict`.
- [ ] 4.4 Manual UI pass: install, update with a local channel, skip, delete,
  offline behavior.
