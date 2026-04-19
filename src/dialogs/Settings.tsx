import { useEffect } from 'react';
import {
  Modal,
  Stack,
  Select,
  Switch,
  NumberInput,
  Button,
  Group,
  Text,
  Divider,
} from '@mantine/core';
import { useForm } from '@mantine/form';
import { notifications } from '@mantine/notifications';
import { commands } from '../lib/tauri';
import type { UIConfigPatch } from '../lib/tauri';

interface SettingsFormValues {
  speaker: string;
  sample_rate: number;
  notify_on_ready: boolean;
  notify_on_error: boolean;
  max_cache_size_mb: number;
  auto_cleanup_days: number;
  theme: string;
}

interface SettingsModalProps {
  opened: boolean;
  onClose: () => void;
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

export function SettingsModal({ opened, onClose }: SettingsModalProps) {
  const form = useForm<SettingsFormValues>({
    initialValues: {
      speaker: 'xenia',
      sample_rate: 48000,
      notify_on_ready: true,
      notify_on_error: true,
      max_cache_size_mb: 500,
      auto_cleanup_days: 30,
      theme: 'auto',
    },
    validate: {
      max_cache_size_mb: (v) => (v < 100 ? 'Минимум 100 МБ' : null),
      auto_cleanup_days: (v) => (v < 0 ? 'Значение не может быть отрицательным' : null),
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
        max_cache_size_mb: config.max_cache_size_mb,
        auto_cleanup_days: config.auto_cleanup_days,
        theme: config.theme,
      });
    });
    // form is excluded intentionally: setValues is stable, re-running on form change would loop
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [opened]);

  const handleSubmit = async (values: SettingsFormValues) => {
    const patch: UIConfigPatch = {
      speaker: values.speaker,
      sample_rate: values.sample_rate,
      notify_on_ready: values.notify_on_ready,
      notify_on_error: values.notify_on_error,
      max_cache_size_mb: values.max_cache_size_mb,
      auto_cleanup_days: values.auto_cleanup_days,
      theme: values.theme as UIConfigPatch['theme'],
    };

    try {
      await commands.updateConfig(patch);
      notifications.show({
        title: 'Настройки сохранены',
        message: 'Изменения применены.',
        color: 'green',
      });
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
            Кэш
          </Text>

          <NumberInput
            label="Максимальный размер кэша (МБ)"
            min={100}
            key={form.key('max_cache_size_mb')}
            {...form.getInputProps('max_cache_size_mb')}
          />

          <NumberInput
            label="Автоудаление через (дней)"
            description="0 — отключить автоудаление"
            min={0}
            key={form.key('auto_cleanup_days')}
            {...form.getInputProps('auto_cleanup_days')}
          />

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
    </Modal>
  );
}
