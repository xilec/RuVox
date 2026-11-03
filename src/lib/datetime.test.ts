import { describe, expect, it } from 'vitest';
import { formatNaiveUtcDateTime } from './datetime';

/** The options the helper passes to `toLocaleString`. */
const OPTIONS: Intl.DateTimeFormatOptions = {
  day: 'numeric',
  month: 'short',
  year: 'numeric',
  hour: '2-digit',
  minute: '2-digit',
};

describe('formatNaiveUtcDateTime', () => {
  it('renders a naive UTC timestamp in local time and the app language', () => {
    const expected = new Date(Date.UTC(2026, 10, 2, 7, 3)).toLocaleString(
      'ru-RU',
      OPTIONS,
    );
    expect(formatNaiveUtcDateTime('2026-11-02T07:03:00', 'ru')).toBe(expected);
  });

  it('renders the English locale through en-US', () => {
    const expected = new Date(Date.UTC(2026, 10, 2, 7, 3)).toLocaleString(
      'en-US',
      OPTIONS,
    );
    expect(formatNaiveUtcDateTime('2026-11-02T07:03:00', 'en')).toBe(expected);
  });

  it('trims fractions beyond milliseconds', () => {
    // `chrono` may emit 3, 6 or 9 fractional digits; only `Date`'s three are
    // meaningful and longer fractions are not portable across JS engines.
    const expected = new Date(Date.UTC(2026, 10, 2, 7, 3, 0, 123)).toLocaleString(
      'ru-RU',
      OPTIONS,
    );
    expect(formatNaiveUtcDateTime('2026-11-02T07:03:00.123456', 'ru')).toBe(
      expected,
    );
    expect(formatNaiveUtcDateTime('2026-11-02T07:03:00.123', 'ru')).toBe(expected);
  });
});
