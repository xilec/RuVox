import { describe, expect, it } from 'vitest';

import {
  applyEngineChange,
  computeEngineFormState,
  type AvailabilityMap,
} from './engineSelection';

const BOTH_AVAILABLE: AvailabilityMap = {
  piper: { available: true, reason: null },
  silero: { available: true, reason: null },
};

const ONLY_PIPER: AvailabilityMap = {
  piper: { available: true, reason: null },
  silero: { available: false, reason: 'Python-стек не установлен' },
};

const ONLY_SILERO: AvailabilityMap = {
  piper: { available: false, reason: 'Голос не загружен' },
  silero: { available: true, reason: null },
};

const NEITHER: AvailabilityMap = {
  piper: { available: false, reason: 'oops' },
  silero: { available: false, reason: 'oops' },
};

describe('computeEngineFormState', () => {
  it('preserves saved engine when both are available', () => {
    const s = computeEngineFormState(
      { engine: 'silero', piper_voice: 'irina', speaker: 'baya' },
      BOTH_AVAILABLE,
    );
    expect(s.engine).toBe('silero');
    expect(s.piperVoice).toBe('irina');
    expect(s.sileroSpeaker).toBe('baya');
    expect(s.coercedAwayFromUnavailable).toBe(false);
  });

  it('coerces silero → piper when silero is unavailable', () => {
    const s = computeEngineFormState(
      { engine: 'silero', piper_voice: 'ruslan', speaker: 'xenia' },
      ONLY_PIPER,
    );
    expect(s.engine).toBe('piper');
    expect(s.coercedAwayFromUnavailable).toBe(true);
  });

  it('does not flag coercion when saved engine matches forced fallback', () => {
    // piper is unavailable but the saved engine was already silero → no coercion.
    const s = computeEngineFormState(
      { engine: 'silero', piper_voice: 'ruslan', speaker: 'baya' },
      ONLY_SILERO,
    );
    expect(s.engine).toBe('silero');
    expect(s.coercedAwayFromUnavailable).toBe(false);
  });

  it('falls back to piper when both are unavailable', () => {
    const s = computeEngineFormState(
      { engine: 'silero', piper_voice: 'ruslan', speaker: 'xenia' },
      NEITHER,
    );
    expect(s.engine).toBe('piper');
    // Coerced because saved engine was silero, fallback chose piper.
    expect(s.coercedAwayFromUnavailable).toBe(true);
  });

  it('uses defaults when config has empty voice fields', () => {
    const s = computeEngineFormState(
      { engine: 'piper', piper_voice: '', speaker: '' },
      BOTH_AVAILABLE,
    );
    expect(s.piperVoice).toBe('ruslan');
    expect(s.sileroSpeaker).toBe('xenia');
  });
});

describe('applyEngineChange', () => {
  const start = computeEngineFormState(
    { engine: 'piper', piper_voice: 'ruslan', speaker: 'xenia' },
    BOTH_AVAILABLE,
  );

  it('flips to the selected engine when available', () => {
    const next = applyEngineChange(start, 'silero', BOTH_AVAILABLE);
    expect(next.engine).toBe('silero');
    // Voices preserved across the flip.
    expect(next.piperVoice).toBe('ruslan');
    expect(next.sileroSpeaker).toBe('xenia');
  });

  it('rejects a flip to an unavailable engine', () => {
    const next = applyEngineChange(start, 'silero', ONLY_PIPER);
    expect(next).toEqual(start);
  });

  it('clears the coercion alert after a manual change', () => {
    const coerced = computeEngineFormState(
      { engine: 'silero', piper_voice: 'ruslan', speaker: 'xenia' },
      ONLY_PIPER,
    );
    expect(coerced.coercedAwayFromUnavailable).toBe(true);
    const next = applyEngineChange(coerced, 'piper', ONLY_PIPER);
    expect(next.coercedAwayFromUnavailable).toBe(false);
  });
});
