/**
 * Date-time display helpers for stored entry timestamps.
 *
 * The storage layer writes naive UTC timestamps without a timezone suffix
 * (`chrono` may emit 3, 6 or 9 fractional digits). Both the viewer header and
 * the recording-parameters dialog must render them in local time and in the
 * app language, not in the webview locale.
 */

type Locale = 'ru' | 'en';

const LOCALE_TAG: Record<Locale, string> = {
  ru: 'ru-RU',
  en: 'en-US',
};

/**
 * Parse a stored naive UTC timestamp. Without the suffix JS would treat the
 * value as local time; some JS engines only guarantee parsing exactly three
 * fractional digits, so longer fractions are trimmed to milliseconds.
 */
function parseNaiveUtc(naiveUtc: string): Date {
  return new Date(`${naiveUtc.replace(/(\.\d{3})\d+$/, '$1')}Z`);
}

/** Local date and time, e.g. «2 нояб. 2026 г., 14:03» / "Nov 2, 2026, 02:03 PM". */
export function formatNaiveUtcDateTime(naiveUtc: string, locale: Locale): string {
  return parseNaiveUtc(naiveUtc).toLocaleString(LOCALE_TAG[locale], {
    day: 'numeric',
    month: 'short',
    year: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });
}
