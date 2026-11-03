// Pure model of the Settings dialog: the section/row catalogue, the search
// index over both catalogs, and the dirty-field mapping. No React and no
// Tauri calls, so navigation and search rules unit-test in isolation from the
// shell (same split as engineSelection.ts).

import type { MessageKey } from '../i18n/ru';
import { translate } from './i18n';
import type { EngineKind, ReadingWidth } from './tauri';

/** Single source of truth for the form state shared by every section. */
export interface SettingsFormValues {
  engine: EngineKind;
  piper_voice: string;
  teratts_voice: string;
  speaker: string;
  sample_rate: number;
  notify_on_ready: boolean;
  notify_on_error: boolean;
  preview_dialog_enabled: boolean;
  hide_to_tray_on_close: boolean;
  max_cache_size_mb: number;
  code_block_mode: string;
  english_reading: string;
  reading_width: ReadingWidth;
  reading_scale: number;
  theme: string;
  language: string;
}

export type SettingsField = keyof SettingsFormValues;

export type SettingsSectionId =
  | 'engine'
  | 'reading'
  | 'notifications'
  | 'storage'
  | 'interface'
  | 'dictionary'
  | 'about';

export interface SettingsSectionSpec {
  readonly id: SettingsSectionId;
  readonly titleKey: MessageKey;
}

/** Nav order; also the order in which Enter picks the first search match. */
export const SETTINGS_SECTIONS: readonly SettingsSectionSpec[] = [
  { id: 'engine', titleKey: 'settings.section.engine' },
  { id: 'reading', titleKey: 'settings.section.reading' },
  { id: 'notifications', titleKey: 'settings.section.notifications' },
  { id: 'storage', titleKey: 'settings.section.storage' },
  { id: 'interface', titleKey: 'settings.section.interface' },
  { id: 'dictionary', titleKey: 'settings.section.dictionary' },
  { id: 'about', titleKey: 'settings.section.about' },
];

export type SettingsRowId =
  | 'models'
  | 'engine'
  | 'voice'
  | 'sample_rate'
  | 'code_block_mode'
  | 'english_reading'
  | 'preview_dialog_enabled'
  | 'reading_width'
  | 'reading_scale'
  | 'notify_on_ready'
  | 'notify_on_error'
  | 'max_cache_size_mb'
  | 'data_folder'
  | 'cleanup'
  | 'hide_to_tray_on_close'
  | 'theme'
  | 'language'
  | 'dictionary_entries'
  | 'dictionary_open'
  | 'version'
  | 'updates'
  | 'logs_folder';

export interface SettingsRowSpec {
  readonly id: SettingsRowId;
  readonly section: SettingsSectionId;
  /** Labels the row is titled with in either engine/state variant. */
  readonly labelKeys: readonly MessageKey[];
  /** Extra search terms (both languages) that never appear in the UI. */
  readonly synonyms?: readonly string[];
}

/**
 * Every setting row, in render order inside each section. The row ids double
 * as the `data-settings-field` hook the reachability test uses, so a control
 * rendered outside this catalogue is a test failure by construction.
 */
export const SETTINGS_ROWS: readonly SettingsRowSpec[] = [
  {
    id: 'models',
    section: 'engine',
    labelKeys: ['settings.models.title'],
    synonyms: ['models', 'модели', 'bundle', 'бандл', 'voices', 'голоса', 'download'],
  },
  {
    id: 'engine',
    section: 'engine',
    labelKeys: ['settings.engine.label'],
    synonyms: ['tts', 'speech', 'синтез', 'движок', 'engine'],
  },
  {
    id: 'voice',
    section: 'engine',
    labelKeys: [
      'settings.piper_voice.label',
      'settings.speaker.label',
      'settings.teratts_voice.label',
    ],
    synonyms: ['voice', 'speaker', 'голос', 'диктор', 'piper', 'silero', 'teratts'],
  },
  {
    id: 'sample_rate',
    section: 'engine',
    labelKeys: ['settings.sample_rate.label'],
    synonyms: ['hz', 'герц', 'частота', 'rate'],
  },
  {
    id: 'code_block_mode',
    section: 'reading',
    labelKeys: ['settings.code_block.label'],
    synonyms: ['code', 'код', 'code block', 'блок кода'],
  },
  {
    id: 'english_reading',
    section: 'reading',
    labelKeys: ['settings.english_reading.label'],
    synonyms: ['english', 'английский', 'latin', 'латиница', 'транслитерация'],
  },
  {
    id: 'preview_dialog_enabled',
    section: 'reading',
    labelKeys: ['settings.preview_dialog_enabled'],
    synonyms: ['preview', 'предпросмотр', 'dialog', 'диалог'],
  },
  {
    id: 'reading_width',
    section: 'reading',
    labelKeys: ['viewer.reading_width.label'],
    synonyms: ['width', 'ширина', 'column', 'колонка', 'строка'],
  },
  {
    id: 'reading_scale',
    section: 'reading',
    labelKeys: ['viewer.reading_scale.label'],
    synonyms: ['scale', 'масштаб', 'zoom', 'зум', 'size', 'размер'],
  },
  {
    id: 'notify_on_ready',
    section: 'notifications',
    labelKeys: ['settings.notify_on_ready'],
    synonyms: ['notification', 'уведомление', 'ready', 'готовность', 'тост'],
  },
  {
    id: 'notify_on_error',
    section: 'notifications',
    labelKeys: ['settings.notify_on_error'],
    synonyms: ['notification', 'уведомление', 'error', 'ошибка', 'сбой', 'тост'],
  },
  {
    id: 'max_cache_size_mb',
    section: 'storage',
    labelKeys: ['settings.max_cache_size.label'],
    synonyms: ['cache', 'кэш', 'limit', 'лимит', 'size', 'размер', 'mb', 'мб'],
  },
  {
    id: 'data_folder',
    section: 'storage',
    labelKeys: ['settings.data_folder'],
    synonyms: ['folder', 'папка', 'path', 'путь', 'data', 'данные', 'history', 'история'],
  },
  {
    id: 'cleanup',
    section: 'storage',
    labelKeys: ['settings.cleanup.button'],
    synonyms: ['cleanup', 'очистить', 'clear', 'cache', 'кэш', 'delete', 'удалить'],
  },
  {
    id: 'hide_to_tray_on_close',
    section: 'interface',
    labelKeys: ['settings.hide_to_tray_on_close'],
    synonyms: ['tray', 'трей', 'minimize', 'свернуть', 'close', 'закрытие'],
  },
  {
    id: 'theme',
    section: 'interface',
    labelKeys: ['settings.theme.label'],
    synonyms: ['theme', 'тема', 'appearance', 'оформление', 'dark', 'тёмная'],
  },
  {
    id: 'language',
    section: 'interface',
    labelKeys: ['settings.language.label'],
    synonyms: ['language', 'язык', 'locale', 'локаль'],
  },
  {
    id: 'dictionary_entries',
    section: 'dictionary',
    labelKeys: ['settings.dictionary.entries_count'],
    synonyms: ['dictionary', 'словарь', 'words', 'слова', 'entries', 'записи'],
  },
  {
    id: 'dictionary_open',
    section: 'dictionary',
    labelKeys: ['settings.dictionary.open'],
    synonyms: ['dictionary', 'словарь', 'editor', 'редактор', 'open', 'открыть'],
  },
  {
    id: 'version',
    section: 'about',
    labelKeys: ['settings.version'],
    synonyms: ['version', 'версия', 'build', 'сборка'],
  },
  {
    id: 'updates',
    section: 'about',
    labelKeys: ['settings.check_updates'],
    synonyms: ['update', 'updates', 'обновление', 'обновления', 'check', 'проверить'],
  },
  {
    id: 'logs_folder',
    section: 'about',
    labelKeys: ['settings.logs_folder'],
    synonyms: ['logs', 'логи', 'log', 'журнал', 'folder', 'папка', 'open', 'открыть'],
  },
];

/** Runtime state that gates the conditionally rendered rows. */
interface SettingsRowAvailability {
  readonly updaterEnabled: boolean;
  readonly cacheDir: string;
  readonly logDir: string;
}

/**
 * Rows the dialog actually renders for the current runtime state. The render
 * and the search index share this predicate: a row hidden by its runtime gate
 * (updates on Linux/.deb, a folder whose path has not resolved) must not match
 * a query either, otherwise `sectionHasMatches` keeps the section and Enter
 * targets a `data-settings-field` that is not in the DOM.
 */
export function availableSettingsRows(
  state: SettingsRowAvailability,
): ReadonlySet<SettingsRowId> {
  const available = new Set<SettingsRowId>();
  for (const row of SETTINGS_ROWS) {
    if (row.id === 'updates' && !state.updaterEnabled) continue;
    if (row.id === 'data_folder' && state.cacheDir === '') continue;
    if (row.id === 'logs_folder' && state.logDir === '') continue;
    available.add(row.id);
  }
  return available;
}

/**
 * The one section each dirty-tracked form field belongs to. Typed as an
 * exhaustive `Record<SettingsField, …>`, so a new form field fails the
 * typecheck until it is assigned a section — and thereby joins the dirty
 * count and the validation jump — instead of silently falling out of both.
 */
export const SECTION_OF: Record<SettingsField, SettingsSectionId> = {
  engine: 'engine',
  piper_voice: 'engine',
  teratts_voice: 'engine',
  speaker: 'engine',
  sample_rate: 'engine',
  code_block_mode: 'reading',
  english_reading: 'reading',
  preview_dialog_enabled: 'reading',
  reading_width: 'reading',
  reading_scale: 'reading',
  notify_on_ready: 'notifications',
  notify_on_error: 'notifications',
  max_cache_size_mb: 'storage',
  hide_to_tray_on_close: 'interface',
  theme: 'interface',
  language: 'interface',
};

/** All dirty-tracked fields, in section order (a field lives in one section). */
export const SETTINGS_FIELDS: readonly SettingsField[] = SETTINGS_SECTIONS.flatMap(
  (section) =>
    (Object.keys(SECTION_OF) as SettingsField[]).filter(
      (field) => SECTION_OF[field] === section.id,
    ),
);

/** Form fields owned by each section, used for the dirty dots and the count. */
const SECTION_FIELDS: Record<SettingsSectionId, readonly SettingsField[]> =
  SETTINGS_SECTIONS.reduce<Record<SettingsSectionId, readonly SettingsField[]>>(
    (fields, section) => {
      fields[section.id] = SETTINGS_FIELDS.filter(
        (field) => SECTION_OF[field] === section.id,
      );
      return fields;
    },
    {} as Record<SettingsSectionId, readonly SettingsField[]>,
  );

/**
 * Fields whose value differs from the saved config the dialog opened with.
 * The baseline is the loaded config, not the form defaults, so a value the
 * user picks and then reverts stops counting as changed.
 *
 * Deliberately a pure function over explicit `values`/`baseline` rather than a
 * `@mantine/form` `isDirty()` refactor: the dialog moves its baseline when the
 * saved engine is coerced and when a backend rollback rewrites a value, and a
 * plain reducer keeps that rule unit-testable without mounting the modal (the
 * same reason the section/search catalogue is a plain module).
 */
export function changedSettingsFields(
  values: SettingsFormValues,
  baseline: SettingsFormValues,
): SettingsField[] {
  return SETTINGS_FIELDS.filter((field) => values[field] !== baseline[field]);
}

/** Sections that own at least one changed field. */
export function changedSettingsSections(
  changed: readonly SettingsField[],
): Set<SettingsSectionId> {
  const sections = new Set<SettingsSectionId>();
  for (const section of SETTINGS_SECTIONS) {
    if (SECTION_FIELDS[section.id].some((field) => changed.includes(field))) {
      sections.add(section.id);
    }
  }
  return sections;
}

/** Section rendered for a validation error path (so the error is not hidden). */
export function sectionForField(field: string): SettingsSectionId | null {
  for (const section of SETTINGS_SECTIONS) {
    if (SECTION_FIELDS[section.id].includes(field as SettingsField)) {
      return section.id;
    }
  }
  return null;
}

/** Case- and ё-insensitive folding used by both the index and the highlighter. */
function fold(value: string): string {
  return value.toLowerCase().replace(/ё/g, 'е');
}

export function normalizeSearchQuery(query: string): string {
  return fold(query).trim();
}

function rowSearchTerms(row: SettingsRowSpec): string[] {
  const section = SETTINGS_SECTIONS.find((candidate) => candidate.id === row.section);
  const keys = [...row.labelKeys, ...(section ? [section.titleKey] : [])];
  // Both catalogs are indexed, so a Russian query finds a setting while the UI
  // runs in English and vice versa.
  const labels = keys.flatMap((key) => [translate('ru', key), translate('en', key)]);
  return [...labels, ...(row.synonyms ?? [])].map(fold);
}

/**
 * Whether a row matches the query. Multiple whitespace-separated tokens must
 * all match somewhere in the row's terms; an empty query matches everything.
 */
export function rowMatchesQuery(id: SettingsRowId, query: string): boolean {
  const tokens = normalizeSearchQuery(query).split(/\s+/).filter(Boolean);
  if (tokens.length === 0) return true;
  const row = SETTINGS_ROWS.find((candidate) => candidate.id === id);
  if (!row) return true;
  const terms = rowSearchTerms(row);
  return tokens.every((token) => terms.some((term) => term.includes(token)));
}

/**
 * Whether a section has at least one match among the rows the dialog renders.
 * `available` is the same set the section body gates its conditional rows on,
 * so a hidden row can neither keep its section in the nav nor be Enter-target.
 */
export function sectionHasMatches(
  section: SettingsSectionId,
  query: string,
  available: ReadonlySet<SettingsRowId>,
): boolean {
  return SETTINGS_ROWS.some(
    (row) => row.section === section && available.has(row.id) && rowMatchesQuery(row.id, query),
  );
}

/** First match in nav order — Enter jumps here. */
export function firstMatchingRow(
  query: string,
  available: ReadonlySet<SettingsRowId>,
): SettingsRowSpec | null {
  return (
    SETTINGS_ROWS.find((row) => available.has(row.id) && rowMatchesQuery(row.id, query)) ??
    null
  );
}

export interface HighlightPart {
  readonly text: string;
  readonly match: boolean;
}

/**
 * Split a label into at most three parts around the first case-insensitive
 * occurrence of the query, preserving the label's original casing. A query the
 * label does not contain literally (e.g. a synonym or the other catalog's
 * wording) simply yields one non-matching part — the row still matches.
 */
export function splitHighlight(text: string, query: string): HighlightPart[] {
  const needle = normalizeSearchQuery(query);
  if (!needle) return [{ text, match: false }];
  const index = fold(text).indexOf(needle);
  if (index < 0) return [{ text, match: false }];
  const before = text.slice(0, index);
  const match = text.slice(index, index + needle.length);
  const after = text.slice(index + needle.length);
  return [
    ...(before ? [{ text: before, match: false }] : []),
    { text: match, match: true },
    ...(after ? [{ text: after, match: false }] : []),
  ];
}
