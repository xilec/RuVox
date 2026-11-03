import { describe, expect, it } from 'vitest';
import { displayEngine, displayVoice } from './generationDisplay';
import { translate } from './i18n';

const tt = (key: Parameters<typeof translate>[1], params?: readonly (string | number)[]) =>
  translate('ru', key, params);

describe('generation display labels', () => {
  it('localizes every known engine and passes unknown ids through', () => {
    expect(displayEngine('silero_native', tt)).toBe('Silero (нативный)');
    expect(displayEngine('piper', tt)).toBe('Piper');
    expect(displayEngine('silero', tt)).toBe('Silero (Python)');
    expect(displayEngine('teratts', tt)).toBe('TeraTTSv2');
    expect(displayEngine('mystery', tt)).toBe('mystery');
  });

  it('uses Piper voice names for the Piper engine', () => {
    expect(displayVoice('piper', 'ruslan', tt)).toBe('Руслан (мужской)');
    expect(displayVoice('piper', 'unknown', tt)).toBe('unknown');
  });

  it('uses Silero speaker names for both Silero engines', () => {
    expect(displayVoice('silero_native', 'xenia', tt)).toBe('Xenia');
    expect(displayVoice('silero', 'aidar', tt)).toBe('Aidar');
    expect(displayVoice('silero_native', 'random', tt)).toBe('Случайный');
    // Unknown ids fall back to the stored id instead of being guessed.
    expect(displayVoice('silero_native', 'ru_m5', tt)).toBe('ru_m5');
  });

  it('keeps TeraTTS ids verbatim', () => {
    expect(displayVoice('teratts', 'ru_m5', tt)).toBe('ru_m5');
  });
});
