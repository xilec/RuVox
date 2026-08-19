import { describe, expect, it } from 'vitest';

import { bundleDownloadPercent } from './bundleDownload';
import type { BundleDownloadProgressPayload } from './tauri';

function payload(overrides: Partial<BundleDownloadProgressPayload>): BundleDownloadProgressPayload {
  return {
    engine: 'silero_native',
    file: 'model.onnx',
    file_idx: 0,
    total_files: 4,
    downloaded_bytes: 0,
    total_bytes: 100,
    ...overrides,
  };
}

describe('bundleDownloadPercent', () => {
  it('is 0 at the start of the first file', () => {
    expect(bundleDownloadPercent(payload({}))).toBe(0);
  });

  it('adds the current file fraction to the completed files', () => {
    expect(bundleDownloadPercent(payload({ file_idx: 1, downloaded_bytes: 50 }))).toBe(37.5);
  });

  it('treats an unknown total as no progress within the file', () => {
    expect(bundleDownloadPercent(payload({ file_idx: 2, total_bytes: 0 }))).toBe(50);
  });

  it('counts skipped files as complete', () => {
    expect(bundleDownloadPercent(payload({ file_idx: 1, skipped: true }))).toBe(50);
  });

  it('clamps at 100 and survives a zero total_files', () => {
    expect(bundleDownloadPercent(payload({ file_idx: 5, downloaded_bytes: 100 }))).toBe(100);
    expect(bundleDownloadPercent(payload({ total_files: 0, downloaded_bytes: 50 }))).toBe(50);
  });
});
