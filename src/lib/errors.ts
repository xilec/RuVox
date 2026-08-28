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
  // The *_fetch_failed namespaces localize the status-less network failure
  // with a bare sentence; an HTTP-status param switches to the "(HTTP …)"
  // form (see catalogs).
  if ((code === 'import.fetch_failed' || code === 'image.fetch_failed') && params.length > 0) {
    return [`errors.${code}.http`, `errors.${code}`];
  }
  // Backend site ids live under the `errors.` prefix; frontend-only codes
  // (e.g. availability placeholders) use their bare catalog key.
  return [`errors.${code}`, code];
}

/** A localized result is only usable when every `{n}` placeholder found a
 *  param — otherwise the backend param arity drifted from the catalog text
 *  and the raw detail / generic fallback reads better than "{1}" on screen. */
function hasUnresolvedPlaceholder(s: string): boolean {
  return /\{\d+\}/.test(s);
}

/** CommandError wire shape: `{ type, code, params?, message? }` straight
 *  from a rejected invoke, or a frontend `CodedImportError` reusing the
 *  same fields (checked via duck typing so both localize identically). */
function isCodedShape(err: unknown): err is Record<string, unknown> & { type?: unknown } {
  return (
    typeof err === 'object' &&
    err !== null &&
    ('code' in err || 'type' in err || 'params' in err)
  );
}

function resolveCoded(err: object): string | null {
  const e = err as { type?: unknown; code?: unknown; params?: unknown; message?: unknown };
  const params = Array.isArray(e.params) ? e.params.map(String) : [];
  if (typeof e.code === 'string' && e.code) {
    for (const key of errorKeyCandidates(e.code, params)) {
      const localized = translate(currentLocale(), key, params);
      if (localized !== key && !hasUnresolvedPlaceholder(localized)) return localized;
    }
    // Unknown code (or param-arity drift): fall back to the raw backend
    // detail when present.
  }
  if (typeof e.message === 'string' && e.message) return e.message;
  const generic = GENERIC_BY_TYPE[e.type as keyof typeof GENERIC_BY_TYPE];
  return generic ? t(generic) : null;
}

export function formatError(err: unknown): string {
  if (isCodedShape(err)) {
    const resolved = resolveCoded(err);
    if (resolved) return resolved;
    // Coded shape but nothing usable — fall through to Error/string cases
    // below before resorting to JSON.stringify.
  }
  if (err instanceof Error) return err.message;
  if (typeof err === 'string') return err;
  if (typeof err === 'object' && err !== null) {
    try {
      return JSON.stringify(err);
    } catch {
      return Object.prototype.toString.call(err);
    }
  }
  return String(err);
}
