/**
 * Silero speaker catalogue: the speaker names understood by both the ttsd
 * (Python) and the silero-native engines. Hand-mirrored from the backend
 * catalogue in the same way as `piperVoices.ts`; used by the Settings dialog
 * and the shared generation display labels so the two surfaces cannot drift.
 */


export interface SileroSpeaker {
  id: string;
  /** Display label — a proper name, not localized. */
  label: string;
}

export const SILERO_SPEAKERS: readonly SileroSpeaker[] = [
  { id: 'aidar', label: 'Aidar' },
  { id: 'baya', label: 'Baya' },
  { id: 'kseniya', label: 'Kseniya' },
  { id: 'xenia', label: 'Xenia' },
  { id: 'eugene', label: 'Eugene' },
];

/** Display label for a stored Silero speaker id; unknown ids fall back to the
 *  stored id instead of being guessed. */
export function sileroSpeakerLabel(voice: string): string {
  return SILERO_SPEAKERS.find((s) => s.id === voice)?.label ?? voice;
}
