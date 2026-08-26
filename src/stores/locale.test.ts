import { afterEach, describe, expect, it } from 'vitest';
import { currentLocale, setLocale, toLocale, useLocaleStore } from './locale';

describe('toLocale narrowing', () => {
  it('accepts the known locales', () => {
    expect(toLocale('ru')).toBe('ru');
    expect(toLocale('en')).toBe('en');
  });

  it('falls back to RU for unknown values', () => {
    expect(toLocale('fr')).toBe('ru');
    expect(toLocale('Russian')).toBe('ru');
    expect(toLocale('')).toBe('ru');
  });

  it('falls back to RU for absent values', () => {
    expect(toLocale(null)).toBe('ru');
    expect(toLocale(undefined)).toBe('ru');
  });
});

describe('locale store', () => {
  afterEach(() => {
    setLocale('ru');
  });

  it('defaults to ru and updates via setLocale', () => {
    expect(useLocaleStore.getState().locale).toBe('ru');
    setLocale('en');
    expect(useLocaleStore.getState().locale).toBe('en');
    expect(currentLocale()).toBe('en');
  });
});
