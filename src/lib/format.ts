/**
 * Display formatters shared by the queue list and the generation-params
 * dialog. Pure functions — no React, no locale store (callers pass
 * translated units through i18n templates).
 */

/** `75` → `"1:15"` (minutes:seconds, seconds zero-padded). */
export function formatDuration(seconds: number): string {
  const m = Math.floor(seconds / 60);
  const s = Math.floor(seconds % 60);
  return `${m}:${s.toString().padStart(2, '0')}`;
}

/** `1536000` → `"1.5"` (megabytes with one decimal, for the `{0} МБ` template). */
export function formatMb(bytes: number): string {
  return (bytes / (1024 * 1024)).toFixed(1);
}
