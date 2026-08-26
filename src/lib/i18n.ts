import { useCallback } from 'react';
import { ru, type MessageKey } from '../i18n/ru';
import { en } from '../i18n/en';
import { currentLocale, useLocaleStore, type Locale } from '../stores/locale';

const catalogs: Record<Locale, Record<string, string>> = { ru, en };

function interpolate(template: string, params?: readonly (string | number)[]): string {
  if (!params || params.length === 0) return template;
  return template.replace(/\{(\d+)\}/g, (match, index) => {
    const value = params[Number(index)];
    return value === undefined ? match : String(value);
  });
}

/**
 * Translate `key` in `locale`, interpolating positional `{0}`-params.
 * Fallback chain: locale catalog → RU catalog → the key itself.
 */
export function translate(
  locale: Locale,
  key: string,
  params?: readonly (string | number)[],
): string {
  const raw = catalogs[locale][key] ?? ru[key as MessageKey] ?? key;
  return interpolate(raw, params);
}

/** Reactive hook: re-renders the component when the locale changes. */
export function useT(): (
  key: MessageKey,
  params?: readonly (string | number)[],
) => string {
  const locale = useLocaleStore((s) => s.locale);
  return useCallback(
    (key, params) => translate(locale, key, params),
    [locale],
  );
}

/**
 * Non-React accessor: resolves against the locale current at call time.
 * For notification helpers and other plain modules.
 */
export function t(key: MessageKey, params?: readonly (string | number)[]): string {
  return translate(currentLocale(), key, params);
}
