import { describe, expect, it } from 'vitest';
import {
  canonicalReadingWidth,
  clampReadingScale,
  higherReadingScale,
  lowerReadingScale,
  readingZoom,
} from './reading';

describe('canonicalReadingWidth', () => {
  it('keeps the two valid widths', () => {
    expect(canonicalReadingWidth('narrow')).toBe('narrow');
    expect(canonicalReadingWidth('wide')).toBe('wide');
  });

  it('falls back to narrow for unknown, missing and legacy values', () => {
    expect(canonicalReadingWidth('huge')).toBe('narrow');
    expect(canonicalReadingWidth('')).toBe('narrow');
    expect(canonicalReadingWidth(null)).toBe('narrow');
    expect(canonicalReadingWidth(undefined)).toBe('narrow');
  });
});

describe('clampReadingScale', () => {
  it('clamps to the 80–200 range', () => {
    expect(clampReadingScale(50)).toBe(80);
    expect(clampReadingScale(80)).toBe(80);
    expect(clampReadingScale(150)).toBe(150);
    expect(clampReadingScale(200)).toBe(200);
    expect(clampReadingScale(500)).toBe(200);
  });

  it('keeps an off-ladder in-range value instead of snapping it', () => {
    expect(clampReadingScale(93)).toBe(93);
  });

  it('falls back to the default for non-finite input', () => {
    expect(clampReadingScale(Number.NaN)).toBe(100);
    expect(clampReadingScale(Number.POSITIVE_INFINITY)).toBe(100);
  });
});

describe('stepping along the ladder', () => {
  it('walks to the nearest lower step', () => {
    expect(lowerReadingScale(200)).toBe(175);
    expect(lowerReadingScale(150)).toBe(125);
    expect(lowerReadingScale(100)).toBe(80);
    expect(lowerReadingScale(80)).toBe(80);
  });

  it('walks to the nearest higher step', () => {
    expect(higherReadingScale(80)).toBe(100);
    expect(higherReadingScale(100)).toBe(125);
    expect(higherReadingScale(175)).toBe(200);
    expect(higherReadingScale(200)).toBe(200);
  });

  it('handles off-ladder and out-of-range values', () => {
    expect(lowerReadingScale(93)).toBe(80);
    expect(higherReadingScale(93)).toBe(100);
    expect(lowerReadingScale(50)).toBe(80);
    expect(higherReadingScale(500)).toBe(200);
  });
});

describe('readingZoom', () => {
  it('converts percent to a zoom multiplier', () => {
    expect(readingZoom(100)).toBe(1);
    expect(readingZoom(150)).toBe(1.5);
    expect(readingZoom(80)).toBe(0.8);
    expect(readingZoom(200)).toBe(2);
  });

  it('clamps before converting', () => {
    expect(readingZoom(50)).toBe(0.8);
    expect(readingZoom(500)).toBe(2);
  });
});
