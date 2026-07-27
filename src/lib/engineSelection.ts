// Pure reducer for the Settings engine selector. No React, no Tauri imports —
// everything that needs to read or transform `(config, availability)` lives
// here so it can be unit-tested with Vitest in isolation from the Tauri shell.

import { DEFAULT_PIPER_VOICE } from './piperVoices';
import type { AvailableEngines, EngineKind, UIConfig } from './tauri';

export type AvailabilityMap = AvailableEngines;

/** Speaker id that only the Python ttsd engine supports: it picks a random
 *  speaker per call. The native engine has no such concept and rejects it. */
export const RANDOM_SPEAKER = 'random';

/** Default Silero speaker, used when the saved/picked speaker cannot be
 *  served by the active engine. */
export const DEFAULT_SILERO_SPEAKER = 'xenia';

/**
 * Map a speaker to one the given engine can serve. Currently only `random`
 * needs coercion: valid for ttsd (`silero`), rejected by `silero_native`.
 */
export function coerceSpeakerForEngine(engine: EngineKind, speaker: string): string {
  return engine === 'silero_native' && speaker === RANDOM_SPEAKER
    ? DEFAULT_SILERO_SPEAKER
    : speaker;
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
  const savedEngine = coerceEngineKind(config.engine);
  const savedAvailable = availability[savedEngine].available;
  const engine: EngineKind = savedAvailable
    ? savedEngine
    : pickFallbackEngine(availability);

  return {
    engine,
    piperVoice: config.piper_voice || DEFAULT_PIPER_VOICE,
    sileroSpeaker: coerceSpeakerForEngine(engine, config.speaker || DEFAULT_SILERO_SPEAKER),
    coercedAwayFromUnavailable: !savedAvailable && engine !== savedEngine,
  };
}

/** Map a persisted (possibly stale / unknown) engine string to a known kind. */
function coerceEngineKind(raw: string): EngineKind {
  return raw === 'silero' || raw === 'silero_native' ? raw : 'piper';
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
  return {
    ...state,
    engine: next,
    sileroSpeaker: coerceSpeakerForEngine(next, state.sileroSpeaker),
    coercedAwayFromUnavailable: false,
  };
}

function pickFallbackEngine(availability: AvailabilityMap): EngineKind {
  // Preference order for the automatic fallback: Piper first (in-process,
  // always available in practice), then the Silero engines.
  const order: EngineKind[] = ['piper', 'silero', 'silero_native'];
  const found = order.find((e) => availability[e].available);
  // Nothing available — return Piper so the UI still has a value to render.
  // The save attempt will fail at the backend and the user gets the error.
  return found ?? 'piper';
}
