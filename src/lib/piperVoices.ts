// Hand-mirrored copy of `src-tauri/src/tts/piper/catalog.rs`.
// The Rust catalogue is the source of truth; this TS module exists only so
// the Settings UI can render labels and the "Рекомендуется" badge without an
// extra Tauri round-trip. Catalogue churn is rare (4 voices today), so a
// build-time codegen step is not worth its weight.
//
// When you change `catalog.rs::VOICES`, mirror the change here.

export interface PiperVoice {
  id: string;
  label: string;
  recommended: boolean;
}

export const PIPER_VOICES: readonly PiperVoice[] = [
  { id: 'denis', label: 'Денис (мужской)', recommended: false },
  { id: 'dmitri', label: 'Дмитрий (мужской)', recommended: false },
  { id: 'irina', label: 'Ирина (женский)', recommended: false },
  { id: 'ruslan', label: 'Руслан (мужской)', recommended: true },
];

export const DEFAULT_PIPER_VOICE = 'ruslan';

export function lookupPiperVoice(id: string): PiperVoice | undefined {
  return PIPER_VOICES.find((v) => v.id === id);
}
