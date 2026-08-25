# Design: raise-speed-limit

## Context

The `[0.5, 2.0]` speed range is enforced in three places that must stay in
sync: backend `set_speed` validation (authoritative, rejects with
`config_error`), the frontend optimistic clamp in `handleSpeedChange`, and
the NumberInput widget bounds. The #227 fix adds a fourth site: the startup
restore clamp applied to the persisted `speech_rate`.

## Goals / Non-Goals

**Goals:**

- Single new limit value (3.0) applied consistently at every enforcement
  site in one commit-sized change.

**Non-Goals:**

- Re-sampling or re-normalizing audio for high speeds (mpv `scaletempo2`
  handles pitch correction natively).
- Persisting the range itself in config — it stays a code constant.

## Decisions

- **Literal range constants, no shared constant extraction.** The range is
  already duplicated between Rust and TypeScript across an IPC boundary; a
  shared constant would need codegen for one number. The delta spec is the
  source of truth both sides are written against.
- **No clamping of out-of-range persisted values on restore beyond the
  existing clamp.** The restore clamp (`0.5..=3.0`) only guards against a
  hand-edited `config.json`; normal values pass through unchanged.

## Risks / Trade-offs

- Configs written by future/older builds with values outside `[0.5, 3.0]`
  are silently clamped at restore instead of surfacing an error — accepted,
  since playback must always start with a usable speed.
