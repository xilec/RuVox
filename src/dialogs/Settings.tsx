import { useEffect, useMemo, useRef, useState } from 'react';
import {
  Alert,
  Badge,
  Button,
  Checkbox,
  Divider,
  Group,
  Modal,
  NumberInput,
  Progress,
  SegmentedControl,
  Select,
  Stack,
  Switch,
  Text,
  Tooltip,
  useMantineColorScheme,
} from '@mantine/core';
import type { MantineColorScheme } from '@mantine/core';
import { useForm } from '@mantine/form';
import { notifications } from '@mantine/notifications';
import { revealItemInDir } from '@tauri-apps/plugin-opener';
import { getVersion } from '@tauri-apps/api/app';
import { commands, events } from '../lib/tauri';
import { formatMb as formatMbValue } from '../lib/format';
import type { CleanupMode, EngineKind, UIConfigPatch } from '../lib/tauri';
import type { MessageKey } from '../i18n/ru';
import { formatError } from '../lib/errors';
import { t, useT } from '../lib/i18n';
import { setLocale, toLocale } from '../stores/locale';
import { bundleDownloadPercent } from '../lib/bundleDownload';
import { PIPER_VOICES } from '../lib/piperVoices';
import { checkForUpdatesManual, updaterSupported } from '../lib/updater';
import { DictionaryModal } from './DictionaryModal';
import {
  applyEngineChange,
  buildSettingsPatch,
  computeEngineFormState,
  RANDOM_SPEAKER,
  type AvailabilityMap,
} from '../lib/engineSelection';

interface SettingsFormValues {
  engine: EngineKind;
  piper_voice: string;
  speaker: string;
  sample_rate: number;
  notify_on_ready: boolean;
  notify_on_error: boolean;
  preview_dialog_enabled: boolean;
  max_cache_size_mb: number;
  code_block_mode: string;
  theme: string;
  language: string;
}

const ENGINE_OPTIONS: ReadonlyArray<{ value: EngineKind; key: MessageKey }> = [
  { value: 'silero_native', key: 'settings.engine.silero_native' },
  { value: 'piper', key: 'settings.engine.piper' },
  { value: 'silero', key: 'settings.engine.silero' },
];

/// Pessimistic default used until `getAvailableEngines()` resolves: Piper
/// is always on; the Silero engines are treated as unavailable so users
/// don't briefly see them as enabled and click before the probe lands.
/// Reasons are catalog codes resolved through formatError at render time
/// (locale-aware, same path as the live probe results).
function pessimisticAvailability(): AvailabilityMap {
  return {
    piper: { available: true, reason: null },
    silero: { available: false, reason: { code: 'settings.availability.probing_python' } },
    silero_native: {
      available: false,
      reason: { code: 'settings.availability.probing_bundle' },
    },
  };
}

interface SettingsModalProps {
  opened: boolean;
  onClose: () => void;
  /** Called after the user saves successfully, so the caller can refresh its
   * local copy of UIConfig without re-invoking getConfig on every render. */
  onSaved?: () => void;
  /** Called when the dictionary editor closes — entries may have changed. */
  onDictionaryChanged?: () => void;
}

type Translator = ReturnType<typeof useT>;

const SPEAKER_OPTIONS: ReadonlyArray<{
  value: string;
  label: string;
  key: MessageKey | null;
}> = [
  { value: 'aidar', label: 'Aidar', key: null },
  { value: 'baya', label: 'Baya', key: null },
  { value: 'kseniya', label: 'Kseniya', key: null },
  { value: 'xenia', label: 'Xenia', key: null },
  { value: 'eugene', label: 'Eugene', key: null },
  // `random` is the only label that is prose rather than a voice name.
  { value: 'random', label: '', key: 'settings.speaker.random' },
];

/** `random` is a ttsd-only feature (the Python wrapper picks a speaker per
 *  call); the native engine rejects it, so hide it for silero_native. */
function speakerOptionsForEngine(engine: EngineKind) {
  return engine === 'silero_native'
    ? SPEAKER_OPTIONS.filter((o) => o.value !== RANDOM_SPEAKER)
    : SPEAKER_OPTIONS;
}

const SAMPLE_RATES = [8000, 24000, 48000];

const THEME_OPTIONS: ReadonlyArray<{ value: string; key: MessageKey }> = [
  { value: 'light', key: 'settings.theme.light' },
  { value: 'dark', key: 'settings.theme.dark' },
  { value: 'auto', key: 'settings.theme.auto' },
];

const LANGUAGE_OPTIONS = [
  { value: 'ru', label: 'Русский' },
  { value: 'en', label: 'English' },
];

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

export function SettingsModal({ opened, onClose, onSaved, onDictionaryChanged }: SettingsModalProps) {
  const tt = useT();
  const { setColorScheme } = useMantineColorScheme();
  const [cleanupOpen, setCleanupOpen] = useState(false);
  const [dictionaryOpen, setDictionaryOpen] = useState(false);
  const [dictionaryCount, setDictionaryCount] = useState<number | null>(null);
  const [cacheDir, setCacheDir] = useState<string>('');
  const [appVersion, setAppVersion] = useState<string>('');
  const [logDir, setLogDir] = useState<string>('');
  const [updaterEnabled, setUpdaterEnabled] = useState(false);
  const [coercedAlert, setCoercedAlert] = useState(false);
  const [availability, setAvailability] = useState<AvailabilityMap>(() =>
    pessimisticAvailability(),
  );
  // Live bundle-download state: null when idle, otherwise the current file
  // and the overall percentage derived from the progress events.
  const [bundleDownload, setBundleDownload] = useState<{ file: string; percent: number } | null>(
    null,
  );
  // Whether a download session is active — lets the invoke catch distinguish
  // "command failed before starting" (report here) from "failed mid-download"
  // (already reported by the finished event).
  const downloadActiveRef = useRef(false);
  // Whether the user touched the sample-rate selector in this dialog
  // session. The config default is 24000 (the native engine's own default);
  // picking «Silero (нативный)» follows it only while the user made no
  // explicit choice.
  const sampleRateTouchedRef = useRef(false);
  // Factory defaults for Reset. A module-level constant (not read back from
  // the form): form state updates are async, so form.getValues() inside the
  // click handler still returns the pre-reset values.
  const SETTINGS_DEFAULTS: SettingsFormValues = {
    engine: 'silero_native',
    piper_voice: 'ruslan',
    speaker: 'aidar',
    sample_rate: 24000,
    notify_on_ready: true,
    notify_on_error: true,
    preview_dialog_enabled: true,
    max_cache_size_mb: 500,
    code_block_mode: 'brief',
    theme: 'auto',
    language: 'ru',
  };
  const form = useForm<SettingsFormValues>({
    initialValues: SETTINGS_DEFAULTS,
    validate: {
      max_cache_size_mb: (v) =>
        // Non-reactive t(): Mantine captures the validator once, so a
        // mid-session locale switch must be picked up at call time.
        v < 100 ? t('settings.max_cache_size.min_error') : null,
    },
  });

  useEffect(() => {
    if (!opened) return;
    sampleRateTouchedRef.current = false;
    Promise.all([commands.getConfig(), commands.getAvailableEngines()])
      .then(([config, probed]) => {
        setAvailability(probed);
        const initial = computeEngineFormState(config, probed);
        const loaded = {
          engine: initial.engine,
          piper_voice: initial.piperVoice,
          speaker: initial.sileroSpeaker,
          sample_rate: config.sample_rate,
          notify_on_ready: config.notify_on_ready,
          notify_on_error: config.notify_on_error,
          preview_dialog_enabled: config.preview_dialog_enabled,
          max_cache_size_mb: config.max_cache_size_mb,
          code_block_mode: config.code_block_mode,
          theme: config.theme,
          language: config.language,
        };
        form.setValues(loaded);
        // Dirty tracking compares against the saved config, not the static
        // initialValues.
        form.resetDirty(loaded);
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
    // Always resolve the version (independent of the updater gate) so bug
    // reports can quote it on every platform.
    getVersion().then(setAppVersion).catch(() => setAppVersion(''));
    commands.getLogDir().then(setLogDir).catch(() => setLogDir(''));
    // Dictionary entry count for the section summary; re-read when the
    // dictionary editor closes (see its onClose).
    commands
      .getUserDictionary()
      .then((entries) => setDictionaryCount(entries.length))
      .catch(() => setDictionaryCount(null));
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

  const handleEngineChange = (next: EngineKind) => {
    const current = {
      engine: form.values.engine,
      piperVoice: form.values.piper_voice,
      sileroSpeaker: form.values.speaker,
      coercedAwayFromUnavailable: coercedAlert,
    };
    const updated = applyEngineChange(current, next, availability);
    form.setFieldValue('engine', updated.engine);
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

  // Live bundle-download progress, driven by the backend's
  // bundle_download_* events. Subscribed only while the dialog is open.
  useEffect(() => {
    if (!opened) return;
    const unlisteners = [
      events.bundleDownloadStarted(() => {
        downloadActiveRef.current = true;
        setBundleDownload({ file: 'manifest.json', percent: 0 });
      }),
      events.bundleDownloadProgress((p) => {
        setBundleDownload({ file: p.file, percent: bundleDownloadPercent(p) });
      }),
      events.bundleDownloadFinished((p) => {
        downloadActiveRef.current = false;
        setBundleDownload(null);
        if (p.ok) {
          notifications.show({
            title: tt('settings.bundle.finished.title'),
            message: tt('settings.bundle.finished.message'),
            color: 'green',
          });
          // Re-probe so the engine option becomes selectable immediately.
          commands.getAvailableEngines().then(setAvailability).catch(() => {});
        } else {
          notifications.show({
            title: tt('settings.bundle.failed.title'),
            message: p.message ?? tt('bundle.unknown_error'),
            color: 'red',
          });
        }
      }),
    ];
    return () => {
      unlisteners.forEach((u) => {
        u.then((fn) => fn()).catch(() => {});
      });
    };
    // `tt` re-subscribes on locale switch so toasts use the active catalog.
  }, [opened, tt]);

  const handleBundleDownload = () => {
    // Switch to the progress view immediately — the started event lands one
    // IPC round-trip later, and without this the button stays clickable and
    // the user can queue a second download.
    setBundleDownload({ file: 'manifest.json', percent: 0 });
    commands.downloadSileroNativeBundle().catch((err) => {
      // Mid-download failures are already reported by the
      // bundle_download_finished { ok: false } event; only a command that
      // failed before starting needs a notification here.
      if (!downloadActiveRef.current) {
        setBundleDownload(null);
        notifications.show({
          title: tt('settings.bundle.failed.title'),
          message: formatError(err),
          color: 'red',
        });
      }
    });
  };

  // Reset the nested cleanup modal's visibility when Settings closes, so
  // reopening Settings does not resurrect a stale nested-open state.
  useEffect(() => {
    if (!opened) setCleanupOpen(false);
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
        speaker: values.speaker,
        sample_rate: values.sample_rate,
        notify_on_ready: values.notify_on_ready,
        notify_on_error: values.notify_on_error,
        preview_dialog_enabled: values.preview_dialog_enabled,
        max_cache_size_mb: values.max_cache_size_mb,
        code_block_mode: values.code_block_mode,
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
    }
  };

  return (
    <Modal
      opened={opened}
      onClose={onClose}
      title={tt('settings.title')}
      size="md"
      // While the nested cleanup modal is up, let it own Escape and outside-
      // click handling — otherwise Mantine fires both modals' onClose at once.
      closeOnEscape={!cleanupOpen}
      closeOnClickOutside={!cleanupOpen}
    >
      <form onSubmit={form.onSubmit(handleSubmit)}>
        <Stack gap="sm">
          <Text size="sm" fw={500} c="dimmed">
            {tt('settings.section.speech')}
          </Text>

          {coercedAlert && (
            <Alert color="yellow" variant="light">
              {tt('settings.coerced_alert')}
            </Alert>
          )}

          <Select
            label={tt('settings.engine.label')}
            description={tt('settings.engine.description')}
            data={ENGINE_OPTIONS.map((opt) => ({
              value: opt.value,
              label: tt(opt.key),
              disabled: !availability[opt.value].available,
            }))}
            value={form.values.engine}
            onChange={(v) => v && handleEngineChange(v as EngineKind)}
          />

          {!availability.silero.available && availability.silero.reason && (
            <Text size="xs" c="dimmed" mt={-8}>
              {tt('settings.engine.reason_silero', [formatError(availability.silero.reason)])}
            </Text>
          )}

          {!availability.silero_native.available && availability.silero_native.reason && (
            <Text size="xs" c="dimmed" mt={-8}>
              {tt('settings.engine.reason_silero_native', [formatError(availability.silero_native.reason)])}
            </Text>
          )}

          {!availability.silero_native.available &&
            (bundleDownload ? (
              <Stack gap={4}>
                <Text size="xs" c="dimmed">
                  {tt('settings.bundle.downloading', [
                    bundleDownload.file,
                    Math.round(bundleDownload.percent),
                  ])}
                </Text>
                <Progress value={bundleDownload.percent} animated />
              </Stack>
            ) : (
              <Group justify="flex-start">
                <Button variant="default" size="xs" onClick={handleBundleDownload}>
                  {tt('settings.bundle.download_button')}
                </Button>
              </Group>
            ))}

          {form.values.engine === 'piper' ? (
            <Stack gap={6}>
              <Select
                label={tt('settings.piper_voice.label')}
                description={tt('settings.piper_voice.description')}
                data={piperVoiceOptions}
                key={form.key('piper_voice')}
                {...form.getInputProps('piper_voice')}
                rightSection={
                  PIPER_VOICES.find((v) => v.id === form.values.piper_voice)?.recommended ? (
                    <Tooltip label={tt('settings.piper_voice.recommended_tooltip')}>
                      <Badge size="xs" color="blue" variant="light">
                        {tt('settings.piper_voice.recommended_badge')}
                      </Badge>
                    </Tooltip>
                  ) : null
                }
                rightSectionWidth={60}
              />
              <Group justify="flex-start">
                <Button
                  variant="default"
                  size="xs"
                  onClick={() =>
                    commands.downloadPiperVoice(form.values.piper_voice).catch((err) => {
                      notifications.show({
                        title: tt('settings.piper_voice.download_failed.title'),
                        message: formatError(err),
                        color: 'red',
                      });
                    })
                  }
                >
                  {tt('settings.piper_voice.download_now')}
                </Button>
              </Group>
            </Stack>
          ) : (
            <Select
              label={tt('settings.speaker.label')}
              data={speakerOptions}
              key={form.key('speaker')}
              {...form.getInputProps('speaker')}
            />
          )}

          <Select
            label={tt('settings.sample_rate.label')}
            data={sampleRateOptions}
            value={String(form.values.sample_rate)}
            onChange={(v) => {
              sampleRateTouchedRef.current = true;
              form.setFieldValue('sample_rate', v ? parseInt(v, 10) : 24000);
            }}
            error={form.errors.sample_rate}
          />

          <Stack gap={4}>
            <Text size="sm">{tt('settings.code_block.label')}</Text>
            <SegmentedControl
              value={form.values.code_block_mode}
              onChange={(v) => form.setFieldValue('code_block_mode', v)}
              data={[
                { label: tt('settings.code_block.brief'), value: 'brief' },
                { label: tt('settings.code_block.read'), value: 'read' },
              ]}
            />
            <Text size="xs" c="dimmed">
              {tt('settings.code_block.description')}
            </Text>
          </Stack>

          <Divider />

          <Text size="sm" fw={500} c="dimmed">
            {tt('settings.section.dictionary')}
          </Text>

          <Group justify="flex-start">
            <Text size="xs" c="dimmed">
              {tt('settings.dictionary.entries_count', [dictionaryCount ?? '—'])}
            </Text>
            <Button variant="default" onClick={() => setDictionaryOpen(true)}>
              {tt('settings.dictionary.open')}
            </Button>
          </Group>

          <Divider />

          <Text size="sm" fw={500} c="dimmed">
            {tt('settings.section.notifications')}
          </Text>

          <Switch
            label={tt('settings.notify_on_ready')}
            key={form.key('notify_on_ready')}
            {...form.getInputProps('notify_on_ready', { type: 'checkbox' })}
          />

          <Switch
            label={tt('settings.notify_on_error')}
            key={form.key('notify_on_error')}
            {...form.getInputProps('notify_on_error', { type: 'checkbox' })}
          />

          <Divider />

          <Text size="sm" fw={500} c="dimmed">
            {tt('settings.section.preview')}
          </Text>

          <Switch
            label={tt('settings.preview_dialog_enabled')}
            key={form.key('preview_dialog_enabled')}
            {...form.getInputProps('preview_dialog_enabled', {
              type: 'checkbox',
            })}
          />

          <Divider />

          <Text size="sm" fw={500} c="dimmed">
            {tt('settings.section.cache')}
          </Text>

          <NumberInput
            label={tt('settings.max_cache_size.label')}
            description={tt('settings.max_cache_size.description')}
            min={100}
            key={form.key('max_cache_size_mb')}
            {...form.getInputProps('max_cache_size_mb')}
          />

          {cacheDir && (
            <Stack gap={4}>
              <Text size="xs" c="dimmed">
                {tt('settings.data_folder')}
              </Text>
              <Text size="sm" style={{ wordBreak: 'break-all', fontFamily: 'var(--mantine-font-family-monospace)' }}>
                {cacheDir}
              </Text>
            </Stack>
          )}

          <Group justify="flex-start">
            <Button variant="default" onClick={handleOpenCacheDir} disabled={!cacheDir}>
              {tt('settings.open_folder')}
            </Button>
            <Button variant="default" onClick={() => setCleanupOpen(true)}>
              {tt('settings.cleanup.button')}
            </Button>
          </Group>

          <Divider />

          <Text size="sm" fw={500} c="dimmed">
            {tt('settings.section.logs')}
          </Text>

          {logDir && (
            <Stack gap={4}>
              <Text size="xs" c="dimmed">
                {tt('settings.logs_folder')}
              </Text>
              <Text size="sm" style={{ wordBreak: 'break-all', fontFamily: 'var(--mantine-font-family-monospace)' }}>
                {logDir}
              </Text>
            </Stack>
          )}

          <Group justify="flex-start">
            <Button variant="default" onClick={handleOpenLogDir} disabled={!logDir}>
              {tt('settings.open_folder')}
            </Button>
          </Group>

          <Divider />

          <Text size="sm" fw={500} c="dimmed">
            {tt('settings.section.interface')}
          </Text>

          <Select
            label={tt('settings.theme.label')}
            data={THEME_OPTIONS.map((opt) => ({ value: opt.value, label: tt(opt.key) }))}
            key={form.key('theme')}
            {...form.getInputProps('theme')}
          />

          <Select
            label={tt('settings.language.label')}
            data={LANGUAGE_OPTIONS}
            key={form.key('language')}
            value={form.values.language}
            onChange={(v) => {
              if (!v) return;
              // Spec: the locale store updates IMMEDIATELY so the whole UI
              // relabels before the dialog is saved.
              form.setFieldValue('language', v);
              setLocale(toLocale(v));
            }}
          />

          {updaterEnabled && (
            <>
              <Divider />

              <Text size="sm" fw={500} c="dimmed">
                {tt('settings.section.updates')}
              </Text>

              <Group justify="flex-start">
                <Button variant="default" onClick={() => void checkForUpdatesManual()}>
                  {tt('settings.check_updates')}
                </Button>
              </Group>
            </>
          )}

          <Text size="xs" c="dimmed" ta="right" mt="md">
            {tt('settings.version', [appVersion || '—'])}
          </Text>

          <Group justify="flex-end" mt="md">
            <Button
              variant="subtle"
              onClick={() => {
                // Reset restores factory defaults. Mantine's reset() restores
                // the last values snapshot — which the dialog-open effect
                // overwrites with the loaded config via resetDirty(loaded) —
                // so set the defaults explicitly afterwards. The language
                // selector relabels the whole UI on change; form state
                // updates are async, so sync the locale store from the
                // defaults constant, not from getValues().
                form.reset();
                form.setValues(SETTINGS_DEFAULTS);
                setLocale(toLocale(SETTINGS_DEFAULTS.language));
              }}
            >
              {tt('settings.reset')}
            </Button>
            <Button type="submit">{tt('common.save')}</Button>
          </Group>
        </Stack>
      </form>

      <CleanupCacheModal
        opened={cleanupOpen}
        defaultTargetMb={form.values.max_cache_size_mb}
        onClose={() => setCleanupOpen(false)}
      />

      <DictionaryModal
        opened={dictionaryOpen}
        onClose={() => {
          setDictionaryOpen(false);
          onDictionaryChanged?.();
          commands
            .getUserDictionary()
            .then((entries) => setDictionaryCount(entries.length))
            .catch(() => setDictionaryCount(null));
        }}
      />
    </Modal>
  );
}
