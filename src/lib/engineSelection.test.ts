import { describe, expect, it } from 'vitest';

import {
  applyEngineChange,
  buildSettingsPatch,
  computeEngineFormState,
  type AvailabilityMap,
} from './engineSelection';

const BOTH_AVAILABLE: AvailabilityMap = {
  piper: { available: true, reason: null },
  silero: { available: true, reason: null },
  silero_native: { available: false, reason: 'Бандл моделей не скачан' },
};

const ALL_AVAILABLE: AvailabilityMap = {
  piper: { available: true, reason: null },
  silero: { available: true, reason: null },
  silero_native: { available: true, reason: null },
};

const ONLY_PIPER: AvailabilityMap = {
  piper: { available: true, reason: null },
  silero: { available: false, reason: 'Python-стек не установлен' },
  silero_native: { available: false, reason: 'Бандл моделей не скачан' },
};

const ONLY_SILERO: AvailabilityMap = {
  piper: { available: false, reason: 'Голос не загружен' },
  silero: { available: true, reason: null },
  silero_native: { available: false, reason: 'Бандл моделей не скачан' },
};

const NEITHER: AvailabilityMap = {
  piper: { available: false, reason: 'oops' },
  silero: { available: false, reason: 'oops' },
  silero_native: { available: false, reason: 'oops' },
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
    expect(s.sileroSpeaker).toBe('aidar');
  });

  it('preserves a saved silero_native engine when it is available', () => {
    const s = computeEngineFormState(
      { engine: 'silero_native', piper_voice: 'ruslan', speaker: 'xenia' },
      ALL_AVAILABLE,
    );
    expect(s.engine).toBe('silero_native');
    expect(s.coercedAwayFromUnavailable).toBe(false);
  });

  it('coerces silero_native → piper when the bundle is not downloaded', () => {
    const s = computeEngineFormState(
      { engine: 'silero_native', piper_voice: 'ruslan', speaker: 'xenia' },
      BOTH_AVAILABLE,
    );
    expect(s.engine).toBe('piper');
    expect(s.coercedAwayFromUnavailable).toBe(true);
  });

  it('replaces a saved random speaker when the saved engine is silero_native', () => {
    const s = computeEngineFormState(
      { engine: 'silero_native', piper_voice: 'ruslan', speaker: 'random' },
      ALL_AVAILABLE,
    );
    expect(s.engine).toBe('silero_native');
    expect(s.sileroSpeaker).toBe('aidar');
  });

  it('keeps a saved random speaker for the ttsd engine', () => {
    const s = computeEngineFormState(
      { engine: 'silero', piper_voice: 'ruslan', speaker: 'random' },
      ALL_AVAILABLE,
    );
    expect(s.sileroSpeaker).toBe('random');
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

  it('coerces the random speaker away when switching to silero_native', () => {
    const onSilero = computeEngineFormState(
      { engine: 'silero', piper_voice: 'ruslan', speaker: 'random' },
      ALL_AVAILABLE,
    );
    const next = applyEngineChange(onSilero, 'silero_native', ALL_AVAILABLE);
    expect(next.engine).toBe('silero_native');
    expect(next.sileroSpeaker).toBe('aidar');
  });

  it('keeps the random speaker when switching to engines that support it', () => {
    const next = applyEngineChange(start, 'silero', BOTH_AVAILABLE);
    expect(next.sileroSpeaker).toBe('xenia');
    const withRandom = { ...start, sileroSpeaker: 'random' };
    expect(applyEngineChange(withRandom, 'silero', BOTH_AVAILABLE).sileroSpeaker).toBe('random');
    // Piper does not use the Silero speaker at all — keep it for the round-trip.
    expect(applyEngineChange(withRandom, 'piper', BOTH_AVAILABLE).sileroSpeaker).toBe('random');
  });
});

describe('buildSettingsPatch', () => {
  const values = {
    engine: 'piper' as const,
    piper_voice: 'ruslan',
    speaker: 'aidar',
    sample_rate: 24000,
    theme: 'dark' as const,
  };

  it('includes the engine on a normal save', () => {
    const patch = buildSettingsPatch(values, false);
    expect(patch.engine).toBe('piper');
    expect(patch.speaker).toBe('aidar');
  });

  it('omits the engine while the form is coerced away from an unavailable one', () => {
    // A coerced form shows the fallback engine; persisting it would overwrite
    // the saved engine (or the silero_native default on a bundle-less fresh
    // install), so the patch must leave `engine` out.
    const patch = buildSettingsPatch(values, true);
    expect('engine' in patch).toBe(false);
    expect(patch.theme).toBe('dark');
  });
});
