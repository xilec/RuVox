import { describe, expect, it } from 'vitest';
import { translate } from './i18n';
import {
  changedSettingsFields,
  changedSettingsSections,
  firstMatchingRow,
  normalizeSearchQuery,
  rowMatchesQuery,
  sectionForField,
  sectionHasMatches,
  SETTINGS_FIELDS,
  SETTINGS_ROWS,
  SETTINGS_SECTIONS,
  splitHighlight,
  type SettingsFormValues,
} from './settingsSections';

const DEFAULTS: SettingsFormValues = {
  engine: 'silero_native',
  piper_voice: 'ruslan',
  teratts_voice: 'ru_m5',
  speaker: 'aidar',
  sample_rate: 24000,
  notify_on_ready: true,
  notify_on_error: true,
  preview_dialog_enabled: true,
  hide_to_tray_on_close: false,
  max_cache_size_mb: 500,
  code_block_mode: 'brief',
  english_reading: 'auto',
  reading_width: 'narrow',
  reading_scale: 100,
  theme: 'auto',
  language: 'ru',
};

describe('settings section catalogue', () => {
  it('gives every row a unique id and a known section', () => {
    const ids = SETTINGS_ROWS.map((row) => row.id);
    expect(new Set(ids).size).toBe(ids.length);
    const sections = new Set(SETTINGS_SECTIONS.map((section) => section.id));
    for (const row of SETTINGS_ROWS) {
      expect(sections.has(row.section), `row ${row.id} section`).toBe(true);
    }
  });

  it('covers every section with at least one row', () => {
    expect(new Set(SETTINGS_ROWS.map((row) => row.section))).toEqual(
      new Set(SETTINGS_SECTIONS.map((section) => section.id)),
    );
  });

  it('indexes labels that exist in both catalogs', () => {
    for (const row of SETTINGS_ROWS) {
      for (const key of row.labelKeys) {
        expect(translate('ru', key), `ru ${key}`).not.toBe(key);
        expect(translate('en', key), `en ${key}`).not.toBe(key);
      }
    }
  });

  it('maps every dirty-tracked form field to exactly one section', () => {
    expect(SETTINGS_FIELDS).toHaveLength(Object.keys(DEFAULTS).length);
    expect(new Set(SETTINGS_FIELDS).size).toBe(SETTINGS_FIELDS.length);
  });
});

describe('settings search', () => {
  it('normalizes case, surrounding spaces and ё', () => {
    expect(normalizeSearchQuery('  ЁЖИК ')).toBe('ежик');
  });

  it('matches a Russian label', () => {
    expect(rowMatchesQuery('theme', 'тема')).toBe(true);
  });

  it('matches an English label while the UI may run in Russian', () => {
    expect(rowMatchesQuery('theme', 'theme')).toBe(true);
  });

  it('matches a synonym that is not a visible label', () => {
    expect(rowMatchesQuery('logs_folder', 'журнал')).toBe(true);
  });

  it('requires every query token to match', () => {
    expect(rowMatchesQuery('english_reading', 'англ текст')).toBe(true);
    expect(rowMatchesQuery('english_reading', 'англ код')).toBe(false);
  });

  it('reports whether a section has a match', () => {
    expect(sectionHasMatches('notifications', 'уведомл')).toBe(true);
    expect(sectionHasMatches('notifications', 'код')).toBe(false);
  });

  it('picks the first match in nav order', () => {
    expect(firstMatchingRow('логи')?.id).toBe('logs_folder');
    expect(firstMatchingRow('уведомл')?.id).toBe('notify_on_ready');
    expect(firstMatchingRow('zzzz')).toBeNull();
  });

  it('matches everything for an empty or blank query', () => {
    expect(rowMatchesQuery('engine', '')).toBe(true);
    expect(sectionHasMatches('about', '   ')).toBe(true);
  });
});

describe('settings highlight', () => {
  it('splits a label around the case-insensitive match', () => {
    expect(splitHighlight('Английский текст', 'англ')).toEqual([
      { text: 'Англ', match: true },
      { text: 'ийский текст', match: false },
    ]);
  });

  it('preserves the label casing in the matched part', () => {
    expect(splitHighlight('Theme', 'the')).toEqual([
      { text: 'The', match: true },
      { text: 'me', match: false },
    ]);
  });

  it('returns one non-matching part when the label lacks the query', () => {
    expect(splitHighlight('Хранилище', 'кэш')).toEqual([
      { text: 'Хранилище', match: false },
    ]);
  });

  it('returns one non-matching part for an empty query', () => {
    expect(splitHighlight('Тема', '')).toEqual([{ text: 'Тема', match: false }]);
  });
});

describe('settings dirty fields', () => {
  it('reports only fields that differ from the saved baseline', () => {
    const values = { ...DEFAULTS, theme: 'dark', sample_rate: 48000 };
    expect(changedSettingsFields(values, DEFAULTS)).toEqual(['sample_rate', 'theme']);
  });

  it('reports the sections owning the changed fields', () => {
    const values = { ...DEFAULTS, theme: 'dark', sample_rate: 48000 };
    expect(changedSettingsSections(changedSettingsFields(values, DEFAULTS))).toEqual(
      new Set(['engine', 'interface']),
    );
  });

  it('reports no dirt for the coerced engine baseline', () => {
    // The dialog loads the coerced engine into both the form and the
    // baseline, so the fallback itself is not a change.
    const coerced = { ...DEFAULTS, engine: 'piper' as const };
    expect(changedSettingsFields(coerced, coerced)).toEqual([]);
  });

  it('resolves the section of a validation error path', () => {
    expect(sectionForField('max_cache_size_mb')).toBe('storage');
    expect(sectionForField('unknown_field')).toBeNull();
  });
});
