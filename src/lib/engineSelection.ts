// Pure reducer for the Settings engine selector. No React, no Tauri imports —
// everything that needs to read or transform `(config, availability)` lives
// here so it can be unit-tested with Vitest in isolation from the Tauri shell.

import { DEFAULT_PIPER_VOICE } from './piperVoices';
import type { EngineKind, UIConfig } from './tauri';

export interface EngineAvailability {
  /** Whether the engine can be selected from the UI. Phase 3 of #42 wires
   *  this to a runtime probe of the ttsd / Python stack; in Phase 2 Silero
   *  is unconditionally `false` and Piper is unconditionally `true`. */
  available: boolean;
  /** Russian-language explanation shown in a tooltip / Alert when
   *  `available` is `false`. Null when the engine is available. */
  reason: string | null;
}

export interface AvailabilityMap {
  piper: EngineAvailability;
  silero: EngineAvailability;
}

export interface EngineFormState {
  engine: EngineKind;
  /** Voice the user picked for Piper. Persisted across engine flips so the
   *  Settings dialog re-shows it when they switch back. */
  piperVoice: string;
  /** Voice the user picked for Silero (`config.speaker`). Persisted across
   *  engine flips for the same reason. */
  sileroSpeaker: string;
  /** When `true`, show an inline alert telling the user we coerced the form
   *  away from their saved engine because it's currently unavailable. */
  coercedAwayFromUnavailable: boolean;
}

/**
 * Build the initial engine form state from a saved [`UIConfig`] and the
 * runtime availability map. If the saved engine is unavailable, falls back
 * to the recommended-available engine (currently Piper) and flags the
 * coercion so the UI can surface a one-shot alert.
 */
export function computeEngineFormState(
  config: Pick<UIConfig, 'engine' | 'piper_voice' | 'speaker'>,
  availability: AvailabilityMap,
): EngineFormState {
  const savedEngine: EngineKind = config.engine === 'silero' ? 'silero' : 'piper';
  const savedAvailable = availability[savedEngine].available;
  const engine: EngineKind = savedAvailable
    ? savedEngine
    : pickFallbackEngine(savedEngine, availability);

  return {
    engine,
    piperVoice: config.piper_voice || DEFAULT_PIPER_VOICE,
    sileroSpeaker: config.speaker || 'xenia',
    coercedAwayFromUnavailable: !savedAvailable && engine !== savedEngine,
  };
}

/**
 * Apply the user picking a different engine in the dropdown. Disabled
 * engines (`availability[next].available === false`) are silently rejected
 * — the dropdown must filter them out, but this is the defensive path.
 * Voice fields are preserved so the saved choice round-trips when the user
 * flips back.
 */
export function applyEngineChange(
  state: EngineFormState,
  next: EngineKind,
  availability: AvailabilityMap,
): EngineFormState {
  if (!availability[next].available) {
    return state;
  }
  return { ...state, engine: next, coercedAwayFromUnavailable: false };
}

function pickFallbackEngine(
  unavailable: EngineKind,
  availability: AvailabilityMap,
): EngineKind {
  const other: EngineKind = unavailable === 'piper' ? 'silero' : 'piper';
  if (availability[other].available) {
    return other;
  }
  // Both unavailable — return Piper so the UI still has a value to render.
  // The save attempt will fail at the backend and the user gets the error.
  return 'piper';
}
