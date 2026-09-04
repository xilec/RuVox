import { describe, expect, it } from 'vitest';

import {
  isDuplicateFrom,
  isSingleSourceToken,
  sortAndFilterEntries,
  validateEntryInput,
} from './userDictionary';

describe('validateEntryInput', () => {
  it('accepts a plain word pair', () => {
    expect(validateEntryInput({ from: 'GitHub', to: 'гитхаб' })).toBeNull();
  });

  it('accepts an alnum token with a letter', () => {
    expect(validateEntryInput({ from: 'IPv6', to: 'айпи ви шесть' })).toBeNull();
  });

  it('rejects an empty from', () => {
    expect(validateEntryInput({ from: '', to: 'что-то' })?.reason).toBe('from.required');
  });

  it('rejects a Cyrillic from', () => {
    expect(validateEntryInput({ from: 'Иванов', to: 'иванов' })?.reason).toBe('from.charset');
  });

  it('rejects a digit-only from', () => {
    expect(validateEntryInput({ from: '123', to: 'число' })?.reason).toBe('from.charset');
  });

  it('rejects punctuation in from (hyphens are a follow-up issue)', () => {
    expect(validateEntryInput({ from: 'UTF-8', to: 'у тэф восемь' })?.reason).toBe('from.charset');
  });

  it('rejects an overlong from', () => {
    const long = 'a'.repeat(65);
    expect(validateEntryInput({ from: long, to: 'длинно' })?.reason).toBe('from.too_long');
  });

  it('rejects an empty to', () => {
    expect(validateEntryInput({ from: 'docker', to: '' })?.reason).toBe('to.required');
  });

  it('rejects an overlong to (by characters, not bytes)', () => {
    const long = 'а'.repeat(257);
    expect(validateEntryInput({ from: 'docker', to: long })?.reason).toBe('to.too_long');
  });

  it('flags Latin in to as a non-blocking warning', () => {
    const result = validateEntryInput({ from: 'docker', to: 'Docker' });
    expect(result).toEqual({ reason: 'to.latin_warning', blocking: false });
  });
});

describe('isDuplicateFrom', () => {
  const entries = [
    { from: 'GitHub', to: 'гитхаб' },
    { from: 'nginx', to: 'энджинкс' },
  ];

  it('detects a case-insensitive duplicate', () => {
    expect(isDuplicateFrom(entries, 'github')).toBe(true);
    expect(isDuplicateFrom(entries, 'GITHUB')).toBe(true);
  });

  it('passes a new word', () => {
    expect(isDuplicateFrom(entries, 'kubectl')).toBe(false);
  });
});

describe('sortAndFilterEntries', () => {
  const entries = [
    { from: 'nginx', to: 'энджинкс' },
    { from: 'Docker', to: 'докер' },
    { from: 'kubectl', to: 'куб контрол' },
  ];

  it('sorts by the lowercased from', () => {
    expect(sortAndFilterEntries(entries, '').map((e) => e.from)).toEqual([
      'Docker',
      'kubectl',
      'nginx',
    ]);
  });

  it('filters by a case-insensitive from substring', () => {
    expect(sortAndFilterEntries(entries, 'ku').map((e) => e.from)).toEqual(['kubectl']);
  });

  it('filters by a to substring too', () => {
    expect(sortAndFilterEntries(entries, 'докер').map((e) => e.from)).toEqual(['Docker']);
  });

  it('keeps everything for a blank query', () => {
    expect(sortAndFilterEntries(entries, '  ')).toHaveLength(3);
  });
});

describe('isSingleSourceToken', () => {
  it('accepts a single Latin word with surrounding spaces', () => {
    expect(isSingleSourceToken('  Ivanov ')).toBe(true);
  });

  it('accepts an alnum token', () => {
    expect(isSingleSourceToken('IPv6')).toBe(true);
  });

  it('rejects Cyrillic', () => {
    expect(isSingleSourceToken('иванов')).toBe(false);
  });

  it('rejects multi-word selections', () => {
    expect(isSingleSourceToken('docker compose')).toBe(false);
  });

  it('rejects punctuation and empty selections', () => {
    expect(isSingleSourceToken('C++')).toBe(false);
    expect(isSingleSourceToken('')).toBe(false);
    expect(isSingleSourceToken('   ')).toBe(false);
  });
});
