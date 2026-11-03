/**
 * Display labels for a stored generation snapshot (issue #353).
 *
 * Extracted from `GenerationParamsDialog` so the viewer header and the
 * recording-parameters dialog render the same engine and voice names. Values
 * absent from the snapshot stay `ABSENT` — never guessed.
 */

import type { MessageKey } from '../i18n/ru';
import { PIPER_VOICES } from './piperVoices';
import { RANDOM_SPEAKER } from './engineSelection';
import { sileroSpeakerLabel } from './sileroVoices';

type Translator = (
  key: MessageKey,
  params?: readonly (string | number)[],
) => string;

/** Placeholder for values the snapshot does not carry. */
export const ABSENT = '—';

/** `generation.engine` → localized engine name; unknown ids pass through. */
export function displayEngine(engine: string, tt: Translator): string {
  switch (engine) {
    case 'silero_native':
      return tt('generation.engine.silero_native');
    case 'piper':
      return tt('generation.engine.piper');
    case 'silero':
      return tt('generation.engine.silero');
    case 'teratts':
      return tt('generation.engine.teratts');
    default:
      return engine;
  }
}

/**
 * `generation.voice` → a display name that belongs to the engine that produced
 * the audio: Piper voice names, Silero speaker names, or the stored TeraTTS
 * id. Ids the catalogue does not know fall back to the stored id.
 */
export function displayVoice(
  engine: string,
  voice: string,
  tt: Translator,
): string {
  switch (engine) {
    case 'piper': {
      const known = PIPER_VOICES.find((v) => v.id === voice);
      return known ? tt(known.key) : voice;
    }
    case 'silero':
    case 'silero_native':
      return voice === RANDOM_SPEAKER
        ? tt('settings.speaker.random')
        : sileroSpeakerLabel(voice);
    case 'teratts':
    default:
      // TeraTTS ids (`ru_m5`, ...) are already human-readable; unknown engines
      // keep the stored id rather than inventing a name.
      return voice;
  }
}
