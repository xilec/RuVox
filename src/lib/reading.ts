// Reading-typography helpers (#354 / #357). Pure functions shared by the
// viewer toolbar, the Settings «Чтение» block and the config plumbing, so the
// percent ladder, the clamped range and the CSS zoom factor cannot drift
// apart. Rust owns the persisted canonicalization; these helpers mirror it for
// the UI.

import type { ReadingWidth } from './tauri';

/** Percent steps offered by the reading-scale stepper. */
const READING_SCALE_STEPS = [80, 100, 125, 150, 175, 200] as const;

/** Inclusive bounds enforced by the backend on load and patch. */
export const READING_SCALE_MIN = 80;
export const READING_SCALE_MAX = 200;

/** Default / reset value of the reading scale. */
export const READING_SCALE_DEFAULT = 100;

/**
 * Canonicalize a stored reading width: only `"wide"` is meaningful, every
 * other value (legacy configs, hand-edited files) resolves to `"narrow"`.
 * Mirrors `UIConfig::canonical_reading_width` on the Rust side.
 */
export function canonicalReadingWidth(raw: string | null | undefined): ReadingWidth {
  return raw === 'wide' ? 'wide' : 'narrow';
}

/**
 * Clamp a scale percentage into the supported range. Non-finite input is a
 * programming error, not a user value — fall back to the default.
 */
export function clampReadingScale(percent: number): number {
  if (!Number.isFinite(percent)) return READING_SCALE_DEFAULT;
  return Math.min(
    READING_SCALE_MAX,
    Math.max(READING_SCALE_MIN, Math.round(percent)),
  );
}

/**
 * Nearest ladder step below `percent` (the minimum when already at or below
 * it). Stored values are not snapped to the ladder, so a hand-edited 93 still
 * steps down to 80 instead of getting stuck.
 */
export function lowerReadingScale(percent: number): number {
  const current = clampReadingScale(percent);
  for (let i = READING_SCALE_STEPS.length - 1; i >= 0; i -= 1) {
    if (READING_SCALE_STEPS[i] < current) return READING_SCALE_STEPS[i];
  }
  return READING_SCALE_MIN;
}

/** Nearest ladder step above `percent` (the maximum when already at it). */
export function higherReadingScale(percent: number): number {
  const current = clampReadingScale(percent);
  for (const step of READING_SCALE_STEPS) {
    if (step > current) return step;
  }
  return READING_SCALE_MAX;
}

/** CSS `zoom` multiplier for a scale percentage (100 % → 1). */
export function readingZoom(scale: number): number {
  return clampReadingScale(scale) / 100;
}
