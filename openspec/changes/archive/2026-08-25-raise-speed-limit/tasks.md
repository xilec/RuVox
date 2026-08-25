# Tasks: raise-speed-limit

## 1. Backend

- [x] 1.1 Widen `set_speed` validation to `[0.5, 3.0]` and update its doc
      comment (`src-tauri/src/commands/mod.rs`).

## 2. Frontend

- [x] 2.1 Update Player.tsx: clamps in `handleSpeedChange` and the startup
      restore effect, NumberInput `max`, tooltip label (0.5x–3.0x).
- [x] 2.2 Run `pnpm typecheck`, `pnpm test:unit`.

## 3. Verification

- [x] 3.1 `cargo test` green; manual pass: set 2.7x, relaunch, speed is
      restored; 3.5x rejected with a toast naming the range.
