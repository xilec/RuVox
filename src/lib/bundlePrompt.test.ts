import { describe, expect, it } from 'vitest';

import { shouldOfferBundleDownload } from './bundlePrompt';
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

describe('shouldOfferBundleDownload', () => {
  it('offers the download on a fresh install (silero_native, bundle missing)', () => {
    expect(shouldOfferBundleDownload({ engine: 'silero_native' }, BUNDLE_MISSING)).toBe(true);
  });

  it('stays silent once the bundle is on disk', () => {
    expect(shouldOfferBundleDownload({ engine: 'silero_native' }, BUNDLE_PRESENT)).toBe(false);
  });

  it('stays silent for an explicit Piper choice, even with the bundle missing', () => {
    expect(shouldOfferBundleDownload({ engine: 'piper' }, BUNDLE_MISSING)).toBe(false);
  });

  it('stays silent for an explicit Piper choice with the bundle present', () => {
    expect(shouldOfferBundleDownload({ engine: 'piper' }, BUNDLE_PRESENT)).toBe(false);
  });

  it('stays silent for the TeraTTS engine (Settings owns that download)', () => {
    expect(shouldOfferBundleDownload({ engine: 'teratts' }, BUNDLE_MISSING)).toBe(false);
  });

  it('stays silent for the ttsd Silero engine', () => {
    expect(shouldOfferBundleDownload({ engine: 'silero' }, BUNDLE_MISSING)).toBe(false);
  });
});
