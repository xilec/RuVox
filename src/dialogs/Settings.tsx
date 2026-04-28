import { useEffect, useState } from 'react';
import {
  Alert,
  Button,
  Checkbox,
  Divider,
  Group,
  Modal,
  NumberInput,
  Select,
  Stack,
  Switch,
  Text,
  useMantineColorScheme,
} from '@mantine/core';
import type { MantineColorScheme } from '@mantine/core';
import { useForm } from '@mantine/form';
import { notifications } from '@mantine/notifications';
import { revealItemInDir } from '@tauri-apps/plugin-opener';
import { commands } from '../lib/tauri';
import type { CleanupMode, UIConfigPatch } from '../lib/tauri';
import { formatError } from '../lib/errors';

interface SettingsFormValues {
  speaker: string;
  sample_rate: number;
  notify_on_ready: boolean;
  notify_on_error: boolean;
  preview_dialog_enabled: boolean;
  max_cache_size_mb: number;
  theme: string;
}

interface SettingsModalProps {
  opened: boolean;
  onClose: () => void;
  /** Called after the user saves successfully, so the caller can refresh its
   * local copy of UIConfig without re-invoking getConfig on every render. */
  onSaved?: () => void;
}

const SPEAKER_OPTIONS = [
  { value: 'aidar', label: 'Aidar' },
  { value: 'baya', label: 'Baya' },
  { value: 'kseniya', label: 'Kseniya' },
  { value: 'xenia', label: 'Xenia' },
  { value: 'eugene', label: 'Eugene' },
  { value: 'random', label: 'Случайный' },
];

const SAMPLE_RATE_OPTIONS = [
  { value: '8000', label: '8000 Гц' },
  { value: '24000', label: '24000 Гц' },
  { value: '48000', label: '48000 Гц' },
];

const THEME_OPTIONS = [
  { value: 'light', label: 'Светлая' },
  { value: 'dark', label: 'Тёмная' },
  { value: 'auto', label: 'Авто' },
];

function formatMb(bytes: number): string {
  return `${(bytes / (1024 * 1024)).toFixed(1)} МБ`;
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
        parts.push(`удалено записей: ${result.deleted_entries}`);
      }
      parts.push(`файлов: ${result.deleted_files}`);
      parts.push(`освобождено ${formatMb(result.freed_bytes)}`);
      notifications.show({
        title: 'Кэш очищен',
        message: parts.join(', '),
        color: 'green',
      });
      onCleared?.();
      onClose();
    } catch (err) {
      notifications.show({
        title: 'Ошибка очистки кэша',
        message: formatError(err),
        color: 'red',
      });
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <Modal opened={opened} onClose={onClose} title="Очистить кэш" size="md" centered>
      <Stack gap="sm">
        {stats && (
          <Text size="sm" c="dimmed">
            Сейчас в кэше: {formatMb(stats.total_bytes)} ({stats.audio_file_count}{' '}
            файлов)
          </Text>
        )}

        <NumberInput
          label="Очистить до размера, МБ"
          description="Удаляются самые старые записи, пока кэш не уложится в указанный лимит."
          min={0}
          value={targetMb}
          onChange={(v) =>
            setTargetMb(typeof v === 'number' ? v : parseInt(String(v || 0), 10) || 0)
          }
          disabled={cleanFully}
        />

        <Checkbox
          label="Удалять тексты"
          description="Помимо аудио, удалять и сами записи из истории."
          checked={deleteTexts}
          onChange={(e) => setDeleteTexts(e.currentTarget.checked)}
        />

        <Checkbox
          label="Очистить полностью"
          description="Удалить всё аудио (и тексты, если включён флаг выше)."
          checked={cleanFully}
          onChange={(e) => setCleanFully(e.currentTarget.checked)}
        />

        {dangerous && (
          <Alert color="red" variant="light">
            Будут удалены все записи и всё аудио. Действие необратимо.
          </Alert>
        )}

        <Group justify="flex-end" mt="sm">
          <Button variant="subtle" onClick={onClose} disabled={submitting}>
            Отмена
          </Button>
          <Button color={dangerous ? 'red' : 'blue'} loading={submitting} onClick={handleConfirm}>
            {cleanFully ? 'Очистить' : 'Очистить кэш'}
          </Button>
        </Group>
      </Stack>
    </Modal>
  );
}

export function SettingsModal({ opened, onClose, onSaved }: SettingsModalProps) {
  const { setColorScheme } = useMantineColorScheme();
  const [cleanupOpen, setCleanupOpen] = useState(false);
  const [cacheDir, setCacheDir] = useState<string>('');
  const form = useForm<SettingsFormValues>({
    initialValues: {
      speaker: 'xenia',
      sample_rate: 48000,
      notify_on_ready: true,
      notify_on_error: true,
      preview_dialog_enabled: true,
      max_cache_size_mb: 500,
      theme: 'auto',
    },
    validate: {
      max_cache_size_mb: (v) => (v < 100 ? 'Минимум 100 МБ' : null),
    },
  });

  useEffect(() => {
    if (!opened) return;
    commands.getConfig().then((config) => {
      form.setValues({
        speaker: config.speaker,
        sample_rate: config.sample_rate,
        notify_on_ready: config.notify_on_ready,
        notify_on_error: config.notify_on_error,
        preview_dialog_enabled: config.preview_dialog_enabled,
        max_cache_size_mb: config.max_cache_size_mb,
        theme: config.theme,
      });
    });
    commands.getCacheDir().then(setCacheDir).catch(() => setCacheDir(''));
    // form is excluded intentionally: setValues is stable, re-running on form change would loop
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [opened]);

  const handleOpenCacheDir = async () => {
    if (!cacheDir) return;
    try {
      // revealItemInDir always needs an item path, not a bare directory.
      // history.json is always present, so use it as the marker.
      await revealItemInDir(`${cacheDir}/history.json`);
    } catch (err) {
      notifications.show({
        title: 'Не удалось открыть папку',
        message: formatError(err),
        color: 'red',
      });
    }
  };

  const handleSubmit = async (values: SettingsFormValues) => {
    const patch: UIConfigPatch = {
      speaker: values.speaker,
      sample_rate: values.sample_rate,
      notify_on_ready: values.notify_on_ready,
      notify_on_error: values.notify_on_error,
      preview_dialog_enabled: values.preview_dialog_enabled,
      max_cache_size_mb: values.max_cache_size_mb,
      theme: values.theme as UIConfigPatch['theme'],
    };

    try {
      await commands.updateConfig(patch);
      // Mantine doesn't observe backend config; push the new theme into the
      // color-scheme manager directly so the UI reflects the change without
      // waiting for a reload.
      setColorScheme(values.theme as MantineColorScheme);
      notifications.show({
        title: 'Настройки сохранены',
        message: 'Изменения применены.',
        color: 'green',
      });
      onSaved?.();
      onClose();
    } catch {
      notifications.show({
        title: 'Ошибка сохранения',
        message: 'Не удалось сохранить настройки.',
        color: 'red',
      });
    }
  };

  return (
    <Modal opened={opened} onClose={onClose} title="Настройки" size="md">
      <form onSubmit={form.onSubmit(handleSubmit)}>
        <Stack gap="sm">
          <Text size="sm" fw={500} c="dimmed">
            Синтез речи
          </Text>

          <Select
            label="Голос"
            data={SPEAKER_OPTIONS}
            key={form.key('speaker')}
            {...form.getInputProps('speaker')}
          />

          <Select
            label="Частота дискретизации"
            data={SAMPLE_RATE_OPTIONS}
            value={String(form.values.sample_rate)}
            onChange={(v) =>
              form.setFieldValue('sample_rate', v ? parseInt(v, 10) : 48000)
            }
            error={form.errors.sample_rate}
          />

          <Divider />

          <Text size="sm" fw={500} c="dimmed">
            Уведомления
          </Text>

          <Switch
            label="Уведомлять о готовности аудио"
            key={form.key('notify_on_ready')}
            {...form.getInputProps('notify_on_ready', { type: 'checkbox' })}
          />

          <Switch
            label="Уведомлять об ошибках синтеза"
            key={form.key('notify_on_error')}
            {...form.getInputProps('notify_on_error', { type: 'checkbox' })}
          />

          <Divider />

          <Text size="sm" fw={500} c="dimmed">
            Предпросмотр
          </Text>

          <Switch
            label="Показывать диалог предпросмотра перед синтезом"
            key={form.key('preview_dialog_enabled')}
            {...form.getInputProps('preview_dialog_enabled', {
              type: 'checkbox',
            })}
          />

          <Divider />

          <Text size="sm" fw={500} c="dimmed">
            Кэш
          </Text>

          <NumberInput
            label="Максимальный размер кэша (МБ)"
            description="При запуске и при ручной очистке самые старые записи удаляются, пока кэш не уложится в этот лимит."
            min={100}
            key={form.key('max_cache_size_mb')}
            {...form.getInputProps('max_cache_size_mb')}
          />

          {cacheDir && (
            <Stack gap={4}>
              <Text size="xs" c="dimmed">
                Папка кэша
              </Text>
              <Text size="sm" style={{ wordBreak: 'break-all', fontFamily: 'var(--mantine-font-family-monospace)' }}>
                {cacheDir}
              </Text>
            </Stack>
          )}

          <Group justify="flex-start">
            <Button variant="default" onClick={handleOpenCacheDir} disabled={!cacheDir}>
              Открыть папку
            </Button>
            <Button variant="default" onClick={() => setCleanupOpen(true)}>
              Очистить кэш…
            </Button>
          </Group>

          <Divider />

          <Text size="sm" fw={500} c="dimmed">
            Интерфейс
          </Text>

          <Select
            label="Тема оформления"
            data={THEME_OPTIONS}
            key={form.key('theme')}
            {...form.getInputProps('theme')}
          />

          <Group justify="flex-end" mt="md">
            <Button variant="subtle" onClick={() => form.reset()}>
              Сбросить
            </Button>
            <Button type="submit">Сохранить</Button>
          </Group>
        </Stack>
      </form>

      <CleanupCacheModal
        opened={cleanupOpen}
        defaultTargetMb={form.values.max_cache_size_mb}
        onClose={() => setCleanupOpen(false)}
      />
    </Modal>
  );
}
