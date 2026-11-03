import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import {
  Alert,
  Button,
  Checkbox,
  Group,
  Modal,
  NumberInput,
  Select,
  Stack,
  Text,
  TextInput,
  UnstyledButton,
  useMantineColorScheme,
} from '@mantine/core';
import type { MantineColorScheme } from '@mantine/core';
import { useForm, type FormErrors } from '@mantine/form';
import { useMediaQuery } from '@mantine/hooks';
import { notifications } from '@mantine/notifications';
import { revealItemInDir } from '@tauri-apps/plugin-opener';
import { getVersion } from '@tauri-apps/api/app';
import { commands } from '../lib/tauri';
import { formatMb as formatMbValue } from '../lib/format';
import type { CleanupMode, EngineKind, UIConfigPatch } from '../lib/tauri';
import type { MessageKey } from '../i18n/ru';
import { formatError } from '../lib/errors';
import { canonicalReadingWidth, READING_SCALE_DEFAULT } from '../lib/reading';
import { t, useT } from '../lib/i18n';
import { setLocale, toLocale } from '../stores/locale';
import { PIPER_VOICES } from '../lib/piperVoices';
import { SILERO_SPEAKERS } from '../lib/sileroVoices';
import { IconSearch } from '../components/icons';
import { ModelAssetsDialog } from './ModelAssets';
import { SettingsSections } from './SettingsSections';
import { checkForUpdatesManual, updaterSupported } from '../lib/updater';
import {
  applyEngineChange,
  buildSettingsPatch,
  computeEngineFormState,
  RANDOM_SPEAKER,
  type AvailabilityMap,
} from '../lib/engineSelection';
import {
  availableSettingsRows,
  changedSettingsFields,
  changedSettingsSections,
  firstMatchingRow,
  sectionForField,
  sectionHasMatches,
  SETTINGS_SECTIONS,
  type SettingsFormValues,
  type SettingsRowId,
  type SettingsSectionId,
} from '../lib/settingsSections';
import classes from './Settings.module.css';

const ENGINE_OPTIONS: ReadonlyArray<{ value: EngineKind; key: MessageKey }> = [
  { value: 'silero_native', key: 'settings.engine.silero_native' },
  { value: 'piper', key: 'settings.engine.piper' },
  { value: 'silero', key: 'settings.engine.silero' },
  { value: 'teratts', key: 'settings.engine.teratts' },
];

/// Pessimistic default used until `getAvailableEngines()` resolves: Piper
/// is always on; the Silero engines are treated as unavailable so users
/// don't briefly see them as enabled and click before the probe lands.
/// Reasons are catalog codes resolved through formatError at render time
/// (locale-aware, same path as the live probe results).
///
/// `reads_latin_natively` values here are placeholders for the type: the
/// English reading hint is gated on `availabilityProbed`, so the dialog reads
/// the capability only from the probe, which stamps it from the engine profile.
function pessimisticAvailability(): AvailabilityMap {
  return {
    piper: { available: true, reason: null, reads_latin_natively: false },
    silero: {
      available: false,
      reason: { code: 'settings.availability.probing_python' },
      reads_latin_natively: false,
    },
    silero_native: {
      available: false,
      reason: { code: 'settings.availability.probing_bundle' },
      reads_latin_natively: false,
    },
    teratts: {
      available: false,
      reason: { code: 'settings.availability.probing_bundle' },
      reads_latin_natively: true,
    },
  };
}

interface SettingsModalProps {
  opened: boolean;
  onClose: () => void;
  /** Called after the user saves successfully, so the caller can refresh its
   * local copy of UIConfig without re-invoking getConfig on every render. */
  onSaved?: () => void;
  /** Opens the dictionary editor. The editor lives at the AppShell level
   *  (one top-level instance, above every dialog), so Settings only asks
   *  the owner to open it. */
  onOpenDictionary?: () => void;
  /** Bumped by the owner whenever the dictionary editor closes; re-reads
   *  the entry count for the section summary. */
  dictionaryRevision?: number;
}

type Translator = ReturnType<typeof useT>;

const SPEAKER_OPTIONS: ReadonlyArray<{
  value: string;
  label: string;
  key: MessageKey | null;
}> = [
  ...SILERO_SPEAKERS.map((speaker) => ({
    value: speaker.id,
    label: speaker.label,
    key: null,
  })),
  // `random` is the only label that is prose rather than a voice name.
  { value: RANDOM_SPEAKER, label: '', key: 'settings.speaker.random' },
];

/** `random` is a ttsd-only feature (the Python wrapper picks a speaker per
 *  call); the native engine rejects it, so hide it for silero_native. */
function speakerOptionsForEngine(engine: EngineKind) {
  return engine === 'silero_native'
    ? SPEAKER_OPTIONS.filter((o) => o.value !== RANDOM_SPEAKER)
    : SPEAKER_OPTIONS;
}

const SAMPLE_RATES = [8000, 24000, 48000];

// Factory defaults for Reset. A module-level constant (not read back from the
// form): form state updates are async, so form.getValues() inside the click
// handler still returns the pre-reset values.
const SETTINGS_DEFAULTS: SettingsFormValues = {
  engine: 'teratts',
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
  reading_width: canonicalReadingWidth(null),
  reading_scale: READING_SCALE_DEFAULT,
  theme: 'auto',
  language: 'ru',
};

function formatMb(bytes: number, t: Translator): string {
  return `${formatMbValue(bytes)} ${t('common.mb')}`;
}

interface CleanupCacheModalProps {
  opened: boolean;
  defaultTargetMb: number;
  onClose: () => void;
  /** Fired after a successful clear so callers can refresh stats. */
  onCleared?: () => void;
}

function CleanupCacheModal({
  opened,
  defaultTargetMb,
  onClose,
  onCleared,
}: CleanupCacheModalProps) {
  const tt = useT();
  const [targetMb, setTargetMb] = useState<number>(defaultTargetMb);
  const [deleteTexts, setDeleteTexts] = useState(false);
  const [cleanFully, setCleanFully] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [stats, setStats] = useState<{ total_bytes: number; audio_file_count: number } | null>(
    null,
  );

  useEffect(() => {
    if (!opened) return;
    setTargetMb(defaultTargetMb);
    setDeleteTexts(false);
    setCleanFully(false);
    commands.getCacheStats().then(setStats).catch(() => setStats(null));
  }, [opened, defaultTargetMb]);

  const dangerous = cleanFully && deleteTexts;

  const handleConfirm = async () => {
    const mode: CleanupMode = cleanFully
      ? { mode: 'all' }
      : { mode: 'size_limit', target_mb: targetMb };
    setSubmitting(true);
    try {
      const result = await commands.clearCache({ mode, delete_texts: deleteTexts });
      const parts: string[] = [];
      if (result.deleted_entries > 0) {
        parts.push(tt('settings.cleanup.deleted_entries', [result.deleted_entries]));
      }
      parts.push(tt('settings.cleanup.files', [result.deleted_files]));
      parts.push(tt('settings.cleanup.freed', [formatMb(result.freed_bytes, tt)]));
      notifications.show({
        title: tt('settings.cleanup.done.title'),
        message: parts.join(', '),
        color: 'green',
      });
      onCleared?.();
      onClose();
    } catch (err) {
      notifications.show({
        title: tt('settings.cleanup.failed.title'),
        message: formatError(err),
        color: 'red',
      });
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <Modal opened={opened} onClose={onClose} title={tt('settings.cleanup.title')} size="md" centered>
      <Stack gap="sm">
        {stats && (
          <Text size="sm" c="dimmed">
            {tt('settings.cleanup.stats', [
              formatMb(stats.total_bytes, tt),
              stats.audio_file_count,
            ])}
          </Text>
        )}

        <NumberInput
          label={tt('settings.cleanup.target.label')}
          description={tt('settings.cleanup.target.description')}
          min={0}
          value={targetMb}
          onChange={(v) =>
            setTargetMb(typeof v === 'number' ? v : parseInt(String(v || 0), 10) || 0)
          }
          disabled={cleanFully}
        />

        <Checkbox
          label={tt('settings.cleanup.delete_texts')}
          description={tt('settings.cleanup.delete_texts.description')}
          checked={deleteTexts}
          onChange={(e) => setDeleteTexts(e.currentTarget.checked)}
        />

        <Checkbox
          label={tt('settings.cleanup.full')}
          description={tt('settings.cleanup.full.description')}
          checked={cleanFully}
          onChange={(e) => setCleanFully(e.currentTarget.checked)}
        />

        {dangerous && (
          <Alert color="red" variant="light">
            {tt('settings.cleanup.dangerous_warning')}
          </Alert>
        )}

        <Group justify="flex-end" mt="sm">
          <Button variant="subtle" onClick={onClose} disabled={submitting}>
            {tt('common.cancel')}
          </Button>
          <Button color={dangerous ? 'red' : 'blue'} loading={submitting} onClick={handleConfirm}>
            {cleanFully ? tt('settings.cleanup.confirm_full') : tt('settings.cleanup.confirm_partial')}
          </Button>
        </Group>
      </Stack>
    </Modal>
  );
}

export function SettingsModal({
  opened,
  onClose,
  onSaved,
  onOpenDictionary,
  dictionaryRevision,
}: SettingsModalProps) {
  const tt = useT();
  const { setColorScheme } = useMantineColorScheme();
  const [cleanupOpen, setCleanupOpen] = useState(false);
  const [modelsOpen, setModelsOpen] = useState(false);
  const [dictionaryCount, setDictionaryCount] = useState<number | null>(null);
  const [cacheDir, setCacheDir] = useState<string>('');
  const [appVersion, setAppVersion] = useState<string>('');
  const [logDir, setLogDir] = useState<string>('');
  const [updaterEnabled, setUpdaterEnabled] = useState(false);
  const [coercedAlert, setCoercedAlert] = useState(false);
  const [availability, setAvailability] = useState<AvailabilityMap>(() =>
    pessimisticAvailability(),
  );
  /** Whether `availability` already holds a probe result. The pessimistic
   *  placeholder only carries placeholder capabilities, so the English reading
   *  hint waits for the probe instead of reading a hard-coded engine table. */
  const [availabilityProbed, setAvailabilityProbed] = useState(false);
  /** Voice ids shipped by the installed TeraTTS bundle; ids are the model's
   *  own names, so the id is also the label. */
  const [terattsVoices, setTerattsVoices] = useState<readonly string[]>([]);
  const refreshAvailability = useCallback(() => {
    commands
      .getAvailableEngines()
      .then((probed) => {
        setAvailability(probed);
        setAvailabilityProbed(true);
      })
      .catch(() => {});
    commands.getTerattsVoices().then(setTerattsVoices).catch(() => {});
  }, []);
  // Whether the user touched the sample-rate selector in this dialog
  // session. The config default is 24000 (the native engine's own default);
  // picking «Silero (нативный)» follows it only while the user made no
  // explicit choice.
  const sampleRateTouchedRef = useRef(false);
  const [baseline, setBaseline] = useState<SettingsFormValues>(SETTINGS_DEFAULTS);
  const [activeSection, setActiveSection] = useState<SettingsSectionId>('engine');
  const [query, setQuery] = useState('');
  /** Row the search Enter should reveal once its section is rendered. */
  const [jumpTarget, setJumpTarget] = useState<SettingsRowId | null>(null);
  /** Form field a failed submit should focus once its section is rendered. */
  const [validationFocus, setValidationFocus] = useState<string | null>(null);
  const scrollRef = useRef<HTMLDivElement>(null);
  /** Below this width the nav collapses into a select above the content. */
  const isNarrow = useMediaQuery('(max-width: 700px)', false, {
    getInitialValueInEffect: false,
  });
  const form = useForm<SettingsFormValues>({
    initialValues: SETTINGS_DEFAULTS,
    validate: {
      max_cache_size_mb: (v) =>
        // Non-reactive t(): Mantine captures the validator once, so a
        // mid-session locale switch must be picked up at call time.
        v < 100 ? t('settings.max_cache_size.min_error') : null,
    },
  });

  // Dictionary entry count for the section summary; re-read when the owner
  // reports the dictionary editor closed (dictionaryRevision bump).
  useEffect(() => {
    if (!opened) return;
    commands
      .getDictionaries()
      .then((collections) =>
        setDictionaryCount(collections.reduce((sum, c) => sum + c.entries.length, 0)),
      )
      .catch(() => setDictionaryCount(null));
  }, [opened, dictionaryRevision]);

  useEffect(() => {
    if (!opened) return;
    sampleRateTouchedRef.current = false;
    Promise.all([commands.getConfig(), commands.getAvailableEngines()])
      .then(([config, probed]) => {
        setAvailability(probed);
        setAvailabilityProbed(true);
        const initial = computeEngineFormState(config, probed);
        const loaded = {
          engine: initial.engine,
          piper_voice: initial.piperVoice,
          teratts_voice: initial.terattsVoice,
          speaker: initial.sileroSpeaker,
          sample_rate: config.sample_rate,
          notify_on_ready: config.notify_on_ready,
          notify_on_error: config.notify_on_error,
          preview_dialog_enabled: config.preview_dialog_enabled,
          hide_to_tray_on_close: config.hide_to_tray_on_close,
          max_cache_size_mb: config.max_cache_size_mb,
          code_block_mode: config.code_block_mode,
          english_reading: config.english_reading,
          reading_width: config.reading_width,
          reading_scale: config.reading_scale,
          theme: config.theme,
          language: config.language,
        };
        form.setValues(loaded);
        // Dirty tracking compares against the saved config, not the static
        // initialValues (and stores the coerced engine, so an engine the user
        // never picked is not reported as a change).
        setBaseline(loaded);
        setCoercedAlert(initial.coercedAwayFromUnavailable);
      })
      .catch((err) => {
        notifications.show({
          title: tt('settings.load_failed.title'),
          message: formatError(err),
          color: 'red',
        });
      });
    commands.getCacheDir().then(setCacheDir).catch(() => setCacheDir(''));
    // A catalogue failure must not fail the dialog: the selector falls back
    // to the saved id and the availability reason explains the engine state.
    commands.getTerattsVoices().then(setTerattsVoices).catch(() => setTerattsVoices([]));
    // Always resolve the version (independent of the updater gate) so bug
    // reports can quote it on every platform.
    getVersion().then(setAppVersion).catch(() => setAppVersion(''));
    commands.getLogDir().then(setLogDir).catch(() => setLogDir(''));
    // Update section only on installs the updater can serve (#226): Windows
    // and Linux AppImage; a failed probe hides it like .deb/nix do.
    updaterSupported().then(setUpdaterEnabled).catch(() => setUpdaterEnabled(false));
    // form is excluded intentionally: setValues is stable, re-running on form change would loop
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [opened]);

  const piperVoiceOptions = useMemo(
    () => PIPER_VOICES.map((v) => ({ value: v.id, label: tt(v.key) })),
    [tt],
  );

  const speakerOptions = useMemo(
    () =>
      speakerOptionsForEngine(form.values.engine).map((o) => ({
        value: o.value,
        label: o.key ? tt(o.key) : o.label,
      })),
    [form.values.engine, tt],
  );

  const sampleRateOptions = useMemo(
    () => SAMPLE_RATES.map((hz) => ({ value: String(hz), label: tt('settings.sample_rate.hz', [hz]) })),
    [tt],
  );

  const terattsVoiceOptions = useMemo(() => {
    const options = terattsVoices.map((id) => ({ value: id, label: id }));
    const saved = form.values.teratts_voice;
    // Keep a stored id the manifest does not list selectable: otherwise the
    // selector shows no selection and a save looks like a silent voice change.
    if (saved && !terattsVoices.includes(saved)) {
      options.push({ value: saved, label: saved });
    }
    return options;
  }, [terattsVoices, form.values.teratts_voice]);

  const engineOptions = useMemo(
    () =>
      ENGINE_OPTIONS.map((opt) => ({
        value: opt.value,
        label: tt(opt.key),
        disabled: !availability[opt.value].available,
      })),
    [tt, availability],
  );

  const handleEngineChange = (next: EngineKind) => {
    const current = {
      engine: form.values.engine,
      piperVoice: form.values.piper_voice,
      sileroSpeaker: form.values.speaker,
      terattsVoice: form.values.teratts_voice,
      coercedAwayFromUnavailable: coercedAlert,
    };
    const updated = applyEngineChange(current, next, availability);
    form.setFieldValue('engine', updated.engine);
    form.setFieldValue('teratts_voice', updated.terattsVoice);
    // applyEngineChange coerces ttsd-only speakers (e.g. 'random') away
    // when the native engine is picked.
    form.setFieldValue('speaker', updated.sileroSpeaker);
    setCoercedAlert(updated.coercedAwayFromUnavailable);
    // The native engine defaults to 24000; follow it only while the user
    // hasn't explicitly picked a sample rate in this dialog session.
    if (next === 'silero_native' && !sampleRateTouchedRef.current) {
      form.setFieldValue('sample_rate', 24000);
    }
  };

  // Reset the nested cleanup modal's visibility when Settings closes, so
  // reopening Settings does not resurrect a stale nested-open state.
  useEffect(() => {
    if (!opened) {
      setCleanupOpen(false);
      setModelsOpen(false);
      setQuery('');
    }
  }, [opened]);

  const handleOpenCacheDir = async () => {
    if (!cacheDir) return;
    try {
      // revealItemInDir accepts a file or a directory. For the cache we pass
      // history.json as a stable marker; the log handler below passes the
      // directory path directly (the freshly created log dir may be empty).
      await revealItemInDir(`${cacheDir}/history.json`);
    } catch (err) {
      notifications.show({
        title: tt('settings.open_folder_failed.title'),
        message: formatError(err),
        color: 'red',
      });
    }
  };

  const handleOpenLogDir = async () => {
    if (!logDir) return;
    try {
      await revealItemInDir(logDir);
    } catch (err) {
      notifications.show({
        title: tt('settings.open_folder_failed.title'),
        message: formatError(err),
        color: 'red',
      });
    }
  };

  const handleSubmit = async (values: SettingsFormValues) => {
    const patch = buildSettingsPatch(
      {
        engine: values.engine,
        piper_voice: values.piper_voice,
        teratts_voice: values.teratts_voice,
        speaker: values.speaker,
        sample_rate: values.sample_rate,
        notify_on_ready: values.notify_on_ready,
        notify_on_error: values.notify_on_error,
        preview_dialog_enabled: values.preview_dialog_enabled,
        hide_to_tray_on_close: values.hide_to_tray_on_close,
        max_cache_size_mb: values.max_cache_size_mb,
        code_block_mode: values.code_block_mode,
        english_reading: values.english_reading,
        reading_width: values.reading_width,
        reading_scale: values.reading_scale,
        theme: values.theme as UIConfigPatch['theme'],
        language: values.language,
      },
      coercedAlert,
    );

    try {
      await commands.updateConfig(patch);
      // Mantine doesn't observe backend config; push the new theme into the
      // color-scheme manager directly so the UI reflects the change without
      // waiting for a reload.
      setColorScheme(values.theme as MantineColorScheme);
      notifications.show({
        title: tt('settings.saved.title'),
        message: tt('settings.saved.message'),
        color: 'green',
      });
      onSaved?.();
      onClose();
    } catch (err) {
      notifications.show({
        title: tt('settings.save_failed.title'),
        message: formatError(err),
        color: 'red',
      });
      // The backend rolls the tray setting back when the icon cannot be
      // created, so re-read the stored value and show it instead of the
      // rejected one.
      commands
        .getConfig()
        .then((config) => {
          form.setValues({
            ...form.values,
            hide_to_tray_on_close: config.hide_to_tray_on_close,
          });
          // The rejected value is not the stored one: move the baseline with
          // it so the rollback does not look like an unsaved change.
          setBaseline((current) => ({
            ...current,
            hide_to_tray_on_close: config.hide_to_tray_on_close,
          }));
        })
        .catch(() => {});
    }
  };

  const handleSampleRateChange = (next: number) => {
    sampleRateTouchedRef.current = true;
    form.setFieldValue('sample_rate', next);
  };

  const handleLanguageChange = (next: string) => {
    if (!next) return;
    // Spec: the locale store updates IMMEDIATELY so the whole UI relabels
    // before the dialog is saved.
    form.setFieldValue('language', next);
    setLocale(toLocale(next));
  };

  const handleReset = () => {
    // Reset restores factory defaults and clears field errors. The language
    // selector relabels the whole UI on change; form state updates are async,
    // so sync the locale store from the defaults constant, not getValues().
    form.reset();
    form.setValues(SETTINGS_DEFAULTS);
    setLocale(toLocale(SETTINGS_DEFAULTS.language));
  };

  const activeQuery = query.trim();
  // One availability predicate feeds both the rendered rows and the search
  // index, so a row the dialog hides cannot match or be an Enter target.
  const availableRows = useMemo(
    () => availableSettingsRows({ updaterEnabled, cacheDir, logDir }),
    [updaterEnabled, cacheDir, logDir],
  );

  const handleSearchEnter = () => {
    const first = firstMatchingRow(query, availableRows);
    if (!first) return;
    setActiveSection(first.section);
    setJumpTarget(first.id);
  };

  // Escape peels the query off first; the handler sits on the document because
  // the modal header and the padding around the form never reach the form's
  // key events. Once the query is empty the modal's own `closeOnEscape` takes
  // over, and the nested cleanup modal keeps Escape to itself.
  useEffect(() => {
    if (!opened) return;
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key !== 'Escape' || cleanupOpen || activeQuery === '') return;
      event.preventDefault();
      setQuery('');
    };
    document.addEventListener('keydown', handleKeyDown, true);
    return () => document.removeEventListener('keydown', handleKeyDown, true);
  }, [opened, cleanupOpen, activeQuery]);

  const handleValidationFailure = (errors: FormErrors) => {
    const firstField = Object.keys(errors)[0];
    if (!firstField) return;
    // The invalid field may live in another section, or be filtered out by the
    // search: drop the query, jump there and focus the control so the error
    // cannot hide behind an inactive pane.
    const section = sectionForField(firstField);
    setQuery('');
    if (section) setActiveSection(section);
    setValidationFocus(firstField);
  };

  const matchingSections = SETTINGS_SECTIONS.filter((section) =>
    sectionHasMatches(section.id, query, availableRows),
  );
  const visibleSections = activeQuery ? matchingSections : SETTINGS_SECTIONS;
  const currentSection =
    visibleSections.find((section) => section.id === activeSection) ??
    visibleSections[0] ??
    null;

  const changedFields = changedSettingsFields(form.values, baseline);
  const dirtySections = changedSettingsSections(changedFields);
  const dirtyCount = changedFields.length;
  const piperVoiceRecommended =
    PIPER_VOICES.find((voice) => voice.id === form.values.piper_voice)?.recommended === true;

  // The panes swap their content in place; keep the reading position
  // predictable when the section changes (including a search-driven jump).
  useEffect(() => {
    if (scrollRef.current) scrollRef.current.scrollTop = 0;
  }, [currentSection?.id]);

  // Enter moves from the query to the first match: scroll it into view after
  // the section swap has committed.
  useEffect(() => {
    if (!jumpTarget) return;
    const container = scrollRef.current;
    const element = container?.querySelector(`[data-settings-field="${jumpTarget}"]`);
    element?.scrollIntoView({ block: 'nearest' });
    // Hand keyboard focus to the section title so a screen-reader user hears
    // where the jump landed instead of being left on the search input.
    container?.querySelector<HTMLElement>('[data-settings-section-title]')?.focus();
    setJumpTarget(null);
  }, [jumpTarget, currentSection?.id]);

  // A failed submit lands on the invalid control after the section swap.
  useEffect(() => {
    if (!validationFocus) return;
    const row = scrollRef.current?.querySelector<HTMLElement>(
      `[data-settings-field="${validationFocus}"]`,
    );
    const control =
      row?.querySelector<HTMLElement>('input, select, textarea, button') ?? row;
    control?.focus();
    setValidationFocus(null);
  }, [validationFocus, currentSection?.id]);

  const searchInput = (
    <div data-settings-search>
      <TextInput
        size="sm"
        placeholder={tt('settings.search.placeholder')}
        aria-label={tt('settings.search.placeholder')}
        leftSection={<IconSearch size={14} />}
        value={query}
        onChange={(event) => setQuery(event.currentTarget.value)}
        onKeyDown={(event) => {
          if (event.key === 'Enter') {
            // Enter in a text input submits the form by default; the query
            // belongs to navigation, not to saving.
            event.preventDefault();
            handleSearchEnter();
          }
        }}
      />
    </div>
  );

  return (
    <Modal
      opened={opened}
      onClose={onClose}
      title={tt('settings.title')}
      size={900}
      classNames={{ content: classes.content, body: classes.body }}
      // While the nested cleanup modal is up, let it own Escape and outside-
      // click handling — otherwise Mantine fires both modals' onClose at once.
      // A non-empty query takes Esc for itself (clears the search first).
      closeOnEscape={!cleanupOpen && activeQuery === ''}
      closeOnClickOutside={!cleanupOpen}
    >
      <form
        className={classes.form}
        onSubmit={form.onSubmit(handleSubmit, handleValidationFailure)}
      >
        {isNarrow && (
          <div className={classes.narrowTop}>
            {searchInput}
            <div data-settings-nav-select>
              <Select
                size="sm"
                label={tt('settings.nav.select_label')}
                data={visibleSections.map((section) => ({
                  value: section.id,
                  label: tt(section.titleKey),
                }))}
                value={currentSection?.id ?? null}
                onChange={(value) => {
                  if (value) setActiveSection(value as SettingsSectionId);
                }}
                allowDeselect={false}
                renderOption={({ option }) => {
                  const section = SETTINGS_SECTIONS.find(
                    (candidate) => candidate.id === option.value,
                  );
                  const dirty = section ? dirtySections.has(section.id) : false;
                  return (
                    <Group justify="space-between" wrap="nowrap" gap="xs" w="100%">
                      <span>{option.label}</span>
                      {dirty && (
                        <span
                          className={classes.dirtyDot}
                          data-settings-dirty-dot
                          title={tt('settings.dirty.section')}
                          aria-hidden="true"
                        />
                      )}
                    </Group>
                  );
                }}
              />
            </div>
          </div>
        )}

        <div className={classes.panes}>
          {!isNarrow && (
            <div className={classes.nav} data-settings-nav>
              {searchInput}
              <nav className={classes.navList} aria-label={tt('settings.nav.label')}>
                {visibleSections.map((section) => {
                  const dirty = dirtySections.has(section.id);
                  const current = currentSection?.id === section.id;
                  return (
                    <UnstyledButton
                      key={section.id}
                      id={`settings-nav-${section.id}`}
                      type="button"
                      aria-current={current ? 'true' : undefined}
                      className={`${classes.navItem} ${current ? classes.navItemActive : ''}`}
                      onClick={() => setActiveSection(section.id)}
                    >
                      <span className={classes.navLabel}>{tt(section.titleKey)}</span>
                      {dirty && (
                        <span
                          className={classes.dirtyDot}
                          data-settings-dirty-dot
                          title={tt('settings.dirty.section')}
                          aria-hidden="true"
                        />
                      )}
                    </UnstyledButton>
                  );
                })}
              </nav>
            </div>
          )}

          <div className={classes.contentPane}>
            <div className={classes.scroll} ref={scrollRef} data-settings-scroll>
              {currentSection ? (
                <>
                  <div data-settings-section-title={currentSection.id} tabIndex={-1}>
                    <Text className={classes.sectionTitle}>
                      {tt(currentSection.titleKey)}
                    </Text>
                  </div>
                  <SettingsSections
                    section={currentSection.id}
                    query={query}
                    available={availableRows}
                    form={form}
                    availability={availability}
                    availabilityProbed={availabilityProbed}
                    coercedAlert={coercedAlert}
                    engineOptions={engineOptions}
                    piperVoiceOptions={piperVoiceOptions}
                    piperVoiceRecommended={piperVoiceRecommended}
                    speakerOptions={speakerOptions}
                    sampleRateOptions={sampleRateOptions}
                    terattsVoiceOptions={terattsVoiceOptions}
                    cacheDir={cacheDir}
                    logDir={logDir}
                    appVersion={appVersion}
                    dictionaryCount={dictionaryCount}
                    onEngineChange={handleEngineChange}
                    onSampleRateChange={handleSampleRateChange}
                    onOpenModels={() => setModelsOpen(true)}
                    onOpenCacheDir={() => void handleOpenCacheDir()}
                    onOpenLogDir={() => void handleOpenLogDir()}
                    onOpenCleanup={() => setCleanupOpen(true)}
                    onOpenDictionary={() => onOpenDictionary?.()}
                    onCheckUpdates={() => void checkForUpdatesManual()}
                    onLanguageChange={handleLanguageChange}
                  />
                </>
              ) : (
                <div data-settings-no-results role="status" aria-live="polite">
                  <Text size="sm" c="dimmed">
                    {tt('settings.search.no_results')}
                  </Text>
                </div>
              )}
            </div>
          </div>
        </div>

        <footer className={classes.footer} data-settings-footer>
          <Text size="sm" c="dimmed" role="status" aria-live="polite">
            {tt('settings.dirty.count', [dirtyCount])}
          </Text>
          <Group gap="sm">
            <Button variant="subtle" onClick={handleReset}>
              {tt('settings.reset')}
            </Button>
            <Button type="submit">{tt('common.save')}</Button>
          </Group>
        </footer>
      </form>

      <ModelAssetsDialog
        opened={modelsOpen}
        onClose={() => setModelsOpen(false)}
        onAvailabilityChanged={refreshAvailability}
      />

      <CleanupCacheModal
        opened={cleanupOpen}
        defaultTargetMb={form.values.max_cache_size_mb}
        onClose={() => setCleanupOpen(false)}
      />
    </Modal>
  );
}
