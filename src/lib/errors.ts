import { t, translate } from './i18n';
import { currentLocale } from '../stores/locale';
import type { MessageKey } from '../i18n/ru';

// Tauri command errors come back as typed JSON objects shaped like
// `{ type, code, params?, message? }` (see CommandError in commands/mod.rs
// and the ipc-commands spec). `formatError` is the single localization
// point for them: known `code` → catalog entry with interpolated params;
// unknown code → raw `message`; nothing usable → generic per-`type` string.
const GENERIC_BY_TYPE = {
  not_found: 'errors.generic.not_found',
  storage_error: 'errors.generic.storage_error',
  synthesis_error: 'errors.generic.synthesis_error',
  playback_error: 'errors.generic.playback_error',
  config_error: 'errors.generic.config_error',
  internal: 'errors.generic.internal',
} as const satisfies Record<string, MessageKey>;

/** Codes whose message form depends on whether params were provided. */
function errorKeyCandidates(code: string, params: string[]): string[] {
  if (code === 'image.fetch_failed' && params.length > 0) {
    return ['errors.image.fetch_failed.http', 'errors.image.fetch_failed'];
  }
  return [`errors.${code}`];
}

export function formatError(err: unknown): string {
  if (err instanceof Error) return err.message;
  if (typeof err === 'string') return err;
  if (typeof err === 'object' && err !== null) {
    const e = err as { type?: unknown; code?: unknown; params?: unknown; message?: unknown };
    const params = Array.isArray(e.params) ? e.params.map(String) : [];
    if (typeof e.code === 'string' && e.code) {
      for (const key of errorKeyCandidates(e.code, params)) {
        const localized = translate(currentLocale(), key, params);
        if (localized !== key) return localized;
      }
      // Unknown code: fall back to the raw backend detail when present.
      if (typeof e.message === 'string' && e.message) return e.message;
    }
    if (typeof e.message === 'string' && e.message) return e.message;
    const generic = GENERIC_BY_TYPE[e.type as keyof typeof GENERIC_BY_TYPE];
    if (generic) return t(generic);
    try {
      return JSON.stringify(err);
    } catch {
      return Object.prototype.toString.call(err);
    }
  }
  return String(err);
}
