import { describe, expect, it } from 'vitest';

import {
  findFormCollision,
  isDuplicateFrom,
  isSingleSourceToken,
  parseFormList,
  sortAndFilterEntries,
  spokenForm,
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

  it('accepts a Russian word', () => {
    expect(validateEntryInput({ from: 'замок', to: 'зам+ок' })).toBeNull();
  });

  it('rejects a multi-word from', () => {
    expect(validateEntryInput({ from: 'Иванов Пётр', to: 'иванов' })?.reason).toBe('from.charset');
  });

  it('rejects a mixed-script from', () => {
    expect(validateEntryInput({ from: 'IPзамок', to: 'ип замок' })?.reason).toBe('from.charset');
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

describe('inflected forms', () => {
  it('accepts Russian forms on a Russian entry', () => {
    expect(
      validateEntryInput({ from: 'замок', to: 'зам+ок', forms: ['замка', 'замке'] }),
    ).toBeNull();
  });

  it('rejects forms on a Latin entry', () => {
    expect(
      validateEntryInput({ from: 'docker', to: 'докер', forms: ['dockers'] })?.reason,
    ).toBe('forms.requires_cyrillic');
  });

  it('rejects a non-Russian or multi-word form', () => {
    for (const form of ['zamka', 'замка дом', 'кіт', '123']) {
      expect(validateEntryInput({ from: 'замок', to: 'зам+ок', forms: [form] })?.reason).toBe(
        'forms.charset',
      );
    }
  });

  it('rejects a form equal to the source or a sibling', () => {
    expect(validateEntryInput({ from: 'замок', to: 'зам+ок', forms: ['Замок'] })?.reason).toBe(
      'forms.duplicate',
    );
    expect(
      validateEntryInput({ from: 'замок', to: 'зам+ок', forms: ['замка', 'Замка'] })?.reason,
    ).toBe('forms.duplicate');
  });

  it('splits a comma-separated list and drops blank entries', () => {
    expect(parseFormList(' замка, , замке , ')).toEqual(['замка', 'замке']);
    expect(parseFormList('')).toEqual([]);
  });

  it('finds a form owned by another entry', () => {
    const entries = [
      { from: 'замок', to: 'зам+ок', forms: ['замка'] },
      { from: 'дворец', to: 'двор+ец' },
    ];
    expect(findFormCollision(entries, 'дворец', ['замка'])).toBe('замка');
    expect(findFormCollision(entries, 'дворец', ['замке'])).toBeNull();
    expect(findFormCollision(entries, 'замок', ['замка'], 'замок')).toBeNull();
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

  it('filters by an inflected form too', () => {
    const withForms = [...entries, { from: 'замок', to: 'зам+ок', forms: ['замка'] }];
    expect(sortAndFilterEntries(withForms, 'замка').map((e) => e.from)).toEqual(['замок']);
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

  it('accepts a Russian word', () => {
    expect(isSingleSourceToken('замок')).toBe(true);
    expect(isSingleSourceToken('  Замок ')).toBe(true);
  });

  it('rejects a mixed-script token', () => {
    expect(isSingleSourceToken('IPзамок')).toBe(false);
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

describe('stress marks', () => {
  it('accepts a mark before a vowel', () => {
    expect(validateEntryInput({ from: 'moskva', to: 'М+осква' })).toBeNull();
    expect(validateEntryInput({ from: 'zamok', to: 'зам+ок' })).toBeNull();
    expect(validateEntryInput({ from: 'yolka', to: '+Ёлка' })).toBeNull();
    expect(validateEntryInput({ from: 'moskva', to: 'Москв+а' })).toBeNull();
  });

  it('rejects a malformed mark', () => {
    for (const to of ['Моск+ва', 'М+оск+ва', '+Москва', 'Москва+']) {
      expect(validateEntryInput({ from: 'moskva', to })?.reason).toBe('to.stress_malformed');
    }
  });

  it('strips the mark from the spoken form', () => {
    expect(spokenForm('М+осква')).toBe('Москва');
    expect(spokenForm('зам+ок')).toBe('замок');
    expect(spokenForm('Москва')).toBe('Москва');
    expect(spokenForm('Моск+ва')).toBeNull();
  });

  it('counts the length without the mark', () => {
    const spoken = 'б'.repeat(257);
    expect(validateEntryInput({ from: 'x', to: spoken })?.reason).toBe('to.too_long');
    expect(validateEntryInput({ from: 'x', to: `+${'а'.repeat(255)}` })).toBeNull();
  });
});
