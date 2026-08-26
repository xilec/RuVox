import type { MessageKey } from '../i18n/ru';

// Hand-mirrored copy of `src-tauri/src/tts/piper/catalog.rs`.
// The Rust catalogue is the source of truth; this TS module exists only so
// the Settings UI can render labels and the "recommended" badge without an
// extra Tauri round-trip. Catalogue churn is rare (4 voices today), so a
// build-time codegen step is not worth its weight.
//
// When you change `catalog.rs::VOICES`, mirror the change here.

export interface PiperVoice {
  id: string;
  /** Catalog key of the display label (localized at render time). */
  key: MessageKey;
  recommended: boolean;
}

export const PIPER_VOICES: readonly PiperVoice[] = [
  { id: 'denis', key: 'settings.piper_voice.denis', recommended: false },
  { id: 'dmitri', key: 'settings.piper_voice.dmitri', recommended: false },
  { id: 'irina', key: 'settings.piper_voice.irina', recommended: false },
  { id: 'ruslan', key: 'settings.piper_voice.ruslan', recommended: true },
];

export const DEFAULT_PIPER_VOICE = 'ruslan';
