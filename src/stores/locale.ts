import { create } from 'zustand';

export type Locale = 'ru' | 'en';

interface LocaleState {
  locale: Locale;
  setLocale: (locale: Locale) => void;
}

/**
 * Active UI language. Seeded once at App start from `getConfig().language`
 * (config.json is the single source of truth — no localStorage); the
 * Settings selector updates the store immediately so the whole UI relabels
 * without reload, and persists via `updateConfig`.
 */
export const useLocaleStore = create<LocaleState>((set) => ({
  locale: 'ru',
  setLocale: (locale) => set({ locale }),
}));

/** Current locale for non-React modules (read at call time). */
export function currentLocale(): Locale {
  return useLocaleStore.getState().locale;
}

/** Switch the UI language immediately (Settings selector, startup seeding). */
export function setLocale(locale: Locale): void {
  useLocaleStore.setState({ locale });
}

const LOCALES: readonly Locale[] = ['ru', 'en'];

/** Narrow a config value to a known locale, falling back to RU. */
export function toLocale(v: string | null | undefined): Locale {
  return LOCALES.includes(v as Locale) ? (v as Locale) : 'ru';
}
