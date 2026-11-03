import { describe, expect, it } from 'vitest';

import { bundlePromptEngine } from './bundlePrompt';
import type { AvailableEngines } from './tauri';

const BUNDLE_PRESENT: AvailableEngines = {
  piper: { available: true, reason: null, reads_latin_natively: false },
  silero: { available: false, reason: { code: 'silero.uv_missing' }, reads_latin_natively: false },
  silero_native: { available: true, reason: null, reads_latin_natively: false },
  teratts: { available: true, reason: null, reads_latin_natively: true },
};

const BUNDLE_MISSING: AvailableEngines = {
  piper: { available: true, reason: null, reads_latin_natively: false },
  silero: { available: false, reason: { code: 'silero.uv_missing' }, reads_latin_natively: false },
  silero_native: { available: false, reason: { code: 'native.bundle_missing' }, reads_latin_natively: false },
  teratts: { available: false, reason: { code: 'teratts.bundle_missing' }, reads_latin_natively: true },
};

describe('bundlePromptEngine', () => {
  it('offers the TeraTTS bundle on a fresh config (new default, bundle missing)', () => {
    expect(bundlePromptEngine({ engine: 'teratts' }, BUNDLE_MISSING)).toBe('teratts');
  });

  it('stays silent once the TeraTTS bundle is on disk', () => {
    expect(bundlePromptEngine({ engine: 'teratts' }, BUNDLE_PRESENT)).toBeNull();
  });

  it('offers its own bundle to an existing Silero Native user', () => {
    expect(bundlePromptEngine({ engine: 'silero_native' }, BUNDLE_MISSING)).toBe('silero_native');
  });

  it('decides on the persisted engine, not on the new default', () => {
    // A Silero Native user whose bundle is installed is not prompted just
    // because the (unused) TeraTTS bundle is missing.
    expect(
      bundlePromptEngine(
        { engine: 'silero_native' },
        { ...BUNDLE_MISSING, silero_native: BUNDLE_PRESENT.silero_native },
      ),
    ).toBeNull();
  });

  it('stays silent once the Silero Native bundle is on disk', () => {
    expect(bundlePromptEngine({ engine: 'silero_native' }, BUNDLE_PRESENT)).toBeNull();
  });

  it('stays silent for an explicit Piper choice, even with both bundles missing', () => {
    expect(bundlePromptEngine({ engine: 'piper' }, BUNDLE_MISSING)).toBeNull();
    expect(bundlePromptEngine({ engine: 'piper' }, BUNDLE_PRESENT)).toBeNull();
  });

  it('stays silent for the ttsd Silero engine', () => {
    expect(bundlePromptEngine({ engine: 'silero' }, BUNDLE_MISSING)).toBeNull();
  });

  it('stays silent for an unknown persisted engine value', () => {
    expect(bundlePromptEngine({ engine: 'future_engine' }, BUNDLE_MISSING)).toBeNull();
  });
});
